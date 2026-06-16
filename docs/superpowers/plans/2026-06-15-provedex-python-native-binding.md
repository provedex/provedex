# Native Python Binding (provedex / PyO3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `provedex`, a native Python SDK that signs Provedex events in-process via PyO3 around `provedex-core`, byte-identical to the Rust reference.

**Architecture:** A standalone Rust crate `provedex-python` at `bindings/python/provedex/`, built with maturin, published to PyPI as `provedex`. Thin pass-through: every crypto call lands in `provedex-core`, so there is exactly one canonical-JSON encoder in the system and byte-compat is structural. maturin mixed layout: a private compiled `_provedex` module plus a `python/provedex/__init__.py` that re-exports it and ships `.pyi` stubs.

**Tech Stack:** Rust 1.89, PyO3 0.22 (abi3-py311), pythonize 0.22, maturin >= 1.7, pytest, ruff, mypy. Spec: `docs/superpowers/specs/2026-06-15-provedex-python-native-binding-design.md`.

**Branch:** `feat/python-native-binding` (already created; spec already committed there).

**Tracking issue:** #6 (scaffold pyo3 wrapper). This plan does the scaffold AND the full v1 surface in one pass.

---

## PyO3 version note (read once before Task 1)

This plan targets **PyO3 0.22**, which uses the Bound API throughout:
- `#[pymodule] fn name(m: &Bound<'_, PyModule>) -> PyResult<()>`
- `PyModule::new(py, "name")` returns `Bound<'_, PyModule>`
- `Python::get_type::<T>()` returns `Bound<'_, PyType>`
- `wrap_pyfunction!(f, m)` takes `&Bound<PyModule>`
- `pythonize::depythonize(&bound_any)` and `pythonize::pythonize(py, &value)`

If `maturin develop` reports an unknown symbol, the installed PyO3 minor differs from 0.22. Do NOT fall back to the removed gil-ref APIs (`&PyModule`, `PyModule::new_bound`); instead pin `pyo3 = "=0.22"` in `Cargo.toml` and re-resolve. Minor name differences (e.g. `get_type` vs `get_type_bound`) are normal compile-fixes; read the compiler error and adjust.

---

## File structure (locked here, built across tasks)

```
bindings/python/provedex/
  Cargo.toml                      # provedex-python crate, [lib] name = "_provedex", cdylib
  pyproject.toml                  # maturin backend, module-name = "provedex._provedex"
  src/lib.rs                      # #[pymodule] fn _provedex: registers everything
  src/errors.rs                   # ProvedexError + 4 subclasses, core-error mapping
  src/keypair.rs                  # SigningKeypair pyclass
  src/events.rs                   # events submodule: 7 factories + from_dict, AgentEvent pyclass
  src/signed.rs                   # SignedEvent pyclass, sign_event, compute_self_hash
  src/session.rs                  # Session pyclass
  src/verify.rs                   # ChainReport pyclass, verify_chain, verify_file
  src/canonical.rs                # canonical_json
  python/provedex/__init__.py     # re-export from ._provedex, register provedex.events
  python/provedex/__init__.pyi    # top-level type stubs
  python/provedex/events.pyi      # events submodule stubs
  python/provedex/py.typed        # PEP 561 marker
  tests/conftest.py               # shared fixtures (built CLI path)
  tests/test_keypair.py
  tests/test_events.py
  tests/test_signed.py
  tests/test_session.py
  tests/test_verify.py
  tests/test_canonical.py
  tests/test_compat.py            # golden-vector byte-compat
  tests/test_cross_verify.py      # integration: Rust CLI <-> Python
  examples/basic.py
  README.md
  RELEASING.md

crates/provedex-core/
  src/keys.rs                     # add Clone to SigningKeypair (Task 2)
  examples/emit_compat_vectors.rs # golden-vector generator (Task 11)

tests/compat/vectors/             # generated goldens, committed (Task 11)
  canonical_json.json
  self_hash.json

.github/workflows/
  ci.yml                          # add bindings-python-native job (Task 14)
  release-python.yml              # maturin wheel build + PyPI (Task 16)
```

---

### Task 1: Scaffold the crate, maturin mixed layout, smoke import

**Files:**
- Create: `bindings/python/provedex/Cargo.toml`
- Create: `bindings/python/provedex/pyproject.toml`
- Create: `bindings/python/provedex/src/lib.rs`
- Create: `bindings/python/provedex/python/provedex/__init__.py`
- Create: `bindings/python/provedex/python/provedex/py.typed`
- Create: `bindings/python/provedex/tests/test_smoke.py`
- Create: `bindings/python/provedex/.gitignore`

- [ ] **Step 1: Write `.gitignore`**

```
target/
.venv/
*.so
__pycache__/
.pytest_cache/
.mypy_cache/
.ruff_cache/
dist/
```

- [ ] **Step 2: Write `Cargo.toml`**

```toml
[package]
name = "provedex-python"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
repository = "https://github.com/provedex/provedex"
publish = false

# Empty [workspace] makes this crate its own workspace root. Without it, cargo
# walks up, finds the repo-root workspace, and errors because this package is
# not in its `members` list. The crates rules forbid FFI inside a crates/
# member, so this binding stays decoupled with its own Cargo.lock.
[workspace]

[lib]
name = "_provedex"
crate-type = ["cdylib"]

[dependencies]
provedex-core = { path = "../../../crates/provedex-core" }
pyo3 = { version = "0.22", features = ["extension-module", "abi3-py311"] }
pythonize = "0.22"
serde_json = "1.0"

[profile.release]
strip = true
```

Note: the `[workspace]` line above is load-bearing; do not remove it. Because this crate is its own workspace, repo-root `cargo` commands with `--workspace` do not touch it (intentional), and it carries its own `Cargo.lock`.

- [ ] **Step 3: Write `pyproject.toml`**

```toml
[build-system]
requires = ["maturin>=1.7,<2.0"]
build-backend = "maturin"

[project]
name = "provedex"
version = "0.1.0"
description = "Native Python SDK for Provedex: Ed25519-signed, hash-chained agent evidence, byte-identical to the Rust reference."
readme = "README.md"
requires-python = ">=3.11"
license = { text = "Apache-2.0" }
authors = [
    { name = "Aditya Suresh", email = "adi@provedex.io" },
]
keywords = ["audit", "signing", "ed25519", "hash-chain", "compliance", "provedex", "evidence"]
classifiers = [
    "Development Status :: 4 - Beta",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: Apache Software License",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
    "Programming Language :: Rust",
    "Topic :: Security :: Cryptography",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0",
    "ruff>=0.5",
    "mypy>=1.10",
]

[project.urls]
Homepage = "https://github.com/provedex/provedex"
Repository = "https://github.com/provedex/provedex"
Issues = "https://github.com/provedex/provedex/issues"

[tool.maturin]
module-name = "provedex._provedex"
python-source = "python"
features = ["pyo3/extension-module"]

[tool.ruff]
line-length = 100
target-version = "py311"

[tool.ruff.lint]
select = ["E", "F", "I", "B", "UP"]

[tool.mypy]
python_version = "3.11"
strict = true

[tool.pytest.ini_options]
markers = [
    "integration: requires the provedex CLI binary built from source",
]
```

- [ ] **Step 4: Write the empty module `src/lib.rs`**

```rust
//! Native Python bindings for provedex-core. Built with PyO3 + maturin and
//! published to PyPI as `provedex`. Thin pass-through: every cryptographic
//! operation is delegated to provedex-core, so the canonical-JSON encoder and
//! signature scheme are identical to the Rust reference.

use pyo3::prelude::*;

#[pymodule]
fn _provedex(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
```

- [ ] **Step 5: Write `python/provedex/__init__.py`**

```python
"""Native Python SDK for Provedex.

Signs Ed25519, hash-chained agent evidence in-process, byte-identical to the
Rust reference. See https://github.com/provedex/provedex.
"""

from ._provedex import __version__

__all__ = ["__version__"]
```

- [ ] **Step 6: Write `python/provedex/py.typed`** (empty file)

- [ ] **Step 7: Write the smoke test `tests/test_smoke.py`**

```python
def test_import_and_version():
    import provedex

    assert isinstance(provedex.__version__, str)
    assert provedex.__version__ == "0.1.0"
```

- [ ] **Step 8: Create the venv, install tooling, build, run the smoke test**

```bash
cd bindings/python/provedex
python3.11 -m venv .venv
.venv/bin/pip install --upgrade pip maturin pytest
.venv/bin/maturin develop
.venv/bin/pytest tests/test_smoke.py -v
```

Expected: `maturin develop` builds `_provedex`; the test PASSES.

- [ ] **Step 9: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex/Cargo.toml bindings/python/provedex/Cargo.lock \
  bindings/python/provedex/pyproject.toml \
  bindings/python/provedex/src/lib.rs bindings/python/provedex/python \
  bindings/python/provedex/tests/test_smoke.py bindings/python/provedex/.gitignore
