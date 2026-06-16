//! Session: the ergonomic primary path. Wraps core LedgerSession, which owns
//! the seq counter, parent-hash chaining, and the fsync-on-append ledger.

use std::path::PathBuf;

use pyo3::prelude::*;

use provedex_core::{Ledger, LedgerSession};

use crate::errors::{ledger_err, session_err};
use crate::events::AgentEvent;
use crate::keypair::SigningKeypair;
use crate::signed::SignedEvent;

/// One signing session bound to a single ledger file. `record` allocates the
/// next seq, signs against the running parent hash, appends with fsync, and
/// advances the chain. Resumes from any pre-existing events on open.
#[pyclass]
pub struct Session {
    inner: LedgerSession,
}

#[pymethods]
impl Session {
    /// Open (or resume) a session writing to `ledger_path`.
    #[staticmethod]
    #[pyo3(signature = (*, keypair, ledger_path, session_id))]
    fn open(keypair: &SigningKeypair, ledger_path: PathBuf, session_id: String) -> PyResult<Self> {
        let ledger = Ledger::open(ledger_path).map_err(ledger_err)?;
        let inner = LedgerSession::open(keypair.inner.clone(), ledger, session_id)
            .map_err(session_err)?;
        Ok(Self { inner })
    }

    /// Seal `event`, append it to the ledger, and return the SignedEvent.
    ///
    /// This call fsyncs the ledger before returning (durability), which costs a
    /// few milliseconds. On an async backend, wrap it in `asyncio.to_thread`.
    fn record(&self, event: &AgentEvent, py: Python<'_>) -> PyResult<SignedEvent> {
        // Release the GIL across the seal + fsync so other Python threads run.
        let signed = py
            .allow_threads(|| self.inner.seal_and_append(event.inner.clone()))
            .map_err(session_err)?;
        Ok(SignedEvent::wrap(signed))
    }

    #[getter]
    fn session_id(&self) -> String {
        self.inner.session_id().to_string()
    }

    #[getter]
    fn pubkey_hex(&self) -> String {
        self.inner.pubkey_hex()
    }

    fn __repr__(&self) -> String {
        format!("Session(session_id='{}')", self.inner.session_id())
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Session>()?;
    Ok(())
}
