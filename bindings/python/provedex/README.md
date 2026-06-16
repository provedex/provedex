# provedex (native Python SDK)

Native, in-process Ed25519 signing and hash-chaining for AI-agent evidence. Byte-identical to the Provedex Rust reference: a ledger signed here verifies with `provedex verify`, and vice versa.

This is the opt-in fast-path. The default integration for non-Rust apps is the localhost sidecar (`provedex-agent`); see ADR 0004. Use this binding when you want sub-millisecond, in-process signing with no extra process to run.

## Install

```bash
pip install provedex
```

Pre-built wheels ship for cpython 3.11+ on Linux x86_64, Linux aarch64, and macOS arm64. No Rust toolchain required to install. Add `provedex` to the requirements of the backend service that runs your AI agents.

## How it fits your backend

`provedex` is a library you embed in the backend that runs your agents and automations, not a separate service. The model:

- **Sign in-process.** Wherever your agent does something worth proving (an LLM call, a tool call, an utterance), you call `session.record(...)`. The event is signed and appended to a local ledger as it happens. No network hop, no sidecar.
- **The key and the ledger live on the backend host.** The signing key is read once at startup from a path you control. The ledger is an append-only NDJSON file on that host.
- **Verify anywhere, later, by anyone.** A regulator, an auditor, or you on a laptop can run `provedex verify` (or `provedex.verify_file`) against the ledger with only the public key, offline, with no involvement from the backend that produced it. That separation is the point: the operator never has to be trusted for the integrity of the log.

```
your backend (agents + automations)          an auditor, months later
  pip install provedex                          (only needs the public key)
  session.record(event)  --->  ledger.ndjson  --->  provedex verify  ->  VALID / BROKEN
  (signing key stays here)     (the evidence)       (offline, no trust in you)
```

## Quickstart

```python
import hashlib
import os

import provedex


def sha256_hex(data: str | bytes) -> str:
    """Event payloads carry SHA-256 hex digests, not raw content. Hash with
    your own hashlib; what you hash vs. keep in clear is your decision."""
    if isinstance(data, str):
        data = data.encode("utf-8")
    return hashlib.sha256(data).hexdigest()


# Once at startup. The key is created on first run, then reused (0600 on unix).
keypair = provedex.SigningKeypair.load_or_create(
    os.path.expanduser("~/.provedex/keys/ed25519.key")
)

# Open one session per conversation / agent run. Resumes if the ledger exists.
session = provedex.Session.open(
    keypair=keypair,
    ledger_path=os.path.expanduser("~/.provedex/ledger.ndjson"),
    session_id="conversation-42",
)

session.record(
    provedex.events.session_started(
        agent_id="intake-bot", model_id="gpt-4o", session_id="conversation-42"
    )
)

prompt = "Summarize the patient's chief complaint."
response = call_your_model(prompt)  # your code
signed = session.record(
    provedex.events.model_invoked(
        model_id="gpt-4o",
        prompt_sha256=sha256_hex(prompt),
        response_sha256=sha256_hex(response),
        prompt_tokens=120,
        response_tokens=80,
    )
)
print(signed.seq, signed.self_hash)

session.record(provedex.events.session_ended(reason="completed", summary_sha256=sha256_hex(response)))

# Anyone with the public key can now verify this ledger, offline.
report = provedex.verify_file(os.path.expanduser("~/.provedex/ledger.ndjson"))
assert report.ok
```

## Events

One typed factory per core variant. The variant set is locked to the Rust core; there is no Python-only event. All arguments are keyword-only.

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

### What goes in the fields

- **`*_sha256` fields take a 64-character SHA-256 hex digest that you compute.** The ledger stores digests, not raw prompts, responses, or audio - this keeps sensitive content out of the evidence while still proving exactly what was processed. For **raw text** (a prompt, a response, a transcript), hash the UTF-8 bytes with `sha256_hex` above. For a **structured payload** (a dict or list, such as tool arguments), hash its canonical JSON so the digest is reproducible by anyone, in any language:

  ```python
  def canonical_sha256(payload: object) -> str:
      return hashlib.sha256(provedex.canonical_json(payload)).hexdigest()
  ```

  `provedex.canonical_json` is the same deterministic encoder the chain signs with (sorted keys, fixed number formatting), so an auditor re-hashing the original gets the identical digest. Do NOT hash an ad-hoc `str(dict)` or `json.dumps` - those are not stable across runs or languages. Pick one convention (raw-bytes for text, canonical-JSON for structures), apply it consistently, and document it for whoever verifies.
