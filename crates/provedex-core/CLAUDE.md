# crates/provedex-core - cryptographic primitives

The core library. Public API. Will be published to crates.io. Bindings (Python, Node) wrap this.

## Modules

| Module | Purpose | Public API |
|--------|---------|------------|
| `event` | `AgentEvent` enum (SessionStarted, UtteranceCaptured, ToolCalled, ...). Tagged JSON via serde. | `AgentEvent` |
| `signed` | `SignedEvent` struct, canonical JSON encoder, `compute_self_hash`, seal + verify. | `SignedEvent`, `canonical_json`, `compute_self_hash`, `GENESIS_PARENT_HASH` |
| `keys` | Ed25519 keypair generation, persistence, signature verification. Default paths under `~/.provedex/`. | `SigningKeypair`, `verify_signature`, `default_*_path` |
| `ledger` | Append-only NDJSON ledger. fsync on append. `read_all`, `last`, `count`, `verify`. | `Ledger`, `read_file` |
| `chain` | Walk the ledger, validate hashes + signatures + parent linkage. | `verify_chain`, `ChainReport`, `ChainStatus` |
| `export` | `ExportBundle` for the regulator-export packet. | `ExportBundle` |

## Invariants (do not break these without an ADR)

1. **Canonical JSON is deterministic and forever.** Object keys sorted alphabetically. No whitespace. Fixed escape rules. Numbers in their stdlib `to_string()` form. Any change here invalidates every signature ever produced. Schema bump must increment `schema_version` in `ExportBundle` and add a versioned canonical-json spec.
2. **`self_hash` covers `(seq, timestamp_nanos, event, parent_hash)` and only those fields.** Order in the hashed map does not matter (canonical JSON sorts). Adding a field to `SignedEvent` does not extend the hash unless added inside `compute_self_hash` AND a schema bump is published.
3. **Genesis parent hash is exactly 64 zeros (hex).** Not a hash of nothing. Not a typed sentinel. The string `"0".repeat(64)`.
4. **Signature is over the raw 32-byte hash, not over the hex.** Verification recomputes the hash, then verifies against the embedded `signer_pubkey`.
5. **Ledger is append-only.** `Ledger::append` is the only sanctioned write path. Server's `AppState::seal_and_append` is the only sanctioned event emitter for live runs. Never edit a line in place outside the demo-only tamper-test.
6. **Sequence numbers are dense and start at 0.** `verify_chain` rejects any gap or out-of-order seq.

## Adding a new event variant

1. Add the variant to `AgentEvent` in `src/event.rs` with a `payload` struct.
2. Add a roundtrip test in `event::tests`.
3. Update `provedex-cli/src/commands/replay.rs::describe` to format it.
4. If this changes the canonical-JSON shape on the wire, add an ADR, bump `ExportBundle::schema_version`, and write a `docs/spec/event-schema-vN.md`.

## Public API doc requirement

Every public item: `///` doc explaining purpose, invariants, panics. Public methods on public structs need at least one runnable doctest (use `tempfile` from dev-deps). Doctests count toward CI green-tests.

## Dependencies

Workspace deps only. Crypto crates (`ed25519-dalek`, `sha2`) are audit-relevant; do not bump major versions without an ADR.

## Forbidden

- No I/O in `event`, `signed`, `chain`. Pure compute.
- No `unsafe` without an ADR.
- No `unwrap` outside tests.
- Do not add a Merkle tree, Bloom filter, or transparency-log primitive here. Those go in a future `provedex-rekor` crate.

## Tests

- `signed::tests` covers canonical-JSON determinism + seal/verify roundtrip + tamper detection.
- `chain::tests` covers 100-event chain + signature tamper + missing-event detection.
- `keys::tests` covers sign/verify + load/save roundtrip.
- `ledger::tests` covers append + reopen + last + empty.
- `export::tests` covers chain-report inclusion.
- `examples/basic_signing.rs` is also runnable: `cargo run -p provedex-core --example basic_signing`.

Keep test count visible. Adding behavior without adding a test fails review.