git commit -m "feat(bindings/python): scaffold native provedex pyo3 crate

PyO3 + maturin mixed layout (private _provedex module, python/provedex
re-export). abi3-py311 single-wheel-per-platform. Smoke test builds and
imports. Refs #6."
```

---

### Task 2: Add `Clone` to core `SigningKeypair`

The Python `Session.open` must move a keypair into `provedex_core::LedgerSession`, but a PyO3 pyclass is shared (borrowed), not owned. Clone the inner keypair into the session. This is an additive change: it touches `keys.rs` but changes no key file format, no canonical-JSON, and no chain invariant, so no ADR is required.

**Files:**
- Modify: `crates/provedex-core/src/keys.rs`

- [ ] **Step 1: Write the failing test** (append to the `tests` module in `keys.rs`)

```rust
    #[test]
    fn clone_preserves_identity() {
        let kp = SigningKeypair::generate();
        let cloned = kp.clone();
        assert_eq!(kp.pubkey_hex(), cloned.pubkey_hex());
        let msg = b"same key signs the same";
        let sig = hex::encode(cloned.sign(msg).to_bytes());
        assert!(verify_signature(&kp.pubkey_hex(), msg, &sig).is_ok());
    }
```

- [ ] **Step 2: Run it, confirm it fails to compile**

```bash
cd /Users/adi/Desktop/provedex
cargo test -p provedex-core keys::tests::clone_preserves_identity
```

Expected: compile error, `SigningKeypair` does not implement `Clone`.

- [ ] **Step 3: Implement `Clone` manually via the secret bytes**

Add below the `impl SigningKeypair` block in `keys.rs`. Manual impl (not derive) so it does not depend on whether `ed25519_dalek::SigningKey` derives `Clone`:

```rust
impl Clone for SigningKeypair {
    fn clone(&self) -> Self {
        // Reconstruct from the raw secret bytes; cheaper and version-proof
        // versus relying on a derived Clone in the dalek type.
        Self {
            signing: SigningKey::from_bytes(&self.signing.to_bytes()),
        }
    }
}
```

- [ ] **Step 4: Run the test**

```bash
cargo test -p provedex-core keys::tests::clone_preserves_identity
```

Expected: PASS.

- [ ] **Step 5: Run the full core suite + clippy to confirm no regression**

```bash
cargo test -p provedex-core
cargo clippy -p provedex-core --all-targets -- -D warnings
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add crates/provedex-core/src/keys.rs
git commit -m "feat(core): make SigningKeypair Clone

The native Python binding's Session must move a keypair into LedgerSession
from a shared pyclass; cloning is the clean path. Manual impl via secret
bytes. No key-format or invariant change, so no ADR."
```

---

### Task 3: Error hierarchy

**Files:**
- Create: `bindings/python/provedex/src/errors.rs`
- Modify: `bindings/python/provedex/src/lib.rs`
- Create: `bindings/python/provedex/tests/test_errors.py`

- [ ] **Step 1: Write the failing test `tests/test_errors.py`**

```python
import provedex


def test_exception_hierarchy():
    assert issubclass(provedex.KeyLoadError, provedex.ProvedexError)
    assert issubclass(provedex.SigningError, provedex.ProvedexError)
    assert issubclass(provedex.LedgerError, provedex.ProvedexError)
    assert issubclass(provedex.ChainError, provedex.ProvedexError)
    assert issubclass(provedex.ProvedexError, Exception)
```

- [ ] **Step 2: Run it, confirm failure**

```bash
cd bindings/python/provedex && .venv/bin/pytest tests/test_errors.py -v
```

Expected: FAIL, `module 'provedex' has no attribute 'ProvedexError'`.

- [ ] **Step 3: Write `src/errors.rs`**

```rust
//! Python exception hierarchy and core-error mapping. All binding failures
//! raise; none return error sentinels.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

create_exception!(provedex, ProvedexError, PyException, "Base class for all Provedex errors.");
create_exception!(provedex, KeyLoadError, ProvedexError, "Keypair load or save failure.");
create_exception!(provedex, SigningError, ProvedexError, "Event seal or hash failure.");
create_exception!(provedex, LedgerError, ProvedexError, "Ledger read or write failure.");
create_exception!(provedex, ChainError, ProvedexError, "Malformed verification input.");

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
    // SessionError wraps either a ledger or a signed error; surface the text.
    SigningError::new_err(e.to_string())
}
```

- [ ] **Step 4: Wire it into `src/lib.rs`**

```rust
use pyo3::prelude::*;

mod errors;

#[pymodule]
fn _provedex(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    errors::register(m)?;
    Ok(())
}
```

- [ ] **Step 5: Rebuild and run**

```bash
.venv/bin/maturin develop && .venv/bin/pytest tests/test_errors.py -v
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add bindings/python/provedex/src/errors.rs bindings/python/provedex/src/lib.rs \
  bindings/python/provedex/tests/test_errors.py
git commit -m "feat(bindings/python): exception hierarchy

ProvedexError base + KeyLoadError, SigningError, LedgerError, ChainError.
Mapping helpers from each core error type. Refs #6."
```

---

### Task 4: `SigningKeypair` pyclass

**Files:**
- Create: `bindings/python/provedex/src/keypair.rs`
- Modify: `bindings/python/provedex/src/lib.rs`
- Create: `bindings/python/provedex/tests/test_keypair.py`

- [ ] **Step 1: Write the failing test `tests/test_keypair.py`**

```python
import pytest

import provedex


def test_generate_has_64_hex_pubkey():
    kp = provedex.SigningKeypair.generate()
    assert len(kp.pubkey_hex) == 64
    int(kp.pubkey_hex, 16)  # is hex


def test_save_load_roundtrip_same_pubkey(tmp_path):
    path = str(tmp_path / "k.key")
    kp = provedex.SigningKeypair.generate()
    kp.save(path)
    loaded = provedex.SigningKeypair.load(path)
    assert loaded.pubkey_hex == kp.pubkey_hex


def test_load_or_create_is_stable(tmp_path):
    path = str(tmp_path / "nested" / "k.key")
    first = provedex.SigningKeypair.load_or_create(path)
    second = provedex.SigningKeypair.load_or_create(path)
    assert first.pubkey_hex == second.pubkey_hex


def test_load_missing_raises_key_load_error(tmp_path):
    with pytest.raises(provedex.KeyLoadError):
        provedex.SigningKeypair.load(str(tmp_path / "does-not-exist.key"))
```

- [ ] **Step 2: Run it, confirm failure**

```bash
.venv/bin/pytest tests/test_keypair.py -v
```

Expected: FAIL, no attribute `SigningKeypair`.

- [ ] **Step 3: Write `src/keypair.rs`**

```rust
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
```

- [ ] **Step 4: Register in `src/lib.rs`** (add `mod keypair;` and the class)

```rust
mod errors;
mod keypair;

#[pymodule]
fn _provedex(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    errors::register(m)?;
    m.add_class::<keypair::SigningKeypair>()?;
    Ok(())
}
```

- [ ] **Step 5: Rebuild and run**

```bash
.venv/bin/maturin develop && .venv/bin/pytest tests/test_keypair.py -v
```

Expected: all four tests PASS.

- [ ] **Step 6: Commit**

```bash
git add bindings/python/provedex/src/keypair.rs bindings/python/provedex/src/lib.rs \
  bindings/python/provedex/tests/test_keypair.py
git commit -m "feat(bindings/python): SigningKeypair pyclass

generate / load / load_or_create / save / pubkey_hex over the core keypair.
Missing-file load raises KeyLoadError. Refs #6."
```

---

### Task 5: `events` submodule, `AgentEvent` pyclass, `from_dict`

**Files:**
- Create: `bindings/python/provedex/src/events.rs`
- Modify: `bindings/python/provedex/src/lib.rs`
- Create: `bindings/python/provedex/tests/test_events.py`

- [ ] **Step 1: Write the failing test `tests/test_events.py`**

```python
import pytest

import provedex


def test_each_factory_builds_an_event():
    assert provedex.events.session_started(
        agent_id="a", model_id="m", session_id="s"
    ) is not None
    assert provedex.events.utterance_captured(
        audio_sha256="0" * 64, transcript="hi", lang="en", duration_ms=10
    ) is not None
    assert provedex.events.tool_called(
        tool_name="search", args_sha256="0" * 64, args_redacted={"q": "x"}
    ) is not None
    assert provedex.events.tool_returned(
        tool_name="search", result_sha256="0" * 64, latency_ms=5, success=True
    ) is not None
    assert provedex.events.model_invoked(
        model_id="m", prompt_sha256="0" * 64, response_sha256="0" * 64,
        prompt_tokens=5, response_tokens=2,
    ) is not None
    assert provedex.events.utterance_spoken(
        text_sha256="0" * 64, text="hello", audio_sha256="0" * 64
    ) is not None
    assert provedex.events.session_ended(
        reason="done", summary_sha256="0" * 64
    ) is not None


