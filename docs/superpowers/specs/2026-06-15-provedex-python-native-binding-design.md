# provedex (native Python binding) - design

Date: 2026-06-15
Status: approved, ready for planning
Tracking issue: #6 (scaffold pyo3 wrapper). This spec covers the scaffold AND the
full v1 surface in one pass, because a design partner needs it in production now,
not a hello-world follow-up.

## Goal

Ship `provedex`, a native Python SDK that signs Provedex events in-process via
PyO3 around `provedex-core`. A design partner runs a Python backend on AWS
(Linux x86_64) and needs in-process signing with the same byte output as the
Rust reference. This is the v0.2 roadmap milestone: native FFI binding, opt-in
fast-path, byte-compatible with the sidecar and the Rust core.

## Why native, why now

The default integration is the localhost HTTP sidecar (ADR 0004, 1-2 ms per
sign). The native binding is the opt-in fast-path for customers who want
sub-millisecond, in-process signing with no extra process to deploy. The design
partner asked for exactly this: a `pip install provedex` SDK, same functionality
as the core, simple setup, low latency.

Measured budget (from the repo benchmarks):

| Path | Per signed event |
|------|------------------|
| PyO3 native seal (sign + hash, no I/O), GIL released | ~11-15 us |
| PyO3 FFI boundary crossing | ~50-200 ns |
| Pure-Python reimplementation (pynacl + hashlib + Python canonical-JSON) | ~20-50 us |
| Sidecar HTTP roundtrip | 1000-2000 us |
| `Session.record` (seal + append + fsync) | ~3.8 ms, dominated by fsync |

PyO3 wins on latency and, more importantly, keeps a single canonical-JSON
encoder. A pure-Python reimplementation would introduce a second encoder that
must stay byte-identical to Rust forever, including the non-ASCII-not-escaped
rule and the stdlib number formatting. For a product whose value is
byte-identical evidence, a second drifting encoder is the wrong risk. Decision:
PyO3.

## Architecture

A standalone Rust crate `provedex-python` at `bindings/python/provedex/`, built
with maturin, published to PyPI as `provedex` (name confirmed free, 404 on
2026-06-15).

- Path dependency on `provedex-core` (`../../../crates/provedex-core`). NOT a
  member of the root cargo workspace; the workspace lists only `crates/*` and
  the crates rules forbid FFI inside a `crates/` member. The binding crate has
  its own `Cargo.toml`.
- Thin pass-through. Zero cryptographic logic in the binding. Every signing,
  hashing, and verification call lands in `provedex-core`. There is exactly one
  canonical-JSON encoder in the whole system (the Rust one), so byte-compat is
  structural, not a property we have to test into existence.
- The PyO3 layer releases the GIL around `seal` and `append` so a signing call
  never blocks other Python threads.

### File structure

maturin mixed Rust/Python layout (`python-source = "python"` in pyproject), so
the Python package files and the Rust sources sit side by side:

```
bindings/python/provedex/
  Cargo.toml                     # provedex-python crate, cdylib, pyo3 + provedex-core path dep
  pyproject.toml                 # maturin backend, python-source = "python", name = "provedex"
  src/lib.rs                     # #[pymodule] root: registers classes, functions, submodule
  src/keypair.rs                 # SigningKeypair pyclass wrapping core SigningKeypair
  src/events.rs                  # events submodule: 7 typed factory fns -> AgentEvent pyclass
  src/session.rs                 # Session pyclass wrapping core LedgerSession
  src/signed.rs                  # SignedEvent pyclass (read-only view) + sign_event fn
  src/verify.rs                  # verify_chain, verify_file, ChainReport pyclass
  src/canonical.rs               # canonical_json(obj) -> bytes
  src/errors.rs                  # ProvedexError base + subclasses, From<CoreError> mapping
  python/provedex/__init__.pyi   # type stubs for the compiled module
  python/provedex/py.typed       # PEP 561 marker
  tests/                         # pytest unit + integration suite
  examples/basic.py              # minimal end-to-end
  README.md                      # quickstart, API, latency, byte-compat note
  RELEASING.md                   # maturin + twine recipe
```

The `AgentEvent` and `SignedEvent` Python objects are opaque handles backed by
the Rust types. Python never constructs the tagged JSON by hand; the typed
factories own that, which keeps the 7 variants exact and forbids binding-only
variants.

## Public API

