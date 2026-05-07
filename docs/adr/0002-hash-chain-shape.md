# 0002. Hash chain shape: parent_hash + self_hash + Ed25519 signature

Date: 2026-05-07
Status: accepted

## Context

Provedex needs every event in the ledger to be tamper-evident. A regulator or auditor must be able to detect, with cryptographic certainty:

1. Whether any event has been edited after the fact.
2. Whether any event has been deleted from the middle of the ledger.
3. Whether any event has been inserted out of order.
4. Whether the events were emitted by the holder of a specific signing key.

We considered several structures:

- A flat list of signed events with no inter-event link. Detects edits to a single event but does not detect deletion or insertion.
- A Merkle tree over the entire session, with the root signed at the end. Detects everything but requires the entire session before a verifier can check anything; bad fit for streaming demos and live regulator dashboards.
- A linked hash chain (each event references the previous event's hash) plus per-event signatures. Detects edit, deletion, insertion, and reordering, and supports incremental verification of partial logs.
- A blockchain-style Merkle-DAG. Overkill for a single-writer ledger; introduces consensus problems that do not exist in our use case.

## Decision

Each `SignedEvent` carries five fields used for integrity:

- `seq: u64` - dense, monotonic per session, starts at 0.
- `timestamp_nanos: u64` - Unix nanoseconds at emission.
- `parent_hash: String` - hex SHA-256 of the previous event's `self_hash`. For the genesis event (seq 0), parent_hash is exactly 64 zeros.
- `self_hash: String` - hex SHA-256 of canonical-JSON over `{seq, timestamp_nanos, event, parent_hash}` (and only those fields).
- `signature: String` - hex Ed25519 signature over the raw 32-byte `self_hash` digest. Signed with the keypair stored in `~/.provedex/keys/ed25519.key`.

Verification walks the ledger and checks:

1. `seq` is dense and monotonically increasing from 0.
2. `parent_hash` of event N+1 equals `self_hash` of event N.
3. `self_hash` is the SHA-256 of canonical-JSON over the four hashed fields.
4. `signature` validates against `signer_pubkey` for the bytes of `self_hash`.

If any check fails, verification stops at the first broken event with a clear `broken_at_seq` report.

## Consequences

- Detection guarantees:
  - Edit to any event: `self_hash` recomputation fails OR the next event's `parent_hash` no longer matches.
  - Delete an event: `seq` becomes non-dense; `parent_hash` of the next event fails to match the new previous hash.
  - Insert an event: `seq` collision detected by `verify_chain`.
  - Reorder events: `seq` decreases or `parent_hash` mismatches.
- Verification is incremental. A regulator can verify the first N events without seeing the rest. Useful for streaming export and partial-record subpoenas.
- Schema is locked. Adding a field to `SignedEvent` outside `compute_self_hash`'s input set does not extend the integrity guarantee. Adding a field inside it requires a schema version bump.
- We do not anchor to a transparency log (Sigstore Rekor, certificate transparency) in v1. That would defend against the keyholder themselves rewriting history. v1 trusts the keyholder; v3 will add transparency-log anchoring.
- We do not currently support multi-writer / multi-signer ledgers. Each ledger has exactly one signing key and one writer. Multi-tenant aggregation happens at a higher layer.
- Blockchain consensus is explicitly not required. The single-writer model is correct because the buyer (the AI agent's operator) holds the key and is the only signer.

## References

- Canonical JSON: `docs/adr/0001-canonical-json-format.md`.
- Genesis sentinel: `provedex-core::signed::GENESIS_PARENT_HASH = "0".repeat(64)`.
- Verification logic: `provedex-core::chain::verify_chain`.
- Ed25519 selection rationale: implicit; future ADR if we ever consider rotating to a different signature scheme.