def test_from_dict_roundtrips_a_known_variant():
    d = {"type": "SessionEnded", "payload": {"reason": "done", "summary_sha256": "x"}}
    e = provedex.events.from_dict(d)
    assert e is not None


def test_from_dict_rejects_unknown_variant():
    with pytest.raises(provedex.SigningError):
        provedex.events.from_dict({"type": "NotAVariant", "payload": {}})
```

- [ ] **Step 2: Run it, confirm failure**

```bash
.venv/bin/pytest tests/test_events.py -v
```

Expected: FAIL, no attribute `events`.

- [ ] **Step 3: Write `src/events.rs`**

```rust
//! The seven AgentEvent variants as typed Python factories, plus a from_dict
//! reconstruction path. The variant set is locked to provedex-core; there is no
//! binding-only event.

use pyo3::prelude::*;
use pyo3::types::PyModule;
use pythonize::depythonize;

use provedex_core::AgentEvent as CoreEvent;

use crate::errors::signed_err;

/// Opaque handle around a provedex-core AgentEvent. Built only via the factory
/// functions or from_dict; Python never constructs the tagged JSON by hand.
#[pyclass]
#[derive(Clone)]
pub struct AgentEvent {
    pub(crate) inner: CoreEvent,
}

#[pymethods]
impl AgentEvent {
    fn __repr__(&self) -> String {
        // Tag only; payloads can carry transcripts we do not want in a repr.
        let tag = match &self.inner {
            CoreEvent::SessionStarted { .. } => "SessionStarted",
            CoreEvent::UtteranceCaptured { .. } => "UtteranceCaptured",
            CoreEvent::ToolCalled { .. } => "ToolCalled",
            CoreEvent::ToolReturned { .. } => "ToolReturned",
            CoreEvent::ModelInvoked { .. } => "ModelInvoked",
            CoreEvent::UtteranceSpoken { .. } => "UtteranceSpoken",
            CoreEvent::SessionEnded { .. } => "SessionEnded",
        };
        format!("AgentEvent(type='{tag}')")
    }
}

#[pyfunction]
fn session_started(agent_id: String, model_id: String, session_id: String) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::SessionStarted { agent_id, model_id, session_id },
    }
}

#[pyfunction]
fn utterance_captured(
    audio_sha256: String,
    transcript: String,
    lang: String,
    duration_ms: u64,
) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::UtteranceCaptured { audio_sha256, transcript, lang, duration_ms },
    }
}

#[pyfunction]
fn tool_called(
    tool_name: String,
    args_sha256: String,
    args_redacted: Bound<'_, PyAny>,
) -> PyResult<AgentEvent> {
    let args_redacted = depythonize(&args_redacted)
        .map_err(|e| crate::errors::SigningError::new_err(e.to_string()))?;
    Ok(AgentEvent {
        inner: CoreEvent::ToolCalled { tool_name, args_sha256, args_redacted },
    })
}

#[pyfunction]
fn tool_returned(
    tool_name: String,
    result_sha256: String,
    latency_ms: u64,
    success: bool,
) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::ToolReturned { tool_name, result_sha256, latency_ms, success },
    }
}

#[pyfunction]
fn model_invoked(
    model_id: String,
    prompt_sha256: String,
    response_sha256: String,
    prompt_tokens: u32,
    response_tokens: u32,
) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::ModelInvoked {
            model_id,
            prompt_sha256,
            response_sha256,
            prompt_tokens,
            response_tokens,
        },
    }
}

#[pyfunction]
fn utterance_spoken(text_sha256: String, text: String, audio_sha256: String) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::UtteranceSpoken { text_sha256, text, audio_sha256 },
    }
}

#[pyfunction]
fn session_ended(reason: String, summary_sha256: String) -> AgentEvent {
    AgentEvent {
        inner: CoreEvent::SessionEnded { reason, summary_sha256 },
    }
}

/// Rebuild an AgentEvent from its tagged `{"type", "payload"}` mapping. Rejects
/// any shape that is not one of the seven core variants.
#[pyfunction]
fn from_dict(value: Bound<'_, PyAny>) -> PyResult<AgentEvent> {
    let json: serde_json::Value = depythonize(&value)
        .map_err(|e| crate::errors::SigningError::new_err(e.to_string()))?;
    let inner: CoreEvent = serde_json::from_value(json)
        .map_err(|e| signed_err(provedex_core::SignedError::Json(e)))?;
    Ok(AgentEvent { inner })
}

pub(crate) fn build(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let m = PyModule::new(py, "events")?;
    m.add_class::<AgentEvent>()?;
    m.add_function(wrap_pyfunction!(session_started, &m)?)?;
    m.add_function(wrap_pyfunction!(utterance_captured, &m)?)?;
    m.add_function(wrap_pyfunction!(tool_called, &m)?)?;
    m.add_function(wrap_pyfunction!(tool_returned, &m)?)?;
    m.add_function(wrap_pyfunction!(model_invoked, &m)?)?;
    m.add_function(wrap_pyfunction!(utterance_spoken, &m)?)?;
    m.add_function(wrap_pyfunction!(session_ended, &m)?)?;
    m.add_function(wrap_pyfunction!(from_dict, &m)?)?;
    parent.add_submodule(&m)?;
    // Register in sys.modules so `import provedex.events` resolves, not just
    // attribute access through the parent package.
    py.import("sys")?
        .getattr("modules")?
        .set_item("provedex.events", &m)?;
    Ok(())
}
```

Note: `crate::errors::SigningError` must be visible. In `errors.rs` the `create_exception!` macro already makes `SigningError` a public-in-crate type; reference it as `crate::errors::SigningError`.

- [ ] **Step 4: Register in `src/lib.rs`** (add `mod events;` and `events::build(m)?;`)

```rust
mod errors;
mod events;
mod keypair;

#[pymodule]
fn _provedex(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    errors::register(m)?;
    m.add_class::<keypair::SigningKeypair>()?;
    events::build(m)?;
    Ok(())
}
```

- [ ] **Step 5: Re-export `events` in `python/provedex/__init__.py`**

```python
"""Native Python SDK for Provedex.

Signs Ed25519, hash-chained agent evidence in-process, byte-identical to the
Rust reference. See https://github.com/provedex/provedex.
"""

from ._provedex import (
    ChainError,
    KeyLoadError,
    LedgerError,
    ProvedexError,
    SigningError,
    SigningKeypair,
    __version__,
    events,
)

__all__ = [
    "__version__",
    "events",
    "SigningKeypair",
    "ProvedexError",
    "KeyLoadError",
    "SigningError",
    "LedgerError",
    "ChainError",
]
```

- [ ] **Step 6: Rebuild and run**

```bash
.venv/bin/maturin develop && .venv/bin/pytest tests/test_events.py -v
```

Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add bindings/python/provedex/src/events.rs bindings/python/provedex/src/lib.rs \
  bindings/python/provedex/python/provedex/__init__.py \
  bindings/python/provedex/tests/test_events.py
git commit -m "feat(bindings/python): events submodule with 7 typed factories

session_started, utterance_captured, tool_called, tool_returned,
model_invoked, utterance_spoken, session_ended, plus from_dict. Variant set
locked to core; unknown variants raise SigningError. Refs #6."
```

---

### Task 6: `SignedEvent` pyclass, `sign_event`, `compute_self_hash`

**Files:**
- Create: `bindings/python/provedex/src/signed.rs`
- Modify: `bindings/python/provedex/src/lib.rs`
- Create: `bindings/python/provedex/tests/test_signed.py`

- [ ] **Step 1: Write the failing test `tests/test_signed.py`**

```python
import json

import provedex


def test_sign_event_at_genesis_has_seq_zero_and_signer():
    kp = provedex.SigningKeypair.generate()
    e = provedex.events.session_started(agent_id="a", model_id="m", session_id="s")
    signed = provedex.sign_event(
        event=e, seq=0, parent_hash=provedex.GENESIS_PARENT_HASH, keypair=kp
    )
    assert signed.seq == 0
    assert signed.parent_hash == provedex.GENESIS_PARENT_HASH
    assert signed.signer_pubkey == kp.pubkey_hex
    assert len(signed.self_hash) == 64
    assert len(signed.signature) == 128


def test_signed_event_to_json_parses_and_has_fields():
    kp = provedex.SigningKeypair.generate()
    e = provedex.events.session_ended(reason="done", summary_sha256="x")
    signed = provedex.sign_event(
        event=e, seq=0, parent_hash=provedex.GENESIS_PARENT_HASH, keypair=kp
    )
    parsed = json.loads(signed.to_json())
    assert parsed["seq"] == 0
    assert parsed["event"]["type"] == "SessionEnded"
    assert signed.event["type"] == "SessionEnded"
    assert signed.event["payload"]["reason"] == "done"


def test_compute_self_hash_is_deterministic_for_fixed_inputs():
    e = provedex.events.session_started(agent_id="a", model_id="m", session_id="s")
    h1 = provedex.compute_self_hash(
        seq=0, timestamp_nanos=1234, event=e, parent_hash=provedex.GENESIS_PARENT_HASH
    )
    h2 = provedex.compute_self_hash(
        seq=0, timestamp_nanos=1234, event=e, parent_hash=provedex.GENESIS_PARENT_HASH
    )
    assert h1 == h2
    assert len(h1) == 64
```

