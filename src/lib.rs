use pyo3::prelude::*;
use similar::TextDiff;

/// Compares two multiline strings and returns the difference in the
/// standard unified diff format. This is a high-performance implementation
/// written in Rust.
#[pyfunction]
fn unified_diff(old: String, new: String) -> PyResult<String> {
    let diff = TextDiff::from_lines(&old, &new);
    let diff_output = diff
        .unified_diff()
        .header("original", "modified")
        .to_string();
    Ok(diff_output)
}

#[pymodule]
fn fast_diff_mcp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(unified_diff, m)?)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn it_works() {
        let result = unified_diff("a\nb\n".to_string(), "a\nc\n".to_string()).unwrap();
        assert_eq!(
            result,
            "--- original\n+++ modified\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n"
        );
    }
}
