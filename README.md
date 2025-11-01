The following guide was written by kweinmeister on medium
This is his repo https://github.com/kweinmeister/fast-diff-mcp/tree/main

# Python and Rust interoperability: A walkthrough for building a high performance MCP server

Oct 20, 2025

For years, we’ve been told to pick a side. Are you on Team Python, where you can build complex apps at lightning speed? Or are you on Team Rust, where you get elite performance and memory safety with a language that’s been most admired for 9 years straight?

Now having the best of both worlds is possible, thanks to libraries like PyO3. PyO3 provides a set of Rust macros that handle the low-level details of translating between Python and Rust data types.

In this guide, you’ll learn step-by-step instructions for including Rust code with your Python code. We’ll build a tool for AI agents that conforms to the Model Context Protocol. We’ll write the compute-intensive part in Rust and wrap it in a friendly Python interface.

Our use case will be a string diff tool. Diffing text is a perfect example of a CPU-heavy task that would be appropriate for Rust. We’ll then put our creation to the test against Python’s built-in difflib to measure the performance improvement.

All of the code in this guide is available in the fast-diff-mcp GitHub repository. In fact, you can directly run the MCP server in the Gemini CLI or your favorite development tool by following the steps in the README.

## Project Setup and Workflow
Before you begin, make sure you have the Rust toolchain, Python 3.10 or newer, and uv, the Rust-based Python package manager.

### Initialize the Python-Rust Project
To bridge the gap between our Rust code and the Python world, we need a specialized build tool. Maturin compiles our Rust code and packages it as a standard Python wheel that can be imported.

Let’s create a new project called fast-diff-mcp using the PyO3 bindings, and change to that directory:

```shell
uvx maturin new -b pyo3 fast-diff-mcp
cd fast-diff-mcp
```

As fastmcp needs at least Python 3.10, update the minimum version of Python in your project if needed in the pyproject.toml file:

```
[project]
name = "fast-diff-mcp"
requires-python = ">=3.10"
```

### Set Up the Virtual Environment
Next, let’s create a Python virtual environment with uv. This keeps our project’s dependencies from clashing with other projects.

```
uv venv
source .venv/bin/activate
```

### Add Dependencies
Our project needs dependencies in both languages.

For Python, we’ll use uv add. We need fastmcp for our MCP server. For our development setup, we’ll add pyo3-stubgen to generate typing stubs, faker and pyperf for benchmarking, and maturin to document its use in the project. We’ll also add our own project in “editable” mode, so changes are picked up instantly.
```
# Add the application framework
uv add fastmcp

# Add development tools to the 'dev' extra group
uv add --optional dev pyo3-stubgen faker pyperf maturin
uv add --optional dev --editable .

```
For Rust, we’ll use cargo add. We need similar for the high-performance diffing and pyo3 for the Python bindings.

```
cargo add similar
cargo add pyo3 --features "extension-module"
```

Finally, install Python dependencies with one command:
```
uv pip install ".[dev]"
```

