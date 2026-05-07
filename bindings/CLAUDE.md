# bindings/ - FFI wrappers around provedex-core

Customer apps run in Python, TypeScript, Java, Go. They cannot link to a Rust crate directly. Bindings give them a native package that wraps `provedex-core`.

## Today

- `python/` - PyO3 wrapper. Publishes `provedex` to PyPI. Not built yet.
- `node/` - napi-rs wrapper. Publishes `@provedex/core` to npm. Not built yet.

## The byte-compat rule (non-negotiable)

Every binding MUST produce signed events that are byte-identical to the Rust reference for the same input. A signed event written by the Python binding must verify with `provedex-cli verify` on a Rust ledger and vice versa.

The `tests/compat/` suite (when added) holds golden inputs and expected canonical-JSON output bytes. New bindings must pass it before publishing.

## What bindings are allowed to expose

A binding's surface must be a subset of `provedex-core`'s public API, plus idiomatic-language ergonomic wrappers. No binding-only feature, no extra event variant, no different signature scheme.

Allowed:
- `SigningKeypair.generate() / .load(path) / .save(path) / .pubkey_hex`
- `sign_event(payload, parent_hash, keypair) -> SignedEvent`
- `verify_chain(events) -> ChainReport`
- `Ledger.open(path) / .append(event) / .read_all() / .verify()`
- `canonical_json(value) -> bytes`

Not allowed in a binding:
- New event variants.
- Non-Ed25519 signatures.
- Non-canonical JSON encodings.
- I/O abstractions other than `Ledger` (no Kafka emitter, no DB writer; those are aggregator concerns).

## API style

- Pythonic API in Python (`provedex.SigningKeypair.generate()`, exceptions on failure).
- TypeScript API on Node (`signEvent(payload, parentHash, keypair)`, throws on failure).
- Type stubs / .d.ts must ship with the package.

## Build + publish

- Python: `maturin`. Wheels for cpython 3.11+ on macOS arm64, macOS x86_64, linux x86_64, linux arm64.
- Node: napi-rs. Native binaries for macOS arm64, macOS x86_64, linux x86_64, linux arm64, windows x86_64.
- Version each binding to match the underlying `provedex-core` semver. A binding pinned to `provedex-core 0.2.x` cannot publish if core has moved on without a binding update.

## Forbidden

- No business logic in a binding. Pass-through to core only.
- No silently mutated event payloads.
- No deviation from canonical JSON.
- No partial implementation. Either the binding passes the full compat suite or it does not ship.