- **`args_redacted` (on `tool_called`) is a dict you store in clear** - the non-sensitive subset of the tool arguments (for example an account ID but not an SSN). You decide what is safe to keep readable. It is signed as canonical JSON, so it must be JSON-serializable; non-finite floats (NaN, Infinity) are rejected.
- **`transcript` / `text` on the utterance events are stored in clear** alongside their hash, because a transcript is usually the thing an auditor wants to read. Omit or redact upstream if your data policy forbids it.

Provedex does not redact for you. What is hashed versus kept in clear is the customer's decision (see "Out of scope" in the project README).

## Sessions

`Session` is the primary path: it allocates the next `seq`, chains each event to the previous `self_hash`, appends to the ledger, and fsyncs. On `open` it reads any existing ledger and resumes from the last event, so a restarted process continues the same chain rather than starting over.

- **One session per conversation or run.** Use a distinct `session_id` per logical conversation so the boundary is meaningful to an auditor. (A process-wide session works but means "one session = the process lifetime," which usually is not what you want.)
- **The ledger file is the chain.** Reopening the same `ledger_path` resumes that chain regardless of `session_id`; the `session_id` is recorded inside events as metadata, it does not locate the ledger. If you want separate chains per conversation, give each its own `ledger_path`; if you want one continuous chain, point them all at the same file.
- **Concurrency.** A single `Session` is safe to call from multiple threads or async tasks; the core serializes each seal-and-append, so the chain stays valid under concurrent writers. There is no `close()` - a `Session` holds an appendable file handle that is released when it is garbage-collected.

`record` (and `sign_event`) return a `SignedEvent` with `.seq`, `.timestamp_nanos`, `.event` (the tagged `{"type", "payload"}` dict), `.parent_hash`, `.self_hash`, `.signature`, `.signer_pubkey`, and `.to_json()` (the exact NDJSON ledger line).

For full manual control (you own the seq and parent hash), there is a low-level path:

```python
signed = provedex.sign_event(
    event=e, seq=0, parent_hash=provedex.GENESIS_PARENT_HASH, keypair=keypair
)
```

## Native binding vs. the framework adapters

If your agents are built on a framework, you have a choice:

- **`provedex` (this package)** - in-process, fastest (~11 us/seal), no extra process. You call `session.record(...)` yourself at each event. Most control; you instrument the code.
- **`provedex-pipecat` / `provedex-langchain`** - auto-capture every frame / LLM / tool call via the framework's hooks, no manual `record` calls, but they route through the `provedex-agent` sidecar (1-2 ms/event) and need that process running.

Same backend, different trade-off: native = manual + in-process, adapters = automatic + via sidecar.

## Latency

| Operation | Cost |
|-----------|------|
| `sign_event` / seal (no I/O), GIL released | 11-15 us |
| `Session.record` (seal + append + fsync) | 3.8 ms, dominated by fsync |

`Session.record` fsyncs for durability, the same as the sidecar. On an async backend, run it off the event loop so the fsync does not block:

```python
signed = await asyncio.to_thread(session.record, event)
```

## Failure modes

All failures raise; nothing returns an error sentinel.

| Exception | When |
|-----------|------|
| `provedex.KeyLoadError` | bad key file (length, hex, missing on `load`) |
| `provedex.SigningError` | seal/hash failure, bad event shape in `from_dict`, non-finite float in a payload |
| `provedex.LedgerError` | ledger read/write failure |
| `provedex.ChainError` | malformed (unparseable) ledger input in `verify_file` |

`verify_chain` / `verify_file` do NOT raise on a broken chain; they return `ChainReport(ok=False, broken_at=<seq>, reason=...)`. A broken chain is data, not an exception. Out-of-range or negative integers passed to an event field raise the standard `OverflowError`.

## Byte-compat

There is one canonical-JSON encoder in the whole system: the Rust one. This binding calls it directly, so the bytes it signs are identical to the sidecar and the CLI. The repo's `tests/compat/vectors/` golden suite and the cross-verify tests assert it.

JSON numbers follow the Rust reference exactly: an integer and a float are distinct (`1` and `1.0` hash differently), and non-finite floats (NaN, Infinity) are rejected rather than silently coerced.

## Verifying offline

Anyone with the public key can verify the ledger with no involvement from you:

```bash
provedex verify --ledger ~/.provedex/ledger.ndjson
```

In Python, `provedex.verify_file(path)` returns a `ChainReport` (`.ok`, `.event_count`, `.broken_at`, `.reason`, `.root_hash`). On a valid chain `.broken_at` and `.reason` are `None`.

The signer's public key is `keypair.pubkey_hex` (64 hex chars). Publish it out of band - a trust page, a key registry, a signed disclosure - not only from the same service that produced the ledger, or a verifier is back to trusting the operator, which is the trust this design removes.

## License

Apache-2.0.
