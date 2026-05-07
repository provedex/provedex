# 0003. NDJSON over binary formats for the on-disk ledger

Date: 2026-05-07
Status: accepted

## Context

The on-disk ledger needs a format that:

- Appends fast (signing happens at agent runtime, not in batch).
- Survives crashes mid-write (durable, fsynced, not a DB transaction-in-flight).
- Stays inspectable by an operator with `cat`, `grep`, and `jq`.
- Is reproducible by hand from the canonical-JSON spec.
- Integrates with existing log shipping (Splunk, Datadog, Elastic) without a transcoding step.
- Has no external dependency at the storage layer.

Candidate formats considered:

1. NDJSON - one JSON object per line, append-only file. Plain text. Standard tool support. Verifiable with `jq`.
2. Protocol Buffers - compact, fast, schema-versioned. Adds a code-gen step, hides the wire format from operators, requires the schema to inspect the file.
3. CBOR / MessagePack - binary canonical encodings. Faster than NDJSON, smaller on disk. Same operator-readability problem.
4. SQLite - transactional, queryable. Adds a runtime dependency to the audit primitive itself, which conflicts with the goal of keeping `provedex-core` dependency-light.
5. Append-only B-tree / LSM - performance-oriented, wrong layer; SIEM / aggregator concern, not primitive.

## Decision

The local ledger is NDJSON: one canonically-serialized `SignedEvent` per line, appended via `O_APPEND` writes followed by `fsync_data` for durability.

- Default path: `~/.provedex/ledger.ndjson`.
- Each line is a complete signed event. Independent decode, no cross-line dependency for parsing.
- Writes are serialized through a process-wide mutex in `Ledger::append`.
- Reads are open-and-iterate; we do not maintain an index, because the file is human-tail-able and small enough that linear scan dominates verification cost (signature verification is the bottleneck, not parsing).

Operators can inspect with:

```
tail -f ~/.provedex/ledger.ndjson | jq .
provedex verify
provedex replay
```

## Consequences

- Disk size: NDJSON is 30-60 percent larger than CBOR for the same data. Acceptable at expected event volumes (low thousands per session for v1; the hosted aggregator will compress at rest).
- Append performance: bottlenecked by `fsync_data`, not by serialization. Switching to a binary format would not move the needle.
- Forward compatibility: adding a field to a `SignedEvent` payload is backward-compatible to readers (serde ignores unknown fields if configured; we use that mode for ExportBundle but lock the hashed surface in `SignedEvent::compute_self_hash`).
- Backward compatibility: removing a field is breaking. Bumping the canonical-JSON spec or the event schema requires the steps in ADR 0001.
- Integration with SIEM: Splunk, Datadog, and Elastic ingest NDJSON natively. No transcoding needed when the SIEM forwarder lands in `crates/provedex-siem/`.
- If a future use case requires sub-millisecond append latency at high event rates (e.g. high-frequency trading agents emitting 10k events/sec), we will revisit and likely add a parallel binary format guarded behind a Cargo feature, with NDJSON staying the canonical archival format.
- Tamper-test (demo-only) operates on the NDJSON file directly via line-level rewrites; this would be considerably more involved against a binary format and is one of the small reasons NDJSON wins for a demoable product.