- [ ] **Step 2: Run it, confirm failure**

```bash
.venv/bin/pytest tests/test_signed.py -v
```

Expected: FAIL, no attribute `sign_event`.

- [ ] **Step 3: Write `src/signed.rs`**

```rust
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
    fn event(&self, py: Python<'_>) -> PyResult<PyObject> {
        let value = serde_json::to_value(&self.inner.event)
            .map_err(|e| signed_err(provedex_core::SignedError::Json(e)))?;
        let obj = pythonize::pythonize(py, &value)
            .map_err(|e| crate::errors::SigningError::new_err(e.to_string()))?;
        Ok(obj.into())
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
        format!("SignedEvent(seq={}, self_hash='{}')", self.inner.seq, self.inner.self_hash)
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
    let bytes = core_self_hash(seq, timestamp_nanos, &event.inner, parent_hash)
        .map_err(signed_err)?;
    Ok(hex::encode(bytes))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SignedEvent>()?;
    m.add_function(wrap_pyfunction!(sign_event, m)?)?;
    m.add_function(wrap_pyfunction!(compute_self_hash, m)?)?;
    m.add("GENESIS_PARENT_HASH", provedex_core::GENESIS_PARENT_HASH)?;
    Ok(())
}
```

Note: this uses the `hex` crate. Add `hex = "0.4"` to `[dependencies]` in `Cargo.toml`.

- [ ] **Step 4: Add `hex` dep and register in `src/lib.rs`**

Add to `Cargo.toml` `[dependencies]`:

```toml
hex = "0.4"
```

Update `src/lib.rs`:

```rust
mod errors;
mod events;
mod keypair;
mod signed;

#[pymodule]
fn _provedex(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    errors::register(m)?;
    m.add_class::<keypair::SigningKeypair>()?;
    events::build(m)?;
    signed::register(m)?;
    Ok(())
}
```

- [ ] **Step 5: Re-export in `python/provedex/__init__.py`** (add the new names)

Add `GENESIS_PARENT_HASH`, `SignedEvent`, `sign_event`, `compute_self_hash` to the import block and `__all__`:

```python
from ._provedex import (
    ChainError,
    GENESIS_PARENT_HASH,
    KeyLoadError,
    LedgerError,
    ProvedexError,
    SignedEvent,
    SigningError,
    SigningKeypair,
    __version__,
    compute_self_hash,
    events,
    sign_event,
)

__all__ = [
    "__version__",
    "events",
    "SigningKeypair",
    "SignedEvent",
    "sign_event",
    "compute_self_hash",
    "GENESIS_PARENT_HASH",
    "ProvedexError",
    "KeyLoadError",
    "SigningError",
    "LedgerError",
    "ChainError",
]
```

- [ ] **Step 6: Rebuild and run**

```bash
.venv/bin/maturin develop && .venv/bin/pytest tests/test_signed.py -v
```

Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add bindings/python/provedex/src/signed.rs bindings/python/provedex/src/lib.rs \
  bindings/python/provedex/Cargo.toml bindings/python/provedex/python/provedex/__init__.py \
  bindings/python/provedex/tests/test_signed.py
git commit -m "feat(bindings/python): SignedEvent, sign_event, compute_self_hash

Read-only frozen SignedEvent view with to_json matching the ledger line.
Low-level sign_event plus compute_self_hash for verifier/compat use.
GENESIS_PARENT_HASH exposed as a module constant. Refs #6."
```

---

### Task 7: `Session` pyclass

**Files:**
- Create: `bindings/python/provedex/src/session.rs`
- Modify: `bindings/python/provedex/src/lib.rs`
- Create: `bindings/python/provedex/tests/test_session.py`

- [ ] **Step 1: Write the failing test `tests/test_session.py`**

```python
import provedex


def test_record_chains_seq_and_parent(tmp_path):
    kp = provedex.SigningKeypair.generate()
    ledger = str(tmp_path / "ledger.ndjson")
    s = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    assert s.session_id == "s1"
    assert s.pubkey_hex == kp.pubkey_hex

    a = s.record(provedex.events.session_started(agent_id="a", model_id="m", session_id="s1"))
    b = s.record(provedex.events.session_ended(reason="done", summary_sha256="x"))

    assert a.seq == 0
    assert a.parent_hash == provedex.GENESIS_PARENT_HASH
    assert b.seq == 1
    assert b.parent_hash == a.self_hash


def test_reopen_resumes_seq(tmp_path):
    kp = provedex.SigningKeypair.generate()
    ledger = str(tmp_path / "ledger.ndjson")

    s1 = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    s1.record(provedex.events.session_started(agent_id="a", model_id="m", session_id="s1"))

    s2 = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    c = s2.record(provedex.events.session_ended(reason="done", summary_sha256="x"))
    assert c.seq == 1
```

- [ ] **Step 2: Run it, confirm failure**

```bash
.venv/bin/pytest tests/test_session.py -v
```

Expected: FAIL, no attribute `Session`.

- [ ] **Step 3: Write `src/session.rs`**

```rust
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
```

- [ ] **Step 4: Register in `src/lib.rs`** (add `mod session;` and `session::register(m)?;`)

```rust
mod errors;
mod events;
mod keypair;
mod session;
mod signed;

#[pymodule]
fn _provedex(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    errors::register(m)?;
    m.add_class::<keypair::SigningKeypair>()?;
    events::build(m)?;
    signed::register(m)?;
    session::register(m)?;
    Ok(())
}
```

- [ ] **Step 5: Re-export `Session` in `python/provedex/__init__.py`** (add to import + `__all__`)

Add `Session` to the `from ._provedex import (...)` block (keep alphabetical) and add `"Session",` to `__all__`.

- [ ] **Step 6: Rebuild and run**

```bash
.venv/bin/maturin develop && .venv/bin/pytest tests/test_session.py -v
```

Expected: both PASS.

- [ ] **Step 7: Commit**

```bash
git add bindings/python/provedex/src/session.rs bindings/python/provedex/src/lib.rs \
  bindings/python/provedex/python/provedex/__init__.py \
  bindings/python/provedex/tests/test_session.py
git commit -m "feat(bindings/python): Session pyclass

Wraps core LedgerSession: auto seq + parent chaining + fsync append, resumes
on reopen. record() releases the GIL across seal+fsync. Refs #6."
```

---

### Task 8: `ChainReport`, `verify_chain`, `verify_file`

**Files:**
- Create: `bindings/python/provedex/src/verify.rs`
- Modify: `bindings/python/provedex/src/lib.rs`
- Create: `bindings/python/provedex/tests/test_verify.py`

- [ ] **Step 1: Write the failing test `tests/test_verify.py`**

```python
import provedex


def _build_ledger(tmp_path):
    kp = provedex.SigningKeypair.generate()
    ledger = str(tmp_path / "ledger.ndjson")
    s = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    events = [
        s.record(provedex.events.session_started(agent_id="a", model_id="m", session_id="s1")),
        s.record(provedex.events.session_ended(reason="done", summary_sha256="x")),
    ]
    return ledger, events


def test_verify_chain_ok_for_good_chain(tmp_path):
    _, events = _build_ledger(tmp_path)
    report = provedex.verify_chain(events)
    assert report.ok is True
    assert report.event_count == 2
    assert report.broken_at is None
    assert report.reason is None


def test_verify_file_ok(tmp_path):
    ledger, _ = _build_ledger(tmp_path)
    report = provedex.verify_file(ledger)
    assert report.ok is True
    assert report.event_count == 2


def test_verify_file_empty_for_missing(tmp_path):
    report = provedex.verify_file(str(tmp_path / "nope.ndjson"))
    assert report.ok is True
    assert report.event_count == 0
```

- [ ] **Step 2: Run it, confirm failure**

```bash
.venv/bin/pytest tests/test_verify.py -v
```

Expected: FAIL, no attribute `verify_chain`.

- [ ] **Step 3: Write `src/verify.rs`**

```rust
//! Chain verification surface. A broken-but-parseable chain is data, returned
//! as ChainReport(ok=False); only unparseable input raises ChainError.

use std::path::PathBuf;

use pyo3::prelude::*;

