//! SignedEvent read-only view, the low-level sign_event escape hatch, and
//! compute_self_hash for verifier/compat use.

use pyo3::prelude::*;

use provedex_core::{compute_self_hash as core_self_hash, SignedEvent as CoreSigned};

use crate::errors::signed_err;
use crate::events::AgentEvent;
use crate::keypair::SigningKeypair;

/// Read-only view of a sealed, signed event. Construct via Session.record or
/// sign_event; the fields are not settable from Python.
#[pyclass(frozen)]
pub struct SignedEvent {
    pub(crate) inner: CoreSigned,
}

impl SignedEvent {
    pub(crate) fn wrap(inner: CoreSigned) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl SignedEvent {
    #[getter]
    fn seq(&self) -> u64 {
        self.inner.seq
    }

    #[getter]
    fn timestamp_nanos(&self) -> u64 {
        self.inner.timestamp_nanos
    }

    /// The event as its tagged `{"type", "payload"}` mapping.
    #[getter]
    fn event<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let value = serde_json::to_value(&self.inner.event)
            .map_err(|e| signed_err(provedex_core::SignedError::Json(e)))?;
        pythonize::pythonize(py, &value)
            .map_err(|e| crate::errors::SigningError::new_err(e.to_string()))
    }

    #[getter]
    fn parent_hash(&self) -> String {
        self.inner.parent_hash.clone()
    }

    #[getter]
    fn self_hash(&self) -> String {
        self.inner.self_hash.clone()
    }

    #[getter]
    fn signature(&self) -> String {
        self.inner.signature.clone()
    }

    #[getter]
    fn signer_pubkey(&self) -> String {
        self.inner.signer_pubkey.clone()
    }

    /// The exact NDJSON ledger line bytes for this event, as a `str`. Byte-for
    /// byte identical to the line the Rust ledger would write.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| signed_err(provedex_core::SignedError::Json(e)))
    }

    fn __repr__(&self) -> String {
        format!(
            "SignedEvent(seq={}, self_hash='{}')",
            self.inner.seq, self.inner.self_hash
        )
    }
}

/// Low-level escape hatch: seal one event against a caller-managed parent hash
/// and seq. Most callers use Session, which manages seq and parent chaining.
#[pyfunction]
#[pyo3(signature = (*, event, seq, parent_hash, keypair))]
fn sign_event(
    event: &AgentEvent,
    seq: u64,
    parent_hash: &str,
    keypair: &SigningKeypair,
) -> PyResult<SignedEvent> {
    let signed = CoreSigned::seal(seq, event.inner.clone(), parent_hash, &keypair.inner)
        .map_err(signed_err)?;
    Ok(SignedEvent::wrap(signed))
}

/// Compute the self_hash hex for `(seq, timestamp_nanos, event, parent_hash)`
/// without signing. Exposed for verifier implementations and byte-compat tests.
#[pyfunction]
#[pyo3(signature = (*, seq, timestamp_nanos, event, parent_hash))]
fn compute_self_hash(
    seq: u64,
    timestamp_nanos: u64,
    event: &AgentEvent,
    parent_hash: &str,
) -> PyResult<String> {
    let bytes =
        core_self_hash(seq, timestamp_nanos, &event.inner, parent_hash).map_err(signed_err)?;
    Ok(hex::encode(bytes))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SignedEvent>()?;
    m.add_function(wrap_pyfunction!(sign_event, m)?)?;
    m.add_function(wrap_pyfunction!(compute_self_hash, m)?)?;
    m.add("GENESIS_PARENT_HASH", provedex_core::GENESIS_PARENT_HASH)?;
    Ok(())
}
