# Design: provedex-langchain (Python binding for LangChain + LangGraph)

Status: approved 2026-05-24

This document captures the agreed scope and architecture for the second Python binding. It is the input to the implementation plan that lives at `docs/superpowers/plans/`.

## Goal

Ship `provedex-langchain` as a PyPI package that signs every LLM call, tool call, and operator-declared session boundary inside a LangChain pipeline (and, by inheritance, inside a LangGraph graph) by POSTing to the localhost `provedex-agent` sidecar. One handler instance, hash-chained Ed25519-signed audit ledger as output.

## Why LangChain (and LangGraph by inheritance) next

- LangChain is the dominant agent framework in the regulated-AI buyer's stack.
- LangGraph (built on LangChain) is becoming the standard for stateful agentic workflows in healthcare / financial pipelines.
- Two named competitors (Aira, asqav) are LangChain-first. Shipping a LangChain binding closes the parity gap.
- Pipecat binding already proved the architecture; this sprint reuses the same pattern with a different translation table.

## Decisions locked in brainstorming

1. **One package** (`provedex-langchain`) covers both LangChain and LangGraph. LangGraph fires LangChain callbacks for every LLM / tool / chain step, so a `BaseCallbackHandler` implementation gives audit coverage for both frameworks. Graph-specific events (CheckpointSavedEvent, node enter / exit, edge transitions) are deferred until a customer asks.
2. **Sync + async coverage** via dual inheritance: `ProvedexCallbackHandler(AsyncCallbackHandler, BaseCallbackHandler)`. Both `on_*` and `aon_*` methods delegate to the same enqueue path. Covers sync LCEL chains, async LCEL chains, LangGraph (async), and legacy callback-based code.
3. **Explicit operator-driven session lifecycle**: `start_session()` / `end_session(reason)` plus a `with handler.session("..."):` context manager. No silent inference from chain hierarchy (LCEL composition creates anonymous chains; auto-derivation is too brittle).
4. **Inlined private HTTP client** (`_client.py`). `provedex-client` extraction deferred to the third binding (CrewAI or MCP). The maintenance pain of two inlined copies (pipecat + langchain) is acceptable for one more sprint.

## Architecture

### Module layout

```
bindings/python/provedex-langchain/
  pyproject.toml
  README.md
  src/provedex_langchain/
    __init__.py             exports ProvedexCallbackHandler, ProvedexConfig
    handler.py              ProvedexCallbackHandler (sync + async)
    _client.py              AgentClient (httpx async)
    _state.py               CorrelationState (run_id -> in-flight LLM/tool buffer + dedup)
    mapping.py              pure functions: callback args -> AgentEvent dict
    config.py               ProvedexConfig dataclass + env loading
  tests/
    conftest.py             shared fixtures (agent binary + spawn)
    test_config.py
    test_mapping.py
    test_client.py
    test_state.py
    test_handler_sync.py
    test_handler_async.py
    test_session.py
    test_async_smoke.py
    test_integration.py
  examples/
    langchain_basic.py
    langgraph_basic.py
```

### Component responsibilities

| Module | Responsibility | LOC estimate |
|--------|----------------|--------------|
| `handler.py` | Dual-inheritance callback handler, queue, worker lifecycle, session methods | ~200 |
| `_client.py` | httpx-based POST `/v1/sign` (clone from pipecat, no behavior change) | ~60 |
| `_state.py` | CorrelationState dict keyed on run_id; per-handler frame / run-id dedup | ~70 |
| `mapping.py` | Pure functions translating per-callback args into AgentEvent dicts | ~150 |
| `config.py` | ProvedexConfig dataclass, env loading (PROVEDEX_AGENT_URL) | ~50 |

### Callback to AgentEvent mapping