use provedex_core::{read_file, verify_chain as core_verify, ChainReport as CoreReport, ChainStatus};

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
```

- [ ] **Step 4: Register in `src/lib.rs`** (add `mod verify;` and `verify::register(m)?;`)

- [ ] **Step 5: Re-export in `python/provedex/__init__.py`** (add `ChainReport`, `verify_chain`, `verify_file` to import + `__all__`)

- [ ] **Step 6: Rebuild and run**

```bash
.venv/bin/maturin develop && .venv/bin/pytest tests/test_verify.py -v
```

Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add bindings/python/provedex/src/verify.rs bindings/python/provedex/src/lib.rs \
  bindings/python/provedex/python/provedex/__init__.py \
  bindings/python/provedex/tests/test_verify.py
git commit -m "feat(bindings/python): verify_chain, verify_file, ChainReport

ChainReport exposes ok / status / event_count / broken_at / reason /
root_hash. A broken chain is data (ok=False), not an exception. Refs #6."
```

---

### Task 9: `canonical_json`

**Files:**
- Create: `bindings/python/provedex/src/canonical.rs`
- Modify: `bindings/python/provedex/src/lib.rs`
- Create: `bindings/python/provedex/tests/test_canonical.py`

- [ ] **Step 1: Write the failing test `tests/test_canonical.py`**

```python
import provedex


def test_sorts_object_keys_and_strips_whitespace():
    assert provedex.canonical_json({"b": 1, "a": 2}) == b'{"a":2,"b":1}'


def test_nested_arrays_preserved_in_order():
    assert provedex.canonical_json({"c": [3, 2, 1], "a": 2}) == b'{"a":2,"c":[3,2,1]}'


def test_non_ascii_passes_through_as_raw_utf8():
    # The Rust encoder does NOT \\u-escape non-ASCII; it emits raw UTF-8 bytes.
    assert provedex.canonical_json({"k": "café"}) == '{"k":"café"}'.encode("utf-8")


def test_control_chars_escaped():
    assert provedex.canonical_json({"k": "a\nb"}) == b'{"k":"a\\nb"}'
```

- [ ] **Step 2: Run it, confirm failure**

```bash
.venv/bin/pytest tests/test_canonical.py -v
```

Expected: FAIL, no attribute `canonical_json`.

- [ ] **Step 3: Write `src/canonical.rs`**

```rust
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
    Ok(PyBytes::new(py, &bytes))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(canonical_json, m)?)?;
    Ok(())
}
```

- [ ] **Step 4: Register in `src/lib.rs`** (add `mod canonical;` and `canonical::register(m)?;`)

- [ ] **Step 5: Re-export in `python/provedex/__init__.py`** (add `canonical_json` to import + `__all__`)

- [ ] **Step 6: Rebuild and run**

```bash
.venv/bin/maturin develop && .venv/bin/pytest tests/test_canonical.py -v
```

Expected: all PASS.

- [ ] **Step 7: Commit**

```bash
git add bindings/python/provedex/src/canonical.rs bindings/python/provedex/src/lib.rs \
  bindings/python/provedex/python/provedex/__init__.py \
  bindings/python/provedex/tests/test_canonical.py
git commit -m "feat(bindings/python): canonical_json

Exposes the core deterministic encoder: sorted keys, no whitespace, raw UTF-8
non-ASCII. Refs #6."
```

---

### Task 10: Type stubs + py.typed + mypy clean

**Files:**
- Create: `bindings/python/provedex/python/provedex/__init__.pyi`
- Create: `bindings/python/provedex/python/provedex/events.pyi`
- Modify: `bindings/python/provedex/tests/` (add a typed usage test)

- [ ] **Step 1: Write `python/provedex/__init__.pyi`**

```python
from collections.abc import Mapping
from typing import Any

__version__: str
GENESIS_PARENT_HASH: str

from . import events as events

class SigningKeypair:
    @staticmethod
    def generate() -> SigningKeypair: ...
    @staticmethod
    def load(path: str) -> SigningKeypair: ...
    @staticmethod
    def load_or_create(path: str) -> SigningKeypair: ...
    def save(self, path: str) -> None: ...
    @property
    def pubkey_hex(self) -> str: ...

class SignedEvent:
    @property
    def seq(self) -> int: ...
    @property
    def timestamp_nanos(self) -> int: ...
    @property
    def event(self) -> dict[str, Any]: ...
    @property
    def parent_hash(self) -> str: ...
    @property
    def self_hash(self) -> str: ...
    @property
    def signature(self) -> str: ...
    @property
    def signer_pubkey(self) -> str: ...
    def to_json(self) -> str: ...

class Session:
    @staticmethod
    def open(
        *, keypair: SigningKeypair, ledger_path: str, session_id: str
    ) -> Session: ...
    def record(self, event: events.AgentEvent) -> SignedEvent: ...
    @property
    def session_id(self) -> str: ...
    @property
    def pubkey_hex(self) -> str: ...

class ChainReport:
    @property
    def ok(self) -> bool: ...
    @property
    def status(self) -> str: ...
    @property
    def event_count(self) -> int: ...
    @property
    def broken_at(self) -> int | None: ...
    @property
    def reason(self) -> str | None: ...
    @property
    def root_hash(self) -> str: ...

def sign_event(
    *, event: events.AgentEvent, seq: int, parent_hash: str, keypair: SigningKeypair
) -> SignedEvent: ...
def compute_self_hash(
    *, seq: int, timestamp_nanos: int, event: events.AgentEvent, parent_hash: str
) -> str: ...
def verify_chain(events: list[SignedEvent]) -> ChainReport: ...
def verify_file(path: str) -> ChainReport: ...
def canonical_json(value: Mapping[str, Any] | list[Any] | str | int | float | bool | None) -> bytes: ...

class ProvedexError(Exception): ...
class KeyLoadError(ProvedexError): ...
class SigningError(ProvedexError): ...
class LedgerError(ProvedexError): ...
class ChainError(ProvedexError): ...
```

- [ ] **Step 2: Write `python/provedex/events.pyi`**

```python
from collections.abc import Mapping
from typing import Any

class AgentEvent: ...

def session_started(*, agent_id: str, model_id: str, session_id: str) -> AgentEvent: ...
def utterance_captured(
    *, audio_sha256: str, transcript: str, lang: str, duration_ms: int
) -> AgentEvent: ...
def tool_called(
    *, tool_name: str, args_sha256: str, args_redacted: Mapping[str, Any] | list[Any]
) -> AgentEvent: ...
def tool_returned(
    *, tool_name: str, result_sha256: str, latency_ms: int, success: bool
) -> AgentEvent: ...
def model_invoked(
    *, model_id: str, prompt_sha256: str, response_sha256: str,
    prompt_tokens: int, response_tokens: int,
) -> AgentEvent: ...
def utterance_spoken(*, text_sha256: str, text: str, audio_sha256: str) -> AgentEvent: ...
def session_ended(*, reason: str, summary_sha256: str) -> AgentEvent: ...
def from_dict(value: Mapping[str, Any]) -> AgentEvent: ...
```

Note: the factory functions are defined in Rust with positional params; PyO3 accepts them as keyword args by default, so the keyword-only `*,` stubs match real call sites in the examples and tests. If mypy flags a positional call anywhere, update that call site to use keywords (the public API is keyword-based).

- [ ] **Step 3: Write a typed usage test `tests/test_typing.py`**

```python
import provedex


def test_typed_usage_compiles_and_runs(tmp_path):
    kp = provedex.SigningKeypair.generate()
    s = provedex.Session.open(
        keypair=kp, ledger_path=str(tmp_path / "l.ndjson"), session_id="s1"
    )
    signed = s.record(
        provedex.events.session_started(agent_id="a", model_id="m", session_id="s1")
    )
    report = provedex.verify_chain([signed])
    assert report.ok is True
```

- [ ] **Step 4: Run mypy against the stubs and the test, plus the test itself**

```bash
.venv/bin/pip install -e ".[dev]"
.venv/bin/mypy python/provedex tests/test_typing.py
.venv/bin/pytest tests/test_typing.py -v
```

Expected: `mypy` reports no errors; the test PASSES. If `mypy` cannot find `provedex`, ensure `maturin develop` placed the package in the venv site-packages and that `python/provedex/py.typed` exists.

- [ ] **Step 5: Commit**

```bash
git add bindings/python/provedex/python/provedex/__init__.pyi \
  bindings/python/provedex/python/provedex/events.pyi \
  bindings/python/provedex/tests/test_typing.py
git commit -m "feat(bindings/python): ship type stubs + py.typed

__init__.pyi and events.pyi cover the full surface; mypy strict clean.
Refs #6."
```

---

### Task 11: Golden-vector generator (Rust) + committed vectors

**Files:**
- Create: `crates/provedex-core/examples/emit_compat_vectors.rs`
- Create: `tests/compat/vectors/canonical_json.json` (generated)
- Create: `tests/compat/vectors/self_hash.json` (generated)
- Create: `tests/compat/vectors/rust_signed_ledger.ndjson` (generated, a real Rust-signed chain Python must verify)
- Create: `tests/compat/README.md` (if not present, document the vectors)

