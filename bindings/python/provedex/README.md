# provedex (native Python SDK)

Native, in-process Ed25519 signing and hash-chaining for AI-agent evidence. Byte-identical to the Provedex Rust reference: a ledger signed here verifies with `provedex verify`, and vice versa.

This is the opt-in fast-path. The default integration for non-Rust apps is the localhost sidecar (`provedex-agent`); see ADR 0004. Use this binding when you want sub-millisecond, in-process signing with no extra process to run.

## Install

```bash
pip install provedex
```

Pre-built wheels ship for cpython 3.11+ on Linux x86_64, Linux aarch64, and macOS arm64. No Rust toolchain required to install.

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

One typed factory per core variant. The variant set is locked to the Rust core; there is no Python-only event.

| Factory | Signs |
|---------|-------|
| `events.session_started(agent_id, model_id, session_id)` | session open |
| `events.utterance_captured(audio_sha256, transcript, lang, duration_ms)` | inbound speech |
| `events.tool_called(tool_name, args_sha256, args_redacted)` | tool invocation |
| `events.tool_returned(tool_name, result_sha256, latency_ms, success)` | tool result |
| `events.model_invoked(model_id, prompt_sha256, response_sha256, prompt_tokens, response_tokens)` | LLM call |
| `events.utterance_spoken(text_sha256, text, audio_sha256)` | outbound speech |
| `events.session_ended(reason, summary_sha256)` | session close |

`events.from_dict({"type": ..., "payload": ...})` rebuilds an event from its stored JSON.

## Sessions vs. low-level signing

`Session` is the primary path: it allocates the next `seq`, chains each event to the previous `self_hash`, appends to the ledger, and fsyncs, resuming from any pre-existing events on open. For full manual control there is a low-level path:

```python
signed = provedex.sign_event(
    event=e, seq=0, parent_hash=provedex.GENESIS_PARENT_HASH, keypair=keypair
)
```

## Latency

| Operation | Cost |
|-----------|------|
| `sign_event` / seal (no I/O), GIL released | 11-15 us |
| `Session.record` (seal + append + fsync) | 3.8 ms, dominated by fsync |

`Session.record` fsyncs for durability, the same as the sidecar. On an async backend, run it off the event loop:

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

`verify_chain` / `verify_file` do NOT raise on a broken chain; they return `ChainReport(ok=False, broken_at=<seq>, reason=...)`. A broken chain is data.

## Byte-compat

There is one canonical-JSON encoder in the whole system: the Rust one. This binding calls it directly, so the bytes it signs are identical to the sidecar and the CLI. The repo's `tests/compat/vectors/` golden suite and the cross-verify tests assert it.

## Verifying offline

Anyone with the public key can verify the ledger with no involvement from you:

```bash
provedex verify --ledger ./ledger.ndjson
```

## License

Apache-2.0.