```python
import provedex

# --- keypair ---
kp = provedex.SigningKeypair.generate()
kp = provedex.SigningKeypair.load("~/.provedex/keys/ed25519.key")
kp = provedex.SigningKeypair.load_or_create(path)
kp.save(path)
kp.pubkey_hex                      # property -> str (64 hex chars)

# --- events: one typed factory per core variant, no binding-only variants ---
e = provedex.events.session_started(agent_id=, model_id=, session_id=)
e = provedex.events.utterance_captured(audio_sha256=, transcript=, lang=, duration_ms=)
e = provedex.events.tool_called(tool_name=, args_sha256=, args_redacted={...})
e = provedex.events.tool_returned(tool_name=, result_sha256=, latency_ms=, success=)
e = provedex.events.model_invoked(model_id=, prompt_sha256=, response_sha256=,
                                  prompt_tokens=, response_tokens=)
e = provedex.events.utterance_spoken(text_sha256=, text=, audio_sha256=)
e = provedex.events.session_ended(reason=, summary_sha256=)

# --- primary path: Session wraps core LedgerSession ---
s = provedex.Session.open(keypair=kp, ledger_path="./ledger.ndjson", session_id="s1")
signed = s.record(e)               # seal_and_append: auto seq + parent chain + fsync
s.session_id                       # property -> str
s.pubkey_hex                       # property -> str

# --- low-level escape hatches ---
signed = provedex.sign_event(event=e, seq=0, parent_hash=provedex.GENESIS_PARENT_HASH,
                             keypair=kp)
report = provedex.verify_chain(events)     # events: list[SignedEvent]
report = provedex.verify_file(ledger_path) # read NDJSON then verify
raw    = provedex.canonical_json(obj)      # obj: JSON-able -> bytes

# --- SignedEvent: read-only view ---
signed.seq                # int
signed.timestamp_nanos    # int
signed.event              # dict (tagged {type, payload})
signed.parent_hash        # str
signed.self_hash          # str
signed.signature          # str
signed.signer_pubkey      # str
signed.to_json()          # str (NDJSON line, identical bytes to Rust ledger line)

# --- ChainReport ---
report.ok                 # bool
report.event_count        # int
report.broken_at          # int | None (seq of first break, None if ok)
report.reason             # str | None
```

`provedex.GENESIS_PARENT_HASH` is exposed as a module constant (the 64-zero hex
string) so callers using the low-level `sign_event` do not hardcode it.

### Design decisions inside the surface

- **`Session` is the primary API, not `sign_event`.** Real apps need automatic
  seq allocation and parent-hash chaining; `LedgerSession` already owns that in
  core. `sign_event` stays as the advanced escape hatch for callers managing the
  chain themselves.
- **Typed event factories, not raw dicts.** `provedex.events.session_started(...)`
  is discoverable, type-checked by the stubs, and keeps the variant set locked to
  the 7 core variants. A raw-dict path would let a caller invent a variant that
  the Rust core would reject at deserialization.
- **Sync-only in v1.** `Session.record` does an fsync on append (~3.8 ms, the
  same durability the sidecar provides). The seal itself is ~11 us. Async Python
  backends wrap the call in `asyncio.to_thread`, documented in the README. No
  async API in v1 (YAGNI); the fsync cost is inherent to durability and identical
  to the sidecar, and adding an async surface doubles the API for no latency win
  on the crypto path.
- **`session_id` is explicit, never auto-derived.** The demo-voice integration
  learned that a process-scoped `session_id` surprises an integrator who expects
  one session per conversation. `Session.open(session_id=...)` makes the caller
  own that boundary.

## Errors

Rust core errors map to a Python exception hierarchy. All raise; none return
error sentinels.

```
provedex.ProvedexError            # base
  provedex.KeyLoadError           # KeyError: bad length, missing file, bad hex
  provedex.SigningError           # SignedError: hash mismatch, json, hex
  provedex.LedgerError            # LedgerError: io, parse, append
  provedex.ChainError             # raised by verify_* on malformed input only;
                                  # a broken-but-parseable chain returns a
                                  # ChainReport with ok=False, it does not raise
```

`verify_chain` and `verify_file` distinguish "the chain is broken" (a normal
result: `ChainReport(ok=False, broken_at=...)`) from "the input could not be
parsed at all" (`ChainError`). A broken chain is data, not an exception.