- [ ] **Step 1: Write the generator `crates/provedex-core/examples/emit_compat_vectors.rs`**

```rust
//! Emit byte-compat golden vectors as language-neutral JSON. Inputs use fixed
//! seq/timestamp values so output is deterministic across runs. Run with:
//!   cargo run -p provedex-core --example emit_compat_vectors
//! Writes tests/compat/vectors/{canonical_json,self_hash}.json relative to the
//! repo root.

use std::fs;
use std::path::Path;

use provedex_core::{
    canonical_json, compute_self_hash, AgentEvent, Ledger, LedgerSession, SigningKeypair,
};
use serde_json::{json, Value};

fn canonical_cases() -> Vec<Value> {
    let inputs = vec![
        ("sorted_keys", json!({"b": 1, "a": 2, "c": 3})),
        ("nested_array", json!({"c": [3, 2, 1], "a": 2})),
        ("control_chars", json!({"k": "line1\nline2\t\"end\""})),
        ("non_ascii_raw_utf8", json!({"k": "café \u{2705} \u{1f512}"})),
        ("ints_and_bools", json!({"n": 42, "z": 0, "flag": true, "nil": null})),
        ("empty_object_and_array", json!({"o": {}, "a": []})),
    ];
    inputs
        .into_iter()
        .map(|(name, input)| {
            let bytes = canonical_json(&input);
            json!({
                "name": name,
                "input": input,
                "expected": String::from_utf8(bytes).expect("canonical json is valid utf-8"),
            })
        })
        .collect()
}

fn self_hash_cases() -> Vec<Value> {
    let events = vec![
        (
            "session_started",
            AgentEvent::SessionStarted {
                agent_id: "agent-1".into(),
                model_id: "llama3.2:3b".into(),
                session_id: "sess-1".into(),
            },
        ),
        (
            "model_invoked",
            AgentEvent::ModelInvoked {
                model_id: "gpt-4o".into(),
                prompt_sha256: "a".repeat(64),
                response_sha256: "b".repeat(64),
                prompt_tokens: 12,
                response_tokens: 34,
            },
        ),
        (
            "tool_called_non_ascii",
            AgentEvent::ToolCalled {
                tool_name: "search".into(),
                args_sha256: "c".repeat(64),
                args_redacted: json!({"q": "café \u{2705}"}),
            },
        ),
    ];
    let parent = "0".repeat(64);
    events
        .into_iter()
        .enumerate()
        .map(|(i, (name, event))| {
            let seq = i as u64;
            let timestamp_nanos = 1_700_000_000_000_000_000u64 + seq;
            let hash = compute_self_hash(seq, timestamp_nanos, &event, &parent)
                .expect("hash a known-good event");
            json!({
                "name": name,
                "seq": seq,
                "timestamp_nanos": timestamp_nanos,
                "event": serde_json::to_value(&event).unwrap(),
                "parent_hash": parent,
                "self_hash": hex::encode(hash),
            })
        })
        .collect()
}

/// Emit a real Rust-signed ledger that the Python binding must verify as VALID.
/// This is the "Rust signs, Python verifies" direction of cross-verification.
/// A test-only fixed secret key keeps the signer identity stable across
/// regenerations (timestamps and signatures still vary run to run, which is
/// fine: the consumer only asserts the chain verifies, not specific bytes).
fn emit_signed_ledger(dir: &Path) {
    let path = dir.join("rust_signed_ledger.ndjson");
    // Fresh file each run; appending would duplicate a prior chain.
    let _ = fs::remove_file(&path);

    // SigningKeypair has no public from_bytes; round-trip a fixed secret
    // through a temp key file. [7u8; 32] is a fixture key, not a real identity.
    let key_path = dir.join(".fixture.key");
    fs::write(&key_path, [7u8; 32]).expect("write fixture key");
    let kp = SigningKeypair::load(&key_path).expect("load fixture key");
    fs::remove_file(&key_path).ok();

    let ledger = Ledger::open(&path).expect("open fixture ledger");
    let session = LedgerSession::open(kp, ledger, "rust-fixture".into())
        .expect("open fixture session");
    session
        .seal_and_append(AgentEvent::SessionStarted {
            agent_id: "rust-agent".into(),
            model_id: "gpt-4o".into(),
            session_id: "rust-fixture".into(),
        })
        .expect("seal 0");
    session
        .seal_and_append(AgentEvent::ModelInvoked {
            model_id: "gpt-4o".into(),
            prompt_sha256: "a".repeat(64),
            response_sha256: "b".repeat(64),
            prompt_tokens: 12,
            response_tokens: 34,
        })
        .expect("seal 1");
    session
        .seal_and_append(AgentEvent::SessionEnded {
            reason: "completed".into(),
            summary_sha256: "c".repeat(64),
        })
        .expect("seal 2");
}

fn main() {
    let dir = Path::new("tests/compat/vectors");
    fs::create_dir_all(dir).expect("create vectors dir");
    let canonical = Value::Array(canonical_cases());
    let self_hash = Value::Array(self_hash_cases());
    fs::write(
        dir.join("canonical_json.json"),
        serde_json::to_string_pretty(&canonical).unwrap() + "\n",
    )
    .expect("write canonical_json.json");
    fs::write(
        dir.join("self_hash.json"),
        serde_json::to_string_pretty(&self_hash).unwrap() + "\n",
    )
    .expect("write self_hash.json");
    emit_signed_ledger(dir);
    println!(
        "wrote {} canonical + {} self_hash vectors + rust_signed_ledger.ndjson",
        canonical.as_array().unwrap().len(),
        self_hash.as_array().unwrap().len(),
    );
}
```

Note: the example uses `hex`, `Ledger`, `LedgerSession`, and `SigningKeypair`, all already in `provedex-core`.

- [ ] **Step 2: Run the generator from the repo root**

```bash
cd /Users/adi/Desktop/provedex
cargo run -p provedex-core --example emit_compat_vectors
```

Expected: prints `wrote 6 canonical + 3 self_hash vectors + rust_signed_ledger.ndjson`; creates the three files under `tests/compat/vectors/`.

- [ ] **Step 3: Sanity-check the generated files exist and are non-empty, and the ledger fixture verifies with the CLI**

```bash
test -s tests/compat/vectors/canonical_json.json \
  && test -s tests/compat/vectors/self_hash.json \
  && test -s tests/compat/vectors/rust_signed_ledger.ndjson \
  && cargo run -q -p provedex-cli -- verify --ledger tests/compat/vectors/rust_signed_ledger.ndjson \
  && echo OK
```

Expected: the CLI prints `status: VALID` / `events: 3`, then `OK`.

- [ ] **Step 4: Confirm the example builds under clippy (it is an --all-targets target)**

```bash
cargo clippy -p provedex-core --all-targets -- -D warnings
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add crates/provedex-core/examples/emit_compat_vectors.rs tests/compat/vectors/
git commit -m "test(compat): golden-vector generator + committed vectors

emit_compat_vectors emits deterministic canonical-JSON and self_hash goldens
plus a real Rust-signed ledger fixture for cross-verification. Unblocks #5;
consumed by the Python binding."
```

---

### Task 12: Python byte-compat tests against the goldens

**Files:**
- Create: `bindings/python/provedex/tests/test_compat.py`

- [ ] **Step 1: Write the failing test `tests/test_compat.py`**

```python
import json
from pathlib import Path

import pytest

import provedex

# tests/ -> provedex/ -> python/ -> bindings/ -> repo root
_VECTORS = Path(__file__).resolve().parents[4] / "tests" / "compat" / "vectors"


def _load(name):
    path = _VECTORS / name
    if not path.exists():
        pytest.skip(f"golden vectors not generated: {path}")
    return json.loads(path.read_text())


def test_canonical_json_matches_rust_goldens():
    for case in _load("canonical_json.json"):
        got = provedex.canonical_json(case["input"])
        assert got == case["expected"].encode("utf-8"), case["name"]


def test_self_hash_matches_rust_goldens():
    for case in _load("self_hash.json"):
        event = provedex.events.from_dict(case["event"])
        got = provedex.compute_self_hash(
            seq=case["seq"],
            timestamp_nanos=case["timestamp_nanos"],
            event=event,
            parent_hash=case["parent_hash"],
        )
        assert got == case["self_hash"], case["name"]
```

- [ ] **Step 2: Run it**

```bash
cd bindings/python/provedex && .venv/bin/pytest tests/test_compat.py -v
```

Expected: both PASS (the binding reproduces the Rust goldens byte-for-byte). If the vectors path is wrong for this layout, fix `parents[4]` to point at the repo root and re-run.

- [ ] **Step 3: Commit**

```bash
git add bindings/python/provedex/tests/test_compat.py
git commit -m "test(bindings/python): byte-compat against Rust goldens

Asserts canonical_json and compute_self_hash reproduce the committed Rust
golden vectors exactly. Refs #5, #6."
```

