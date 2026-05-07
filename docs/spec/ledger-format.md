# Ledger format (v1)

Status: normative.

This document specifies the on-disk representation of a Provedex ledger: the file layout, append semantics, read semantics, and crash-recovery guarantees. Operators integrating with the ledger from non-Rust environments (log shippers, SIEM forwarders, custom verifiers) implement against this spec.

References:

- ADR 0003 (NDJSON over binary format).
- `docs/spec/event-schema-v1.md` (per-line content).
- `docs/spec/signature-scheme.md` (per-line signature semantics).

## Scope

This spec covers:

1. File path conventions and how to override them.
2. Line format (NDJSON: one SignedEvent per line).
3. Append semantics: what writes are atomic, what is fsynced, when.
4. Read semantics: how to walk a ledger correctly.
5. Crash-recovery behavior under partial writes.

This spec does NOT cover archival, retention, multi-file rotation, or transmission. Those concerns live in the hosted aggregator and SIEM forwarders, which sit on top of the local ledger and consume it via these read semantics.

## File path

- Default path: `~/.provedex/ledger.ndjson`.
- Override via env: `PROVEDEX_LEDGER=<path>`.
- The directory is created if missing (recursive `mkdir -p`).
- The file is created on first append in `O_CREAT | O_APPEND | O_WRONLY` mode.
- File permissions: standard umask. There is no spec requirement for ledger file permissions because the file content is signed; tampering is detectable, so confidentiality at rest is the operator's choice (host-level encryption, SELinux, etc.).

## Line format

The ledger is NDJSON: each line is exactly one JSON object terminated by a single LF (`\n`, 0x0A) byte. No CRLF. No trailing whitespace before the LF.

Each line decodes to a `SignedEvent` per `event-schema-v1.md` plus the four signed fields `self_hash`, `signature`, `signer_pubkey`. The encoding on the wire does NOT have to be canonical-JSON; the Rust reference uses ordinary `serde_json::to_vec` output (compact, no whitespace, no key sort). Verification recomputes the canonical-JSON of the four hashed fields when checking `self_hash`, so the wire encoding is free to differ in key order or whitespace from the canonical encoding.

A binding's writer SHOULD use ordinary stdlib JSON output (compact form) for the wire. A binding's reader MUST accept any valid JSON object on each line, regardless of key order, and recompute canonical-JSON for hash verification.

Empty lines (a line containing only `\n`) are tolerated by the reader and skipped. The writer never emits empty lines.

The file MAY end without a trailing LF on the last line. Implementations that read by `BufRead::lines()` handle this naturally. Writers SHOULD emit a trailing LF after every event so concurrent appenders never produce a malformed concatenation.

## Append semantics

The Rust reference (`provedex_core::Ledger::append`) does:

1. Acquire the per-`Ledger` writer mutex.
2. Serialize the SignedEvent to bytes via `serde_json::to_vec`.
3. Write the bytes followed by a single `\n` to the file (one `write_all`).
4. Call `fsync_data` (Unix: `fdatasync`; macOS: `F_FULLFSYNC`-equivalent).
5. Release the mutex.

A binding's append implementation MUST satisfy these guarantees:

- **Single-line atomicity.** A reader observing the file mid-append must see either the complete line plus `\n` or none of it. On POSIX, this is provided by combining `O_APPEND` + a single `write_all` of size less than `PIPE_BUF` for short events, and explicit serialization through a writer mutex for any size.
- **Durability.** After `append` returns, the line is on stable storage (fsynced). A power loss at this point cannot lose a line that `append` claimed to have written.
- **Per-process serialization.** Two threads in the same process must serialize through the same mutex. This is the only path that preserves seq monotonicity, since the seq counter and parent_hash mutex live next to the writer mutex in `LedgerSession`.

Cross-process appending to the same ledger file is NOT supported. The seq counter and parent_hash mutex live in process memory; two processes appending concurrently will produce out-of-order seq numbers and broken parent links. Operators that need horizontal scale write to separate ledger files per process and aggregate downstream.

## Read semantics

A reader walks the file line-by-line. The Rust reference uses `BufReader::lines()` and is the authoritative implementation; bindings should match its behavior.

For each line:

1. Skip if the line is empty (whitespace-only).
2. Decode the line as JSON. If decoding fails, the read fails (return an error to the caller). Do NOT skip and continue.
3. Decode the JSON into a `SignedEvent`. If the structure does not match the schema, the read fails.
4. Append to the in-memory event list.

After reading all lines, the caller can:

- Run `verify_chain` to validate the cryptographic chain (per `signature-scheme.md`).
- Iterate the events to reconstruct the session.
- Filter or transform events for downstream consumption (SIEM forwarding, regulator export).

The Rust `read_all` reads the entire file into memory. This is acceptable up to a few hundred thousand events. For larger ledgers, a streaming reader can yield events one at a time; the chain verification can be incremental too. A streaming API is not in v1; bindings can implement their own.

## Crash-recovery behavior

If a process crashes mid-append, the ledger may end with a partial line (no trailing `\n`, no closing `}`, or truncated mid-string). The read semantics treat this as a corrupt record: the JSON decode of the partial line fails, and the read returns an error.

The Rust reference exposes this through `read_all` returning `Err(LedgerError::Json)`. Upstream tools surface it as a chain-broken state.

We do NOT silently truncate at the last good line. A partial line is a signal that an operator should investigate; silent truncation could hide attacker activity (where an adversary deletes the last few events to retract a recorded action).

If an operator wants to recover from a partial-line crash:

1. Inspect the file manually with `tail` to confirm the corruption is at EOF only.
2. Truncate the file to the last `\n` boundary using a separate, explicit tool (e.g. `provedex-cli ledger-recover`, planned).
3. Re-run `provedex verify` to confirm the chain is valid up to the truncation point.

This recovery procedure does not exist as tooling in v1; it is a manual operator step. Future tooling will land as a follow-up.

## Concurrent readers

Any number of reader processes can open the ledger read-only while a single writer process appends. POSIX file semantics guarantee that a reader sees consistent file content up to the offset it has read; readers do NOT need to coordinate with the writer.

The `O_APPEND` semantics on the writer side mean a reader at position `N` will see writes that complete after the reader's open as soon as the reader requests bytes past `N`. There is no cache coherence issue on a single host; cross-host readers (NFS, SMB) are out of scope.

## Examples

A minimal ledger with two events looks like (whitespace added for readability; on disk each line is compact):

```
{"seq":0,"timestamp_nanos":1700000000000000000,"event":{"type":"SessionStarted","payload":{"agent_id":"a","model_id":"m","session_id":"s"}},"parent_hash":"0000000000000000000000000000000000000000000000000000000000000000","self_hash":"52299f4b70c1603526deb7b88cbaa03ca9eb1b91a3da038babb4a175041aec91","signature":"ca2390a43c...","signer_pubkey":"03a107bff3..."}
{"seq":1,"timestamp_nanos":1700000000500000000,"event":{...},"parent_hash":"52299f4b70...","self_hash":"...","signature":"...","signer_pubkey":"03a107bff3..."}
```

(Hashes and signatures truncated for display.)

Inspect with standard tools:

```
cat ~/.provedex/ledger.ndjson | jq .
tail -f ~/.provedex/ledger.ndjson | jq -c '.event.type'
provedex verify
```

## Versioning

This is ledger-format v1. Any change to the line format, append semantics, or read semantics requires:

- A new file `docs/spec/ledger-format-v2.md`.
- A new ADR superseding ADR 0003.
- A bump of `ExportBundle::schema_version`.
- A defined upgrade path for existing v1 ledgers.

The rules in this v1 spec are frozen.
