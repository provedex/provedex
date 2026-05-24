# provedex-pipecat

provedex-pipecat is a Pipecat FrameProcessor that signs every frame in your
voice agent pipeline using the Provedex sidecar. One line of integration code.
Hash-chained, Ed25519-signed audit ledger as output. Built for regulated voice
agents: healthcare scribes, financial voice bots, claims handlers.

The binding translates Pipecat frames into `AgentEvent` shapes and POSTs them
over loopback HTTP to the Provedex sidecar. The sidecar holds the signing key
and ledger. Your pipeline code never touches a key.


## Quickstart

```
pip install provedex-pipecat
```

```python
from provedex_pipecat import ProvedexConfig, ProvedexFrameProcessor

processor = ProvedexFrameProcessor(config=ProvedexConfig())
# Add `processor` anywhere in your Pipecat pipeline.
```

Assumes `provedex-agent` is running on `127.0.0.1:8765` (the default). To
start the agent:

```
provedex-agent
```

Override the URL via the `PROVEDEX_AGENT_URL` environment variable or the
`agent_url` constructor argument.


## Frame mapping

| Pipecat Frame | AgentEvent variant | Fields populated |
|---|---|---|
| `StartFrame` | `SessionStarted` | `agent_id`, `model_id` (both from config), `session_id` (config or uuid) |
| `EndFrame` | `SessionEnded` | `reason = "pipeline_end"`, `summary_sha256 = sha256("")` |
| `TranscriptionFrame` (final) | `UtteranceCaptured` | `audio_sha256 = sha256(transcript bytes)`, `transcript`, `lang`, `duration_ms = 0` if unknown |
| `LLMMessagesFrame` + `LLMFullResponseEndFrame` (paired) | `ModelInvoked` | `model_id` (from config or inferred), `prompt_sha256 = sha256(canonical_json(messages))`, `response_sha256 = sha256(end_frame.text)`, `prompt_tokens = 0` if unknown, `response_tokens = 0` if unknown |
| `TextFrame` (final, post-LLM, no end-frame pairing) | `UtteranceSpoken` | `text_sha256 = sha256(text)`, `text`, `audio_sha256 = sha256(b"")` |
| `FunctionCallInProgressFrame` | `ToolCalled` | `tool_name`, `args_sha256 = sha256(canonical_json(arguments))`, `args_redacted = arguments` |
| `FunctionCallResultFrame` | `ToolReturned` | `tool_name`, `result_sha256 = sha256(canonical_json(result))`, `latency_ms` (measured if start-frame timestamp captured), `success` |

**Skipped frames** (not signed):

- `AudioRawFrame` - too high frequency; hashing every audio chunk would
  saturate the ledger with noise.
- `InterimTranscriptionFrame` - not final; only committed transcripts are
  auditable.
- `MetricsFrame` - telemetry, not a decision event.
- `SystemFrame` subclasses - control flow, not agent output.
- `LLMFullResponseStartFrame` - used internally for pairing only.


## Configuration reference

| Field | Type | Default | Description |
|---|---|---|---|
| `agent_url` | `str` | `$PROVEDEX_AGENT_URL` or `http://127.0.0.1:8765` | URL of the running `provedex-agent`. Override via env var `PROVEDEX_AGENT_URL` or constructor argument. |
| `session_id` | `str` | `uuid4()` | Identifier for this call session. Passed as-is into `SessionStarted`. Override to tie the ledger entry to your own session ID. |
| `agent_id` | `str` | `"pipecat-agent"` | Logical name of your agent. Appears in every signed event for that session. |
| `model_id` | `str` | `"unknown"` | LLM model identifier. Used in `ModelInvoked` events. |
| `include_frames` | `list[type] \| None` | `None` (use default list) | Override the set of frame types to sign. `None` uses the mapping table above. |
| `on_sign_failure` | `"warn" \| "raise" \| "silent"` | `"warn"` | What to do when the agent returns 4xx. `warn` logs a warning and continues. `raise` propagates the exception out of the background worker and kills the pipeline - useful in test environments. `silent` increments counters only. |
| `queue_size` | `int` | `1000` | Capacity of the internal deque. When full, the oldest queued event is dropped. |
| `request_timeout_seconds` | `float` | `2.0` | HTTP timeout for each POST to the agent. |
| `shutdown_drain_seconds` | `float` | `5.0` | How long to wait for the queue to drain after `EndFrame` before forwarding it downstream. |


## Latency budget

Test rig: 1000-frame burst with a 1 ms simulated agent response time
(`tests/test_async_smoke.py`).

| Percentile | Producer block time |
|---|---|
| p50 | 1.1 microseconds |
| p99 | 2.2 microseconds |

The producer just enqueues onto a deque; the background worker does the HTTP
POST off the audio hot path. The signing round-trip never touches the frame's
pass-through latency.


## Failure modes

| Failure | Behaviour | Counter |
|---|---|---|
| Agent unreachable (ConnectionRefused) | warn + drop | `dropped_total` |
| Agent slow (timeout) | warn + drop | `dropped_total` |
| Agent 4xx | log error + apply `on_sign_failure` | `dropped_total` |
| Agent 5xx | warn + drop | `dropped_total` |
| Queue overflow | drop oldest, rate-limited warning | `overflow_total` |
| Frame mapping failure | log warning, drop event | n/a |

Counters are readable as attributes on the processor instance:
`processor.signed_total`, `processor.dropped_total`, `processor.overflow_total`.


## Architecture

This binding does not contain the signing primitive. The primitive is the Rust
agent at https://github.com/provedex/provedex. The binding translates Pipecat
frames into `AgentEvent` shapes per `docs/spec/event-schema-v1.md` and POSTs
them to the agent over loopback HTTP. No key material passes through Python.

The agent signs each event with the operator's Ed25519 key and chains it via
SHA-256 parent hashes into a local NDJSON ledger. Anyone with the public key
can verify the ledger offline without contacting any external service.


## Verifying the ledger

```
provedex verify
```

```
provedex verify --ledger ~/.provedex/ledger.ndjson
```

```
provedex verify --ledger /path/to/sandboxed/ledger.ndjson
```

`provedex verify` walks the chain, checks each Ed25519 signature, recomputes
each SHA-256 parent hash, and exits 0 on success or 1 with a diagnostic on the
first broken link.


## Regulatory context

Tamper-evident audit logs are a direct requirement across several frameworks
currently in force or taking effect in 2026. The EU AI Act Article 12 requires
high-risk AI deployments to produce audit logs that are tamper-evident and
retained for at least six months; enforcement applies from August 2, 2026.
The Colorado AI Act (effective February 1, 2026) requires deployers of
high-risk AI systems to maintain records sufficient to demonstrate compliance
with consumer protection obligations. HIPAA's audit-control safeguard
(45 CFR 164.312(b)) requires clinical voice agents to record and examine
system activity, which for AI scribes means a verifiable transcript of every
utterance processed. FINRA's 2026 examination priorities identify AI agent
auditability as a focus area for broker-dealer supervision. A hash-chained,
Ed25519-signed ledger satisfies the tamper-evident requirement across all four
frameworks with a single integration point.


---

License: Apache-2.0

Main repo: https://github.com/provedex/provedex
