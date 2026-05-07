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

Planned:

- 0001. Canonical JSON format - why we rolled our own vs RFC 8785 / JCS.
- 0002. Hash chain shape - parent_hash + self_hash + signature, why not Merkle tree.
- 0003. Per-session vs per-agent keypair scope.
- 0004. NDJSON over a binary format.
