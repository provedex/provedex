//! canonical_json: the deterministic encoder used for hashing and signing,
//! exposed so callers can hash their own payloads the same way the chain does.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pythonize::depythonize;

/// Encode a JSON-able Python value to canonical-JSON bytes: sorted object keys,
/// no whitespace, fixed escapes, non-ASCII as raw UTF-8.
#[pyfunction]
fn canonical_json<'py>(value: Bound<'py, PyAny>, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
    let json: serde_json::Value = depythonize(&value)
        .map_err(|e| crate::errors::SigningError::new_err(e.to_string()))?;
    let bytes = provedex_core::canonical_json(&json);
    Ok(PyBytes::new_bound(py, &bytes))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(canonical_json, m)?)?;
    Ok(())
}
