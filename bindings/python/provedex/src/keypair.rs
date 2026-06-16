//! SigningKeypair: Ed25519 key generation, persistence, and the public key.

use std::path::PathBuf;

use pyo3::prelude::*;

use crate::errors::key_err;

#[pyclass]
pub struct SigningKeypair {
    pub(crate) inner: provedex_core::SigningKeypair,
}

#[pymethods]
impl SigningKeypair {
    /// Generate a fresh Ed25519 keypair from the OS RNG.
    #[staticmethod]
    fn generate() -> Self {
        Self {
            inner: provedex_core::SigningKeypair::generate(),
        }
    }

    /// Load a keypair from a 32-byte secret-key file.
    #[staticmethod]
    fn load(path: PathBuf) -> PyResult<Self> {
        Ok(Self {
            inner: provedex_core::SigningKeypair::load(path).map_err(key_err)?,
        })
    }

    /// Load the keypair at `path`, or generate and persist one if absent.
    #[staticmethod]
    fn load_or_create(path: PathBuf) -> PyResult<Self> {
        Ok(Self {
            inner: provedex_core::SigningKeypair::load_or_create(path).map_err(key_err)?,
        })
    }

    /// Persist the 32-byte secret key to `path` (0600 on unix).
    fn save(&self, path: PathBuf) -> PyResult<()> {
        self.inner.save(path).map_err(key_err)
    }

    /// The 64-hex-character Ed25519 public key.
    #[getter]
    fn pubkey_hex(&self) -> String {
        self.inner.pubkey_hex()
    }

    fn __repr__(&self) -> String {
        format!("SigningKeypair(pubkey_hex='{}')", self.inner.pubkey_hex())
    }
}