---

### Task 13: Cross-verify integration tests (Rust CLI <-> Python)

**Files:**
- Create: `bindings/python/provedex/tests/conftest.py`
- Create: `bindings/python/provedex/tests/test_cross_verify.py`

- [ ] **Step 1: Write `tests/conftest.py`** (builds and locates the `provedex` CLI)

```python
import shutil
import subprocess
from pathlib import Path

import pytest

# tests/ -> provedex/ -> python/ -> bindings/ -> repo root
_REPO_ROOT = Path(__file__).resolve().parents[4]


@pytest.fixture(scope="session")
def provedex_cli() -> str:
    """Build the provedex CLI once and return its path. Skips integration tests
    if cargo is unavailable."""
    if shutil.which("cargo") is None:
        pytest.skip("cargo not available; cannot build the provedex CLI")
    subprocess.run(
        ["cargo", "build", "--release", "-p", "provedex-cli"],
        cwd=_REPO_ROOT,
        check=True,
    )
    binary = _REPO_ROOT / "target" / "release" / "provedex"
    assert binary.exists(), f"CLI not found at {binary}"
    return str(binary)
```

- [ ] **Step 2: Write `tests/test_cross_verify.py`**

```python
import subprocess

import pytest

import provedex


@pytest.mark.integration
def test_python_signed_ledger_verifies_with_rust_cli(tmp_path, provedex_cli):
    ledger = str(tmp_path / "ledger.ndjson")
    kp = provedex.SigningKeypair.generate()
    s = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    s.record(provedex.events.session_started(agent_id="a", model_id="m", session_id="s1"))
    s.record(
        provedex.events.model_invoked(
            model_id="m", prompt_sha256="a" * 64, response_sha256="b" * 64,
            prompt_tokens=5, response_tokens=2,
        )
    )
    s.record(provedex.events.session_ended(reason="done", summary_sha256="x"))

    result = subprocess.run(
        [provedex_cli, "verify", "--ledger", ledger],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "status: VALID" in result.stdout
    assert "events: 3" in result.stdout


def test_rust_signed_fixture_verifies_in_python():
    # Reverse direction: a ledger SIGNED BY RUST (the committed fixture from
    # Task 11's generator) must verify VALID through the Python binding. This is
    # the true "Rust signs, Python verifies" half of byte-compat. Not an
    # integration test: it needs no CLI, only the committed fixture.
    from pathlib import Path

    fixture = (
        Path(__file__).resolve().parents[4]
        / "tests" / "compat" / "vectors" / "rust_signed_ledger.ndjson"
    )
    if not fixture.exists():
        pytest.skip(f"rust-signed fixture not generated: {fixture}")
    report = provedex.verify_file(str(fixture))
    assert report.ok is True
    assert report.event_count == 3


@pytest.mark.integration
def test_cli_and_python_agree_a_tampered_ledger_is_broken(tmp_path, provedex_cli):
    # Both implementations must independently judge the same tampered chain
    # broken, proving they recompute self_hash over identical bytes.
    ledger = str(tmp_path / "ledger.ndjson")
    kp = provedex.SigningKeypair.generate()
    s = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    s.record(provedex.events.session_started(agent_id="a", model_id="m", session_id="s1"))
    s.record(provedex.events.session_ended(reason="done", summary_sha256="x"))

    # Tamper: flip a value inside the second line's payload.
    with open(ledger) as f:
        lines = f.readlines()
    lines[1] = lines[1].replace('"done"', '"tampered"')
    with open(ledger, "w") as f:
        f.writelines(lines)

    py_report = provedex.verify_file(ledger)
    assert py_report.ok is False

    result = subprocess.run(
        [provedex_cli, "verify", "--ledger", ledger],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 1
    assert "status: BROKEN" in result.stdout
```

This gives both cross-verify directions plus a tamper-agreement check:
- **Python signs, Rust verifies** (`test_python_signed_ledger_verifies_with_rust_cli`): forward.
- **Rust signs, Python verifies** (`test_rust_signed_fixture_verifies_in_python`): reverse, against the committed Rust-signed fixture.
- **Both detect tampering identically** (`test_cli_and_python_agree_a_tampered_ledger_is_broken`): they recompute self_hash over the same bytes.

- [ ] **Step 3: Run the integration tests**

```bash
cd bindings/python/provedex && .venv/bin/pytest tests/test_cross_verify.py -v -m integration
```

Expected: both PASS (first build of the CLI may take a minute).

- [ ] **Step 4: Run the whole suite once (unit + integration)**

```bash
.venv/bin/pytest -v
```

Expected: every test PASSES.

- [ ] **Step 5: Commit**

```bash
git add bindings/python/provedex/tests/conftest.py \
  bindings/python/provedex/tests/test_cross_verify.py
git commit -m "test(bindings/python): cross-verify both directions with Rust

Python-signed ledger verifies VALID under provedex verify (forward); the
committed Rust-signed fixture verifies VALID in Python (reverse); both judge
a tampered ledger BROKEN. Proves the implementations agree on self_hash +
signature over identical bytes. Refs #5, #6."
```

---

### Task 14: CI job

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add a `bindings-python-native` job** (append after the existing `bindings-python` job, same indentation level under `jobs:`)

```yaml
  bindings-python-native:
    name: bindings-python-native (maturin + pytest)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: install build dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y build-essential cmake clang

      - name: install rust toolchain from rust-toolchain.toml
        run: rustup show active-toolchain || rustup toolchain install

      - name: cache cargo registry + target
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry/index
            ~/.cargo/registry/cache
            ~/.cargo/git/db
            target
            bindings/python/provedex/target
          key: ${{ runner.os }}-cargo-pynative-${{ hashFiles('Cargo.lock', 'bindings/python/provedex/Cargo.lock', 'rust-toolchain.toml') }}
          restore-keys: |
            ${{ runner.os }}-cargo-pynative-
            ${{ runner.os }}-cargo-

      - name: setup python 3.11
        uses: actions/setup-python@v5
        with:
          python-version: "3.11"

      - name: generate compat vectors
        run: cargo run -p provedex-core --example emit_compat_vectors

      - name: build provedex CLI (for cross-verify)
        run: cargo build --release -p provedex-cli

      - name: install binding (editable, dev deps) + build extension
        working-directory: bindings/python/provedex
        run: |
          python -m pip install --upgrade pip maturin
          pip install -e ".[dev]"
          maturin develop

      - name: lint (ruff)
        working-directory: bindings/python/provedex
        run: ruff check python tests

      - name: typecheck (mypy)
        working-directory: bindings/python/provedex
        run: mypy python tests

      - name: unit tests
        working-directory: bindings/python/provedex
        run: pytest -v -m "not integration"

      - name: integration tests (cross-verify with the CLI)
        working-directory: bindings/python/provedex
        run: pytest -v -m integration
```

- [ ] **Step 2: Validate the YAML locally**

```bash
cd /Users/adi/Desktop/provedex
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); print('yaml ok')"
```

Expected: `yaml ok`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: build + test the native python binding

New bindings-python-native job: generate compat vectors, build the CLI,
maturin develop, ruff, mypy, unit + integration (cross-verify) tests.
Refs #6."
```

---

### Task 15: README, RELEASING, example

**Files:**
- Create: `bindings/python/provedex/README.md`
- Create: `bindings/python/provedex/RELEASING.md`
- Create: `bindings/python/provedex/examples/basic.py`

- [ ] **Step 1: Write `examples/basic.py`**

```python
"""Minimal end-to-end: open a session, sign three events, verify the ledger."""

import tempfile
from pathlib import Path

import provedex


def main() -> None:
    workdir = Path(tempfile.mkdtemp())
    keypair = provedex.SigningKeypair.load_or_create(str(workdir / "ed25519.key"))
    session = provedex.Session.open(
        keypair=keypair,
        ledger_path=str(workdir / "ledger.ndjson"),
        session_id="demo-session",
    )

    session.record(
        provedex.events.session_started(
            agent_id="demo-agent", model_id="gpt-4o", session_id="demo-session"
        )
    )
    session.record(
        provedex.events.model_invoked(
            model_id="gpt-4o",
            prompt_sha256="a" * 64,
            response_sha256="b" * 64,
            prompt_tokens=12,
            response_tokens=34,
        )
    )
    session.record(
        provedex.events.session_ended(reason="completed", summary_sha256="c" * 64)
    )

    report = session_verify(workdir)
    print(f"signer: {keypair.pubkey_hex}")
    print(f"ledger: {workdir / 'ledger.ndjson'}")
    print(f"verified: ok={report.ok} events={report.event_count}")


def session_verify(workdir: Path) -> provedex.ChainReport:
    return provedex.verify_file(str(workdir / "ledger.ndjson"))


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run the example to confirm it works**

```bash
cd bindings/python/provedex && .venv/bin/python examples/basic.py
```

Expected: prints a signer pubkey, a ledger path, and `verified: ok=True events=3`.

