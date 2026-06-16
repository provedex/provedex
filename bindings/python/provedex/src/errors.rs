//! Python exception hierarchy and core-error mapping. All binding failures
//! raise; none return error sentinels.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

create_exception!(
    provedex,
    ProvedexError,
    PyException,
    "Base class for all Provedex errors."
);
create_exception!(
    provedex,
    KeyLoadError,
    ProvedexError,
    "Keypair load or save failure."
);
create_exception!(
    provedex,
    SigningError,
    ProvedexError,
    "Event seal or hash failure."
);
create_exception!(
    provedex,
    LedgerError,
    ProvedexError,
    "Ledger read or write failure."
);
create_exception!(
    provedex,
    ChainError,
    ProvedexError,
    "Malformed verification input."
);

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("ProvedexError", py.get_type::<ProvedexError>())?;
    m.add("KeyLoadError", py.get_type::<KeyLoadError>())?;
    m.add("SigningError", py.get_type::<SigningError>())?;
    m.add("LedgerError", py.get_type::<LedgerError>())?;
    m.add("ChainError", py.get_type::<ChainError>())?;
    Ok(())
}

pub(crate) fn key_err(e: provedex_core::KeyError) -> PyErr {
    KeyLoadError::new_err(e.to_string())
}

pub(crate) fn signed_err(e: provedex_core::SignedError) -> PyErr {
    SigningError::new_err(e.to_string())
}

pub(crate) fn ledger_err(e: provedex_core::LedgerError) -> PyErr {
    LedgerError::new_err(e.to_string())
}

pub(crate) fn session_err(e: provedex_core::SessionError) -> PyErr {
    // SessionError wraps either a ledger or a signed error; map each to the
    // matching Python class so a disk failure during record() surfaces as
    // LedgerError, not SigningError.
    match e {
        provedex_core::SessionError::Ledger(le) => ledger_err(le),
        provedex_core::SessionError::Signed(se) => signed_err(se),
    }
}
