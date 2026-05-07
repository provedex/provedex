# Event schema (v1)

Status: normative.

This document specifies the JSON shape of every `AgentEvent` variant emitted by a Provedex client. Bindings (Python, Node, Java, Go, Ruby, sidecar HTTP API) must produce events with these exact field names, types, and tag layouts so signed events from any binding verify against the same Rust reference implementation.

References:
- ADR 0001 (canonical JSON format used for hashing).
- ADR 0002 (hash chain shape and the four-field hashed map).
- `docs/spec/canonical-json.md` (encoding rules used by `self_hash`).

## Tag layout

Every event is a JSON object with two top-level fields:

```
{
  "type": "<variant name>",
  "payload": { <variant-specific fields> }
}
```

`type` carries the variant name as a UTF-8 string. `payload` carries the variant's fields as a JSON object. This matches `serde`'s tagged enum representation `#[serde(tag = "type", content = "payload")]` used by the Rust reference at `crates/provedex-core/src/event.rs`.

When the event is hashed, the entire `{"type": ..., "payload": ...}` object becomes the value of the `event` key in the hashed four-field map. Canonical-JSON sorts the top-level keys alphabetically, so `payload` precedes `type` in the byte stream.

## Variants (v1)

This spec defines seven variants. Adding a variant is a v2 change (new spec file, new ADR, schema_version bump). Adding a field to an existing payload is also a v2 change.

### `SessionStarted`

Marks the beginning of a signing session. Emitted once per agent process / per ledger boot.

| Field | Type | Notes |
|-------|------|-------|
| `agent_id` | string | operator-chosen identifier for the agent (e.g. `"voice-scribe-prod-a1"`). |
| `model_id` | string | identifier of the LLM the session is wired to (e.g. `"llama3.2:3b"`, `"gpt-4o"`). |
| `session_id` | string | UUIDv4 or operator-chosen unique session identifier. |

Example:

```
{
  "type": "SessionStarted",
  "payload": {
    "agent_id": "agent-1",
    "model_id": "llama3.2:3b",
    "session_id": "session-demo"
  }
}
```

### `UtteranceCaptured`

A user utterance was transcribed by the agent's STT pipeline.

| Field | Type | Notes |
|-------|------|-------|
| `audio_sha256` | string (64 hex chars) | SHA-256 of the raw audio bytes the agent received. |
| `transcript` | string | the transcribed text. UTF-8. |
| `lang` | string | ISO 639-1 language code (`"en"`, `"es"`, ...). |
| `duration_ms` | u64 | duration of the audio in milliseconds. |

Example:

```
{
  "type": "UtteranceCaptured",
  "payload": {
    "audio_sha256": "9f3b2a1c0d4e5f6789abcdef0123456789abcdef0123456789abcdef01234567",
    "transcript": "patient reports chest pain",
    "lang": "en",
    "duration_ms": 2400
  }
}
```

### `ToolCalled`

The agent invoked a tool (function) the LLM wanted to call.

| Field | Type | Notes |
|-------|------|-------|
| `tool_name` | string | operator-chosen identifier (e.g. `"lookup_patient_history"`). |
| `args_sha256` | string (64 hex chars) | SHA-256 of the canonical-JSON encoding of the full args object. |
| `args_redacted` | object | the args object with PII fields redacted. The redaction policy is the customer's responsibility; this field carries whatever the customer chose to log. |

Example:

```
{
  "type": "ToolCalled",
  "payload": {
    "tool_name": "lookup_patient_history",
    "args_sha256": "aaaa1111bbbb2222cccc3333dddd4444eeee5555ffff6666aaaa1111bbbb2222",
    "args_redacted": { "patient_id": "<redacted>" }
  }
}
```

### `ToolReturned`

The tool finished. Records the outcome and timing.

| Field | Type | Notes |
|-------|------|-------|
| `tool_name` | string | the same name used in the matching `ToolCalled`. |
| `result_sha256` | string (64 hex chars) | SHA-256 of the canonical-JSON encoding of the result object. |
| `latency_ms` | u64 | wall-clock time the tool took. |
| `success` | bool | `true` if the tool returned without error, `false` otherwise. |

### `ModelInvoked`

The agent called the LLM. Records hashes (not contents) of the prompt and response, plus token counts.

| Field | Type | Notes |
|-------|------|-------|
| `model_id` | string | the model identifier (matches the SessionStarted `model_id` unless the agent switched mid-session). |
| `prompt_sha256` | string (64 hex chars) | SHA-256 of the prompt as the agent sent it. |
| `response_sha256` | string (64 hex chars) | SHA-256 of the response as the agent received it. |
| `prompt_tokens` | u32 | prompt token count reported by the LLM. |
| `response_tokens` | u32 | response token count reported by the LLM. |

### `UtteranceSpoken`

