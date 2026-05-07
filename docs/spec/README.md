# Specifications

Byte-level normative documents that any Provedex client implementation must follow.

Planned:

- `event-schema-v1.md` - the `AgentEvent` enum, JSON tagging, payload shape per variant.
- `canonical-json.md` - the deterministic JSON encoding rules used for hashing and signing.
- `signature-scheme.md` - Ed25519 over SHA-256 of canonical-JSON, hex encoding, key format.
- `ledger-format.md` - NDJSON file layout, fsync semantics, parent-hash chaining.

A client (Python, TypeScript, Java, Go) that follows these specs must produce signed events that are byte-identical to the Rust reference implementation in `crates/provedex-core/`.
