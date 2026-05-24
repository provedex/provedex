# provedex-langchain

`provedex-langchain` is a LangChain `BaseCallbackHandler` that signs every LLM
call, tool call, and operator session boundary via the Provedex sidecar. Each
event gets an Ed25519 signature and a SHA-256 parent hash, written to a local
NDJSON ledger that anyone with the operator's public key can verify offline.
The primary buyers are regulated-AI shops: healthcare scribes processing clinical
notes, financial bots executing trades or claims, and customer-service agents
subject to FINRA or state AI-act supervision. One `ProvedexCallbackHandler`
instance covers both LangChain LCEL pipelines and LangGraph state machines,
because LangGraph propagates LangChain callbacks for every LLM and tool step.

## Quickstart

```
pip install provedex-langchain
```

Start the sidecar first (default `127.0.0.1:8765`):

```
provedex-agent --rate-limit-off &
```

```python
from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig

handler = ProvedexCallbackHandler(config=ProvedexConfig())
# pass to your chain or graph:
chain.invoke({"q": "hi"}, config={"callbacks": [handler]})
```

## Callback mapping

| LangChain callback(s) | AgentEvent variant | Fields populated |
|---|---|---|
| `start_session()` (operator call) | `SessionStarted` | `agent_id`, `model_id`, `session_id` from config |
| `end_session(reason)` (operator call) | `SessionEnded` | `reason`, `summary_sha256 = sha256("")` |
| `on_llm_start` / `aon_llm_start` | none (buffered by `run_id`) | model id from `serialized.get("id")`, joined prompts, start timestamp |
| `on_chat_model_start` / `aon_chat_model_start` | none (buffered) | model id, flattened message list, start timestamp |
| `on_llm_end` / `aon_llm_end` (paired with start by `run_id`) | `ModelInvoked` | `model_id`, `prompt_sha256`, `response_sha256`, `prompt_tokens` / `response_tokens` from `llm_output.get("token_usage", {})` if present (else 0) |
| `on_llm_error` (paired) | `ModelInvoked` | `response_sha256 = sha256(f"{type(error).__name__}: {error}")` |
| `on_tool_start` / `aon_tool_start` | `ToolCalled` | `tool_name`, `args_sha256` of canonical-JSON of args, `args_redacted` |
| `on_tool_end` / `aon_tool_end` | `ToolReturned` | `tool_name`, `result_sha256`, `latency_ms`, `success = True` |
| `on_tool_error` | `ToolReturned` | `tool_name`, `result_sha256` of error description, `latency_ms`, `success = False` |

**Skipped** (not signed in v0.1): `on_llm_new_token` (per-token noise),
`on_chain_start` / `on_chain_end` (LCEL composition makes chain boundaries
ambiguous), `on_agent_action` / `on_agent_finish` (covered by tool events),
`on_retriever_start` / `on_retriever_end` (no v1 event variant), `on_text`
(no defined semantics).

## Configuration reference

| Field | Type | Default | Description |
|---|---|---|---|
| `agent_url` | `str` | `$PROVEDEX_AGENT_URL` or `http://127.0.0.1:8765` | URL of the running `provedex-agent`. Override via env var `PROVEDEX_AGENT_URL` or constructor argument. |
| `session_id` | `str` | `uuid4()` | Identifier for this call session. Override to tie the ledger entry to your own session ID. |
| `agent_id` | `str` | `"langchain-agent"` | Logical name of your agent. Appears in every signed event for that session. |
| `model_id` | `str` | `"unknown"` | LLM model identifier. Used in `ModelInvoked` events when the callback args do not supply one. |
| `on_sign_failure` | `"warn" \| "raise" \| "silent"` | `"warn"` | What to do when the agent returns 4xx. `warn` logs a warning and continues. `raise` propagates the exception out of the background worker - useful in test environments. `silent` increments counters only. |
| `queue_size` | `int` | `1000` | Capacity of the internal deque. When full, the oldest queued event is dropped. |
| `request_timeout_seconds` | `float` | `2.0` | HTTP timeout for each POST to the agent. |
| `shutdown_drain_seconds` | `float` | `5.0` | How long to wait for the queue to drain after `handler.stop()` before returning. |

## Session lifecycle

A session groups a set of LLM and tool events under a single `SessionStarted` /
`SessionEnded` pair. The operator controls session boundaries - the handler does
not infer them from chain hierarchy.

Explicit form:

```python
handler.start_session()
chain.invoke({"q": "hi"}, config={"callbacks": [handler]})
handler.end_session(reason="request_complete")
```

Context manager (sync):

```python
with handler.session("user-12345-request"):
    chain.invoke({"q": "hi"}, config={"callbacks": [handler]})
```

Context manager (async):

```python
async with handler.session("user-12345-request"):
    await chain.ainvoke({"q": "hi"}, config={"callbacks": [handler]})
```

On exception, the context manager calls `end_session` with `reason` set to the
exception class name, so the ledger always has a closed session boundary.

## Latency budget

The handler's hot path is a single `deque.appendleft()` call. The background
worker thread drains the deque and performs the HTTP POST off the LLM call
thread.

Measured against a 1ms-latency mock agent with a 1000-callback burst
(one `on_llm_start` + `on_llm_end` pair per iteration):

- p50 producer overhead: 2.5 microseconds
- p99 producer overhead: 5 microseconds

The LLM call thread is not blocked by network I/O.

## Failure modes

| Failure | Behaviour | Counter |
|---|---|---|
| Agent unreachable (ConnectionRefused) | warn + drop | `dropped_total` |
| Agent slow (timeout) | warn + drop | `dropped_total` |
| Agent 4xx | log error + apply `on_sign_failure` | `dropped_total` |
| Agent 5xx | warn + drop | `dropped_total` |
| Queue overflow | drop oldest, rate-limited warning | `overflow_total` |
| Callback with missing fields | log warning, skip enqueue | n/a |
| `run_id` missing on `on_llm_end` (no paired start) | log warning, skip emission | n/a |

Counters are readable as attributes on the handler instance:
`handler.signed_total`, `handler.dropped_total`, `handler.overflow_total`.

## LangGraph

LangGraph fires LangChain callbacks for every LLM and tool step inside a graph.
No additional integration is required. The operator wraps the graph invocation
inside a session context:

```python
async with handler.session("graph-run"):
    await graph.invoke(state, config={"callbacks": [handler]})
```

Graph-specific events (CheckpointSaved, node enter / exit, edge transitions) are
NOT signed in v0.1. They are documented as a follow-up item once a customer
surfaces a concrete audit requirement for checkpoint-level granularity.

## Architecture

This binding does not contain the signing primitive. The primitive is the Rust
sidecar at https://github.com/provedex/provedex. The binding translates
LangChain callback arguments into `AgentEvent` shapes per
`docs/spec/event-schema-v1.md` and POSTs them to the sidecar over loopback
HTTP. The translation is pure Python with no C extensions required.

The sidecar signs each event with the operator's Ed25519 private key and chains
it via SHA-256 parent hashes into a local NDJSON ledger. Each event record
contains the signature, the parent hash, and the payload hash. Anyone with the
operator's public key can run `provedex verify` against the ledger file offline,
without network access and without trusting a third party.

## Verifying the ledger

```
provedex verify
provedex verify --ledger ~/.provedex/ledger.ndjson
provedex verify --ledger /path/to/sandboxed/ledger.ndjson
```

The command reads the NDJSON file, checks the Ed25519 signature on every record,
and verifies the SHA-256 hash chain is unbroken. Exit code 0 means the ledger is
intact.

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

License: Apache-2.0. Main repo: https://github.com/provedex/provedex
