# Architecture Decision Records

Each significant architectural choice gets a numbered, dated, immutable record.

## Format

`NNNN-kebab-title.md` where NNNN is monotonically increasing. Once an ADR is merged, never renumber and never silently rewrite. To change a decision, write a new ADR that supersedes the old one.

Template:

```
# NNNN. Title

Date: YYYY-MM-DD
Status: proposed | accepted | superseded by NNNN

## Context
What is the problem and what are the constraints.

## Decision
What we chose and why this option over the alternatives.

## Consequences
What this makes easy, hard, or impossible. Future obligations.
```

## Index

Accepted:

- [0001. Canonical JSON format](0001-canonical-json-format.md) - why we rolled our own vs RFC 8785 / JCS.
- [0002. Hash chain shape](0002-hash-chain-shape.md) - parent_hash + self_hash + signature, why not Merkle tree.
- [0003. NDJSON over a binary format](0003-ndjson-over-binary-format.md) - operator inspectability with `jq`, `tail`, `cat`.
- [0004. Sidecar binary as the default integration](0004-sidecar-as-default-integration.md) - one Rust binary vs N native FFI bindings.
- [0005. Open-core licensing](0005-open-core-licensing.md) - Apache-2.0 trust primitive, proprietary commercial operations layer.

Proposed:

- [0006. Post-quantum signature migration path](0006-post-quantum-migration.md) - Ed25519 today, hybrid mode behind a flag, ML-DSA-65 on a published roadmap.