## Write the Rust Logic
It’s time to write the application logic in src/lib.rs. The Rust docstring (///) automatically becomes the Python tool’s docstring. The [#[pyfunction] and #[pymodule] macros from pyo3 do all the heavy lifting of exposing our Rust function to Python. You can update the template code in src/lib.rs with this code:

```
// src/lib.rs
use pyo3::prelude::*;
use similar::TextDiff;

/// Compares two multiline strings and returns the difference in the
/// standard unified diff format. This is a high-performance implementation
/// written in Rust.
#[pyfunction]
fn unified_diff(text1: String, text2: String) -> PyResult<String> {
    let diff = TextDiff::from_lines(&text1, &text2);
    let diff_output = diff.unified_diff().header("original", "modified").to_string();
    Ok(diff_output)
}

#[pymodule]
fn fast_diff_mcp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(unified_diff, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let result = unified_diff("a\nb\n".to_string(), "a\nc\n".to_string()).unwrap();
        assert_eq!(result, "--- original\n+++ modified\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n");
    }
}
```
Before going any further, build the module with maturin develop and run the test with cargo test:
```
maturin develop
cargo test
```
`cargo test` to verify your code is working. If you are see linker errors in your environment, you can set the -L flag for where to find your Python library directory, and -l for the Python version. For more information, see the cargo documentation.

```
RUSTFLAGS="-L $(python -c "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))") -lpython$(python -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")" cargo test
```

### Generate Python Stubs
A big part of Python’s appeal is the great editor support. We don’t want to lose that. By generating a .pyi stub file, we can give our editor a map of our compiled Rust code, enabling autocompletion and type checking:

`pyo3-stubgen fast_diff_mcp .`

### The Development Process
We’ve now walked through the main steps. Let’s recap how you can iterate as your code evolves. You can edit your Rust code in src/lib.rs, verify your logic with cargo test, and then run maturin develop to perform an incremental rebuild that installs the module in editable mode. If you change any function signatures, re-run pyo3-stubgen fast_diff_mcp . to keep your editor’s autocompletion in sync.

### Integrate with Python
With the Rust binaries compiled, we can import our new module into the Python app. Our server.py will define two tools: one using our new Rust function, and one using Python’s standard difflib so we can compare them.

```
import asyncio
import logging
import os
from typing import List, Iterator

from fast_diff_mcp import unified_diff
from fastmcp import FastMCP
import difflib

logger: logging.Logger = logging.getLogger(__name__)
mcp: FastMCP = FastMCP("fast-diff-mcp")


@mcp.tool()
def diff_tool_rust_similar(original_text: str, modified_text: str) -> str:
    """
    Compares two multiline strings and returns the difference in the
    standard unified diff format. This is a high-performance implementation
    written in Rust, using the `similar` crate's Myers diff algorithm.
    """
    logger.info(">>> Tool: 'diff_tool_rust_similar' called")
    return unified_diff(original_text, modified_text)


@mcp.tool()
def diff_tool_python_difflib(original_text: str, modified_text: str) -> str:
    """
    Compares two multiline strings using the standard Python `difflib`
    library, which uses the Ratcliff/Obershelp algorithm.
    """
    logger.info(">>> Tool: 'diff_tool_python_difflib' called")
    original_lines: List[str] = original_text.splitlines(keepends=True)
    modified_lines: List[str] = modified_text.splitlines(keepends=True)
    diff: Iterator[str] = difflib.unified_diff(
        original_lines, modified_lines, "original", "modified"
    )
    return "".join(diff)


if __name__ == "__main__":
    logging.basicConfig(format="[%(levelname)s]: %(message)s", level=logging.INFO)
    logger.info(f" MCP server started on port {os.getenv('PORT', 8080)}")
    asyncio.run(
        mcp.run_async(
            transport="streamable-http",
            host="0.0.0.0",
            port=int(os.getenv("PORT", 8080)),
        )
    )
```

You can now install your project, including the Rust dependencies specified in pyproject.toml:
```
uv pip install -e .
uv run server.py
```
The fastmcp command-line interface includes a dev command that launches your server along with the MCP Inspector, a graphical user interface for testing your server’s tools.

To try it, run the following command. This will start your server and open the Inspector in a new browser window.

`uv run fastmcp dev server.py`
The Inspector should automatically connect to your server. If not, select the transport and then click the Connect button. Next, click on the Tools tab at the top. Click the List Tools button. You should see diff_tool_rust_similar appear in the list on the left.

Click on diff_tool_rust_similar. A form will appear on the right with fields for original_text and modified_text. Enter your text into the input fields. For example:

original_text:

line1
line2_old
line3
modified_text:

line1
line2_new
line3
Click the Run Tool button. The output of the tool will appear in the “Result” section below the form, showing the unified diff.

Press enter or click to view image in full size

## Benchmark: Rust vs. Python
So, how much faster is it? We can use the pyperf library to find out. Our benchmark.py script compares our Rust-based unified_diff to Python’s difflib. My benchmark running locally on my MacBook Pro showed that the Rust implementation was over twice as fast.

Press enter or click to view image in full size

There are some important details to keep in mind for this benchmark. First, this included the entire MCP call_tool function scope, which includes initialization code and protocol overhead. Benchmarking just the diff algorithm function would result in a much larger difference.

I used large strings of 10 million chars, to emulate a log processing use case. Smaller strings would shrink the gap, and larger will increase it due to the fixed protocol overhead.

Finally, Rust’s diff implementation uses the Myers algorithm, whereas the Python algorithm uses Ratcliff/Obershelp. In initial benchmarking, I also tried the myers Python library. It performed better than the built-in Python difflib implementation for smaller strings, but did not scale well with larger strings.

The Python diff implementation and benchmark code is in the GitHub repository. Try the benchmark for yourself!
```
uv run python benchmark.py
```
## Conclusion
You’re no longer forced to choose between Python’s speed of development and Rust’s speed of execution. You can have both. Think of Rust not as a replacement for Python, but as a powerful ally. Find your bottlenecks, offload them to Rust, and improve your applications.

To learn more about using Rust in the Cloud, check out the official Rust SDK for Google Cloud. For an example of building a pure Rust MCP server, see Building Scalable AI Agents.