## Byte-compat and testing

This binding is the first second-language signer, so it unblocks the
`tests/compat/` golden-vector suite (#5).

### Golden vectors (source of truth: Rust)

A `provedex-core` example, `emit_compat_vectors`, run via
`cargo run -p provedex-core --example emit_compat_vectors`, emits for a fixed set
of event inputs the canonical-JSON bytes and the resulting `self_hash` hex to
`tests/compat/vectors/*.json`. The inputs use fixed `seq` and `timestamp_nanos`
values (not `now_nanos`), so the generator is deterministic and the goldens are
stable across runs. JSON files are the language-neutral source of truth. The
Python suite reads them and asserts:

- `provedex.canonical_json(input) == expected_bytes`
- the `self_hash` computed by the binding for `(seq, timestamp_nanos, event,
  parent_hash)` equals the golden hex.

Vectors must exercise the encoder edge cases: sorted keys, nested arrays,
control-character escapes, non-ASCII passed through as raw UTF-8 (not `\u`
escaped), and integer formatting.

### Cross-verification (the trust-critical test)

1. Python `Session` writes a ledger, then `provedex verify` (the Rust CLI)
   exits 0 on it.
2. A ledger written by the Rust reference (CLI or a fixture) verifies via
   Python `provedex.verify_file(path)` with `ok=True`.

These two prove a Python-signed receipt and a Rust-signed receipt are mutually
verifiable, which is the entire promise of the byte-compat rule.

### Unit and smoke tests

pytest suite over the Python surface: keypair generate/save/load roundtrip, each
event factory, `Session.record` seq+parent chaining, `sign_event` low-level path,
`verify_chain` on good and tampered input, error mapping (each exception type
raised by the right failure), `canonical_json` against the golden vectors.

The issue #6 smoke test (generate keypair, sign one event, verify it) is a
subset and is included.

## Build and release

- **Build backend**: maturin. `pyproject.toml` declares `requires-python =
  ">=3.11"`.
- **Wheel matrix**: manylinux2014 x86_64 (must-have, the design partner's AWS
  target), plus linux aarch64 and macOS arm64 for developer experience. macOS
  x86_64 deferred unless asked.
- **CI**: a `bindings-python-native` job that runs `maturin develop`, the pytest
  suite, and the cross-verify test against a freshly built `provedex` CLI. ruff
  + mypy on the Python sources and stubs.
- **Release workflow**: maturin-action builds the wheel matrix + sdist on tag
  `python-v0.1.0`, twine-uploads to PyPI as `provedex` 0.1.0 (matches
  `provedex-core` semver). Founder runs the upload via `RELEASING.md`, same as
  the pipecat and langchain bindings.
- **Stubs**: `provedex/__init__.pyi` ships in the wheel; `py.typed` marker
  present (PEP 561).

## Out of scope (YAGNI for v1)

- Async API surface. Documented `asyncio.to_thread` wrap is enough.
- The Node (napi-rs) binding. Tracked separately at #7.
- Batched-flush ledger for higher sign throughput. That is a `provedex-core`
  feature, not a binding concern.
- Switching the existing `provedex-pipecat` and `provedex-langchain` sidecar
  adapters to the native path. Separate follow-up, driven by a measured need.
- macOS x86_64 wheels. Add when an Intel-Mac integrator asks.
- Publishing `provedex-core` to crates.io. Still deferred (the binding path-deps
  the local crate).

## Acceptance criteria

- `bindings/python/provedex/` exists with `Cargo.toml`, `pyproject.toml`,
  `src/lib.rs`, and the module split above.
- `maturin develop` builds the extension locally; `import provedex` works.
- Full surface implemented: `SigningKeypair`, `events.*` (7 factories),
  `Session`, `sign_event`, `verify_chain`, `verify_file`, `canonical_json`,
  `SignedEvent`, `ChainReport`, `GENESIS_PARENT_HASH`, the error hierarchy.
- `tests/compat/vectors/` exists with Rust-generated goldens; Python asserts
  byte-identical canonical-JSON and self_hash.
- Cross-verify passes both directions (Python-signed verifies on Rust CLI;
  Rust-signed verifies in Python).
- `provedex.pyi` stubs + `py.typed` ship; mypy clean.
- CI job green on linux x86_64.
- Does NOT auto-publish to PyPI; the release workflow is tag-gated and the upload
  is a manual founder step.
