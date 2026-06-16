//! canonical_json: the deterministic encoder used for hashing and signing,
//! exposed so callers can hash their own payloads the same way the chain does.

use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Encode a JSON-able Python value to canonical-JSON bytes: sorted object keys,
/// no whitespace, fixed escapes, non-ASCII as raw UTF-8.
#[pyfunction]
fn canonical_json<'py>(value: Bound<'py, PyAny>, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
    let json: serde_json::Value = crate::convert::depythonize_finite(&value)?;
    let bytes = provedex_core::canonical_json(&json);
    Ok(PyBytes::new(py, &bytes))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(canonical_json, m)?)?;
    Ok(())
}
