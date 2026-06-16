//! Shared Python-to-serde_json conversion with a non-finite-float guard.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyList, PyTuple};
use pythonize::depythonize;

use crate::errors::SigningError;

/// Depythonize a Python value to serde_json::Value, rejecting non-finite
/// floats. serde_json coerces NaN/Infinity to null, which would silently alter
/// a signed payload; refuse so the signature always covers the real input.
pub(crate) fn depythonize_finite(value: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    reject_non_finite(value)?;
    depythonize(value).map_err(|e| SigningError::new_err(e.to_string()))
}

fn reject_non_finite(value: &Bound<'_, PyAny>) -> PyResult<()> {
    if let Ok(f) = value.cast::<PyFloat>() {
        if !f.value().is_finite() {
            return Err(SigningError::new_err(
                "non-finite float (NaN or Infinity) cannot be signed",
            ));
        }
    } else if let Ok(d) = value.cast::<PyDict>() {
        for (_, v) in d.iter() {
            reject_non_finite(&v)?;
        }
    } else if let Ok(l) = value.cast::<PyList>() {
        for v in l.iter() {
            reject_non_finite(&v)?;
        }
    } else if let Ok(t) = value.cast::<PyTuple>() {
        for v in t.iter() {
            reject_non_finite(&v)?;
        }
    }
    Ok(())
}
