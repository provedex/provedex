//! Native Python bindings for provedex-core. Built with PyO3 + maturin and
//! published to PyPI as `provedex`. Thin pass-through: every cryptographic
//! operation is delegated to provedex-core, so the canonical-JSON encoder and
//! signature scheme are identical to the Rust reference.

use pyo3::prelude::*;

mod errors;
mod events;
mod keypair;
mod session;
mod signed;
mod verify;

#[pymodule]
fn _provedex(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    errors::register(m)?;
    m.add_class::<keypair::SigningKeypair>()?;
    events::build(m)?;
    signed::register(m)?;
    session::register(m)?;
    verify::register(m)?;
    Ok(())
}