| LangChain callback(s) | AgentEvent variant | Fields populated |
|-----------------------|---------------------|------------------|
| `start_session()` (operator call, not a callback) | `SessionStarted` | `agent_id`, `model_id`, `session_id` from config |
| `end_session(reason)` (operator call) | `SessionEnded` | `reason`, `summary_sha256 = sha256("")` |
| `on_llm_start` / `aon_llm_start` | none (buffered by `run_id`) | stores model id from `serialized.get("id")`, joined prompts, start timestamp |
| `on_chat_model_start` / `aon_chat_model_start` | none (buffered) | stores model id, flattened message list, start timestamp |
| `on_llm_end` / `aon_llm_end` (paired with start by `run_id`) | `ModelInvoked` | `model_id`, `prompt_sha256 = sha256(canonical_json(prompts_or_messages))`, `response_sha256 = sha256(response.generations[0][0].text)`, `prompt_tokens` / `response_tokens` from `response.llm_output.get("token_usage", {})` if present (else 0) |
| `on_llm_error` (paired) | `ModelInvoked` | `response_sha256 = sha256(f"{type(error).__name__}: {error}")` to record the error class + message |
| `on_tool_start` / `aon_tool_start` | `ToolCalled` | `tool_name` from `serialized.get("name")`, `args_sha256` of canonical-JSON of args, `args_redacted` = parsed `inputs` dict if present else `{"input": input_str}` |
| `on_tool_end` / `aon_tool_end` | `ToolReturned` | `tool_name` from state, `result_sha256` of canonical-JSON of output, `latency_ms` (now - start_time), `success = True` |
| `on_tool_error` | `ToolReturned` | `tool_name` from state, `result_sha256` of error description, `latency_ms`, `success = False` |

Skip: `on_llm_new_token` (per-token noise), `on_chain_start` / `on_chain_end` (LCEL composition makes chain boundaries ambiguous; operator wraps with `start_session` if they want a session), `on_agent_action` / `on_agent_finish` (covered by tool events), `on_retriever_start` / `on_retriever_end` (no v1 variant), `on_text` (no semantics).

LangGraph coverage: graph invocations propagate through LangChain's callback system. Operator wraps `graph.invoke(state, config={"callbacks": [handler]})` inside `with handler.session("graph-run"):`. Graph-specific events (checkpoint, node transitions) are NOT signed in v0.1; documented as a follow-up.

### Hashing semantics

The binding signs the bytes it sees. For LLM prompts, `prompt_sha256` hashes the canonical-JSON encoding of the prompt list (string list for `on_llm_start`, message dict list for `on_chat_model_start`). For tool args, the dict if available, otherwise the stringified input. Same honest-receipt posture as pipecat: the binding signs what it sent to the agent, the agent does the cryptographic canonical-JSON for the actual signature input.

### Configuration

```python
@dataclass
class ProvedexConfig:
    agent_url: str  # env PROVEDEX_AGENT_URL, default http://127.0.0.1:8765
    session_id: str  # auto-generated uuid4 by default
    agent_id: str = "langchain-agent"
    model_id: str = "unknown"
    on_sign_failure: Literal["warn", "raise", "silent"] = "warn"
    queue_size: int = 1000
    request_timeout_seconds: float = 2.0
    shutdown_drain_seconds: float = 5.0
```

Pydantic v2 BaseModel with field validators. Identical shape to pipecat's `ProvedexConfig` to keep operator mental model stable across bindings.

### Backpressure and ordering

Same as pipecat: single background worker, bounded `deque(maxlen=queue_size)`, drop-oldest on overflow, rate-limited overflow warnings (1 per second), `on_sign_failure` policy. Producer hot path is O(1) deque append; signing happens off the LLM call thread.

The handler exposes `start()` and `stop()` for explicit worker lifecycle, plus `with handler.session(reason)` context manager that auto-starts the worker on first entry.

### Error handling

| Failure | Behaviour |
|---------|-----------|
| Agent unreachable (ConnectionRefused) | warn + drop, `dropped_total += 1` |
| Agent slow (timeout) | warn + drop |
| Agent 4xx | log error + apply `on_sign_failure` |
| Agent 5xx | warn + drop |
| Queue overflow | drop oldest, rate-limited warning |
| LangChain callback with missing fields | log warning, skip enqueue |
| `run_id` missing on `on_llm_end` (paired-start never fired) | log warning, skip emission |

## Public API

```python
from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig

handler = ProvedexCallbackHandler(
    config=ProvedexConfig(
        agent_url="http://127.0.0.1:8765",
        agent_id="my-langchain-agent",
        model_id="gpt-4o",
    )
)

# Sync LCEL chain:
from langchain_core.prompts import ChatPromptTemplate
chain = ChatPromptTemplate.from_template("Say hi") | llm
chain.invoke({}, config={"callbacks": [handler]})

# Async LCEL chain or LangGraph:
async with handler.session("user-12345-request"):
    result = await chain.ainvoke({}, config={"callbacks": [handler]})

# Or explicit:
handler.start_session()
chain.invoke({}, config={"callbacks": [handler]})
handler.end_session(reason="request_complete")
```

