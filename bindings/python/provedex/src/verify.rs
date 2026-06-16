//! Chain verification surface. A broken-but-parseable chain is data, returned
//! as ChainReport(ok=False); only unparseable input raises ChainError.

use std::path::PathBuf;

use pyo3::prelude::*;

use provedex_core::{
    read_file, verify_chain as core_verify, ChainReport as CoreReport, ChainStatus,
};

use crate::errors::ledger_err;
use crate::signed::SignedEvent;

/// Result of walking a chain: hashes, signatures, parent links, seq density.
#[pyclass(frozen)]
pub struct ChainReport {
    inner: CoreReport,
}

#[pymethods]
impl ChainReport {
    /// True if every event passed hash, signature, parent-link, and seq checks.
    #[getter]
    fn ok(&self) -> bool {
        matches!(self.inner.status, ChainStatus::Valid)
    }

    /// "valid" or "broken".
    #[getter]
    fn status(&self) -> String {
        match self.inner.status {
            ChainStatus::Valid => "valid".to_string(),
            ChainStatus::Broken => "broken".to_string(),
        }
    }

    #[getter]
    fn event_count(&self) -> u64 {
        self.inner.event_count
    }

    /// Seq of the first failing event, or None if the chain is valid.
    #[getter]
    fn broken_at(&self) -> Option<u64> {
        self.inner.broken_at_seq
    }

    #[getter]
    fn reason(&self) -> Option<String> {
        self.inner.broken_reason.clone()
    }

    /// self_hash of the last valid event walked (genesis sentinel if empty).
    #[getter]
    fn root_hash(&self) -> String {
        self.inner.root_hash.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "ChainReport(ok={}, event_count={}, broken_at={:?})",
            matches!(self.inner.status, ChainStatus::Valid),
            self.inner.event_count,
            self.inner.broken_at_seq,
        )
    }
}

/// Verify an in-memory list of SignedEvent objects.
#[pyfunction]
fn verify_chain(events: Vec<Py<SignedEvent>>, py: Python<'_>) -> ChainReport {
    let owned: Vec<provedex_core::SignedEvent> =
        events.iter().map(|e| e.borrow(py).inner.clone()).collect();
    ChainReport {
        inner: core_verify(&owned),
    }
}

/// Read an NDJSON ledger file and verify it. A missing file verifies as an
/// empty, valid chain (event_count = 0).
#[pyfunction]
fn verify_file(path: PathBuf) -> PyResult<ChainReport> {
    let events = read_file(path).map_err(ledger_err)?;
    Ok(ChainReport {
        inner: core_verify(&events),
    })
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ChainReport>()?;
    m.add_function(wrap_pyfunction!(verify_chain, m)?)?;
    m.add_function(wrap_pyfunction!(verify_file, m)?)?;
    Ok(())
}