The agent spoke an utterance via TTS.

| Field | Type | Notes |
|-------|------|-------|
| `text_sha256` | string (64 hex chars) | SHA-256 of the spoken text. |
| `text` | string | the spoken text in UTF-8. Plain transcript; PII redaction is the customer's responsibility. |
| `audio_sha256` | string (64 hex chars) | SHA-256 of the synthesized audio bytes. Empty string if TTS was unavailable and no audio was produced. |

### `SessionEnded`

Marks the end of a signing session. Optional but recommended.

| Field | Type | Notes |
|-------|------|-------|
| `reason` | string | operator-chosen reason (`"user_hangup"`, `"timeout"`, `"server_shutdown"`, ...). |
| `summary_sha256` | string (64 hex chars) | SHA-256 of any post-session summary the agent emits. Empty string permitted. |

## Field type rules

- All `*_sha256` fields are exactly 64 lowercase hex characters. Validation is the implementation's responsibility; the Rust reference does not currently enforce length, but a future ADR may.
- All `*_ms`, `*_tokens`, and timestamp-like numeric fields are unsigned. Bindings that use signed types must reject negative values.
- All string fields are UTF-8. Empty strings are permitted unless noted.
- All field names are exactly the ASCII bytes shown. No alternate spellings, no camelCase variants.

## Test vectors

Each vector below is the result of canonical-JSON encoding the variant. A binding implementation MUST produce these exact bytes and SHA-256 for the same input. Reproduce via:

```
cargo run -p provedex-core --example print_test_vectors
```

### Vector A: SessionStarted

Input:

```
{
  "type": "SessionStarted",
  "payload": {
    "agent_id": "agent-1",
    "model_id": "llama3.2:3b",
    "session_id": "session-demo"
  }
}
```

Canonical bytes (note alphabetical key sort puts `payload` before `type`):

```
{"payload":{"agent_id":"agent-1","model_id":"llama3.2:3b","session_id":"session-demo"},"type":"SessionStarted"}
```

Length: 111 bytes.
SHA-256: `ff330e27c01e0255bf9938540ca41f4ae210c0d2dcb7dd3af8664ccdccaed7b4`.

### Vector B: UtteranceCaptured

Input:

```
{
  "type": "UtteranceCaptured",
  "payload": {
    "audio_sha256": "9f3b2a1c0d4e5f6789abcdef0123456789abcdef0123456789abcdef01234567",
    "transcript": "patient reports chest pain",
    "lang": "en",
    "duration_ms": 2400
  }
}
```

Canonical bytes:

```
{"payload":{"audio_sha256":"9f3b2a1c0d4e5f6789abcdef0123456789abcdef0123456789abcdef01234567","duration_ms":2400,"lang":"en","transcript":"patient reports chest pain"},"type":"UtteranceCaptured"}
```

Length: 195 bytes.
SHA-256: `54fa3c63e96cf2ff520374dce39f85a2949e651a3c981be149684a61221b8023`.

### Vector C: ModelInvoked

Input:

```
{
  "type": "ModelInvoked",
  "payload": {
    "model_id": "gpt-4o",
    "prompt_sha256": "aaaa1111bbbb2222cccc3333dddd4444eeee5555ffff6666aaaa1111bbbb2222",
    "response_sha256": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "prompt_tokens": 482,
    "response_tokens": 71
  }
}
```

Canonical bytes:

```
{"payload":{"model_id":"gpt-4o","prompt_sha256":"aaaa1111bbbb2222cccc3333dddd4444eeee5555ffff6666aaaa1111bbbb2222","prompt_tokens":482,"response_sha256":"1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef","response_tokens":71},"type":"ModelInvoked"}
```

Length: 264 bytes.
SHA-256: `2247c06923a60d74ef10613f3484bee5234a0d55c3734ef05bce5c9bf1dca012`.

### Vector D: SessionEnded

Input:

```
{
  "type": "SessionEnded",
  "payload": {
    "reason": "user_hangup",
    "summary_sha256": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
  }
}
```

Canonical bytes:

```
{"payload":{"reason":"user_hangup","summary_sha256":"deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"},"type":"SessionEnded"}
```

Length: 142 bytes.
SHA-256: `eea60e53b8d2037b191b871dc55e4bdf2df368e10f24cf718e6bcc1b214cbdad`.

## Versioning

This is event-schema v1. Adding a new variant or modifying any payload field's name, type, or presence requires:

- A new file `docs/spec/event-schema-v2.md`.
- A new ADR documenting the change and naming the deprecated variants.
- A bump of `ExportBundle::schema_version` from 1 to 2.
- Coordinated upgrade across all bindings.

The variants and payload shapes in this v1 spec are frozen. The Rust reference at `crates/provedex-core/src/event.rs` is the authoritative implementation; any divergence between this document and the Rust source is a bug in this document.