The `session` context manager: 5-line wrapper that awaits `start()` on first use, calls `start_session()` on enter, calls `end_session(reason)` on exit (even on exception, with reason set to the exception class name). Synchronous version (`with handler.session(...):`) also exists for sync chains.

## Tests

| Layer | Approach |
|-------|----------|
| Unit | pytest + respx. Golden POST shapes per callback type. CorrelationState dedup + pairing. |
| Sync handler | Drive a `FakeListChatModel` sync chain with the handler attached; assert POST sequence. |
| Async handler | Same but with `chain.ainvoke()`. |
| Session lifecycle | Context manager fires SessionStarted on enter, SessionEnded on exit (normal + exception paths). |
| Async smoke | 1000-callback burst, p99 producer block < 1ms, zero drops at default queue. |
| Integration | pytest fixture builds and spawns real `provedex-agent`. Run two pipelines (one LangChain LCEL, one LangGraph) through the handler against `FakeListChatModel`. Then run `provedex verify --ledger` via subprocess. Must exit 0 for both. |

CI lane: extend the existing `bindings-python` matrix to run both bindings, or add a sibling `bindings-python-langchain` job. Decision in the plan; structurally simpler to extend.

## Package metadata

| Field | Value |
|-------|-------|
| Name | `provedex-langchain` |
| Version | `0.1.0` |
| License | Apache-2.0 |
| Python | `>= 3.11` |
| Runtime deps | `langchain-core >= 0.3, < 0.4`, `httpx >= 0.27`, `pydantic >= 2` |
| Dev deps | `pytest`, `pytest-asyncio`, `respx`, `ruff`, `mypy`, `langchain`, `langchain-openai` (FakeListChatModel), `langgraph` |
| Build backend | hatchling |

`langgraph` is a dev dep only - the runtime does not import langgraph. LangGraph users install it separately for their pipeline.

## Documentation

README sections (mirrors pipecat README structure):

1. One-paragraph what + why (regulated-AI wedge, LangChain dominant framework, LangGraph covered transitively).
2. Quickstart: pip install + 5-line code block showing chain.invoke with callbacks=[handler].
3. Callback mapping table (verbatim from this design).
4. Configuration reference.
5. Session lifecycle - explicit method calls vs context manager.
6. Latency budget with measured numbers from `test_async_smoke.py`.
7. Failure modes table.
8. LangGraph note: works via LangChain callbacks; graph-state events deferred.
9. Architecture note - this is a callback adapter, not the primitive. Link to provedex/provedex and event-schema-v1.
10. Verifying the ledger.
11. Regulatory context (same paragraph as pipecat README; identical wedge).

## Risks

| Risk | Mitigation |
|------|------------|
| LangChain callback signatures churn across 0.3.x | Pin `langchain-core >= 0.3, < 0.4`. CI catches breakage. |
| `LLMResult.llm_output.token_usage` shape differs per provider | Defensive `.get(...)` with default 0. Document as best-effort. |
| run_id collisions across concurrent requests | LangChain guarantees run_id is UUID4. Dict keyed on run_id is safe. |
| Sync-handler-from-async or vice versa | Both interfaces implemented. Tests cover both. |
| LangGraph state events not covered | Documented out-of-scope; LLM + tool coverage is the audit-relevant subset. Follow-up issue if a customer asks. |
| Two inlined `_client.py` files now (pipecat + langchain) | One sprint of duplication is acceptable. Extract on third binding. |

## Out of scope

- LangGraph-specific events (CheckpointSavedEvent, node enter / exit, edge transitions).
- Streaming token signing (`on_llm_new_token`).
- Retriever events (`on_retriever_start` / `on_retriever_end`) - no v1 variant.
- CrewAI binding (separate sprint).
- MCP binding (separate sprint).
- `provedex-client` extraction (deferred to third binding).
- LangChain v0.2 backport - we target 0.3 only.

## References

- Agent HTTP API: `docs/spec/openapi.yaml`
- AgentEvent schema: `docs/spec/event-schema-v1.md`
- Canonical JSON: `docs/spec/canonical-json.md`
- ADR-0004 sidecar-as-default: `docs/adr/0004-sidecar-as-default-integration.md`
- Pipecat binding design (for shape parity): `docs/superpowers/specs/2026-05-24-provedex-pipecat-binding-design.md`
- LangChain callbacks docs: https://python.langchain.com/docs/concepts/callbacks/
- LangChain `BaseCallbackHandler` / `AsyncCallbackHandler` API: https://api.python.langchain.com/en/latest/core_api_reference.html
- LangGraph docs: https://langchain-ai.github.io/langgraph/