- [ ] **Step 3: Write `README.md`**

````markdown
# provedex (native Python SDK)

Native, in-process Ed25519 signing and hash-chaining for AI-agent evidence.
Byte-identical to the Provedex Rust reference: a ledger signed here verifies
with `provedex verify`, and vice versa.

This is the opt-in fast-path. The default integration for non-Rust apps is the
localhost sidecar (`provedex-agent`); see ADR 0004. Use this binding when you
want sub-millisecond, in-process signing with no extra process to run.

## Install

```bash
pip install provedex
```

Pre-built wheels ship for cpython 3.11+ on Linux x86_64, Linux aarch64, and
macOS arm64. No Rust toolchain required to install.

## Quickstart

```python
import provedex

keypair = provedex.SigningKeypair.load_or_create("~/.provedex/keys/ed25519.key")
session = provedex.Session.open(
    keypair=keypair, ledger_path="./ledger.ndjson", session_id="conversation-42"
)

session.record(
    provedex.events.session_started(
        agent_id="intake-bot", model_id="gpt-4o", session_id="conversation-42"
    )
)
signed = session.record(
    provedex.events.model_invoked(
        model_id="gpt-4o",
        prompt_sha256="...", response_sha256="...",
        prompt_tokens=120, response_tokens=80,
    )
)
print(signed.seq, signed.self_hash)

report = provedex.verify_file("./ledger.ndjson")
assert report.ok
```

## Events

One typed factory per core variant. The variant set is locked to the Rust core;
there is no Python-only event.

| Factory | Signs |
|---------|-------|
| `events.session_started(agent_id, model_id, session_id)` | session open |
| `events.utterance_captured(audio_sha256, transcript, lang, duration_ms)` | inbound speech |
| `events.tool_called(tool_name, args_sha256, args_redacted)` | tool invocation |
| `events.tool_returned(tool_name, result_sha256, latency_ms, success)` | tool result |
| `events.model_invoked(model_id, prompt_sha256, response_sha256, prompt_tokens, response_tokens)` | LLM call |
| `events.utterance_spoken(text_sha256, text, audio_sha256)` | outbound speech |
| `events.session_ended(reason, summary_sha256)` | session close |

`events.from_dict({"type": ..., "payload": ...})` rebuilds an event from its
stored JSON.

## Sessions vs. low-level signing

`Session` is the primary path: it allocates the next `seq`, chains each event to
the previous `self_hash`, appends to the ledger, and fsyncs, resuming from any
pre-existing events on open. For full manual control there is a low-level path:

```python
signed = provedex.sign_event(
    event=e, seq=0, parent_hash=provedex.GENESIS_PARENT_HASH, keypair=keypair
)
```

## Latency

| Operation | Cost |
|-----------|------|
| `sign_event` / seal (no I/O), GIL released | ~11-15 us |
| `Session.record` (seal + append + fsync) | ~3.8 ms, dominated by fsync |

`Session.record` fsyncs for durability, the same as the sidecar. On an async
backend, run it off the event loop:

```python
signed = await asyncio.to_thread(session.record, event)
```

## Failure modes

All failures raise; nothing returns an error sentinel.

| Exception | When |
|-----------|------|
| `provedex.KeyLoadError` | bad key file (length, hex, missing on `load`) |
| `provedex.SigningError` | seal/hash failure, bad event shape in `from_dict` |
| `provedex.LedgerError` | ledger read/write failure |
| `provedex.ChainError` | malformed verification input |

`verify_chain` / `verify_file` do NOT raise on a broken chain; they return
`ChainReport(ok=False, broken_at=<seq>, reason=...)`. A broken chain is data.

## Byte-compat

There is one canonical-JSON encoder in the whole system: the Rust one. This
binding calls it directly, so the bytes it signs are identical to the sidecar
and the CLI. The repo's `tests/compat/vectors/` golden suite and the
cross-verify tests assert it.

## Verifying offline

Anyone with the public key can verify the ledger with no involvement from you:

```bash
provedex verify --ledger ./ledger.ndjson
```

## License

Apache-2.0.
````

- [ ] **Step 4: Write `RELEASING.md`**

````markdown
# Releasing `provedex` (native Python binding)

Wheels are built by maturin and published to PyPI as `provedex`. Version tracks
`provedex-core` semver.

## One-time

```bash
pip install --upgrade maturin twine
```

A PyPI API token with upload scope for the `provedex` project.

## Build the wheels + sdist

The release is normally cut by the `release-python.yml` GitHub workflow on a
`python-v*` tag (manylinux x86_64, linux aarch64, macOS arm64, all abi3-py311,
plus the sdist). To build locally for a smoke check:

```bash
cd bindings/python/provedex
maturin build --release
ls target/wheels/
```

## Publish

Download the workflow's `dist/` artifact (or use the local `target/wheels/`),
then:

```bash
twine check dist/*
twine upload dist/*
```

## Verify

```bash
python -m venv /tmp/verify-provedex
/tmp/verify-provedex/bin/pip install provedex
/tmp/verify-provedex/bin/python -c "import provedex; print(provedex.__version__)"
```

## Tag

```bash
git tag python-v0.1.0
git push origin python-v0.1.0
```
````

- [ ] **Step 5: Lint the example with ruff**

```bash
cd bindings/python/provedex && .venv/bin/ruff check examples
```

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add bindings/python/provedex/README.md bindings/python/provedex/RELEASING.md \
  bindings/python/provedex/examples/basic.py
git commit -m "docs(bindings/python): README, RELEASING, basic example

Quickstart, event table, latency budget, failure modes, byte-compat note,
offline-verify. Runnable basic.py end-to-end. Refs #6."
```

---

### Task 16: PyPI release workflow

**Files:**
- Create: `.github/workflows/release-python.yml`

- [ ] **Step 1: Write `.github/workflows/release-python.yml`**

```yaml
name: release-python

on:
  push:
    tags:
      - "python-v*"
  workflow_dispatch:

permissions:
  contents: read

jobs:
  build-wheels:
    name: build wheels (${{ matrix.target }})
    runs-on: ${{ matrix.runner }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - runner: ubuntu-latest
            target: x86_64
            manylinux: "2014"
          - runner: ubuntu-latest
            target: aarch64
            manylinux: "2014"
          - runner: macos-14
            target: aarch64
    steps:
      - uses: actions/checkout@v4

      - name: setup python 3.11
        uses: actions/setup-python@v5
        with:
          python-version: "3.11"

      - name: build wheels
        uses: PyO3/maturin-action@v1
        with:
          working-directory: bindings/python/provedex
          target: ${{ matrix.target }}
          manylinux: ${{ matrix.manylinux }}
          args: --release --out dist

      - name: upload wheel artifacts
        uses: actions/upload-artifact@v4
        with:
          name: wheels-${{ matrix.runner }}-${{ matrix.target }}
          path: bindings/python/provedex/dist/*.whl

  build-sdist:
    name: build sdist
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: build sdist
        uses: PyO3/maturin-action@v1
        with:
          working-directory: bindings/python/provedex
          command: sdist
          args: --out dist

      - name: upload sdist artifact
        uses: actions/upload-artifact@v4
        with:
          name: sdist
          path: bindings/python/provedex/dist/*.tar.gz
```

Note: this workflow builds and uploads artifacts only. The founder publishes via
`twine upload` from the downloaded `dist/` (per `RELEASING.md`), matching the
manual-publish discipline used for the pipecat and langchain bindings. A PyPI
auto-publish step (trusted publishing) is a later hardening, not v1.

- [ ] **Step 2: Validate the YAML**

```bash
cd /Users/adi/Desktop/provedex
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-python.yml')); print('yaml ok')"
```

Expected: `yaml ok`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release-python.yml
git commit -m "ci: release-python workflow builds wheels + sdist

maturin-action matrix: manylinux2014 x86_64 + aarch64, macOS arm64, abi3.
Tag-gated on python-v*. Uploads artifacts; founder publishes via twine.
Refs #6."
```

---

## Final verification (after all tasks)

- [ ] **Run the full Rust gate from the repo root** (core change in Task 2 + the example in Task 11 must not regress anything):

```bash
cd /Users/adi/Desktop/provedex
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Expected: all green. (The binding crate is outside the workspace, so `--workspace` does not build it; that is intentional. Build it separately below.)

- [ ] **Run the full binding gate**:

```bash
cd bindings/python/provedex
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
.venv/bin/maturin develop
.venv/bin/ruff check python tests examples
.venv/bin/mypy python tests
.venv/bin/pytest -v
```

Expected: all green; every pytest test passes including integration.

- [ ] **Confirm the example runs**:

```bash
.venv/bin/python examples/basic.py
```

Expected: `verified: ok=True events=3`.

- [ ] **Dispatch a final whole-implementation code review** (subagent-driven-development's final reviewer), then use `superpowers:finishing-a-development-branch` to open the PR.
