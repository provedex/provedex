# Design: provedex-pipecat (Python binding for Pipecat voice agents)

Status: approved 2026-05-24

This document captures the agreed scope and architecture for the first Python binding. It is the input to the implementation plan that lives at `docs/superpowers/plans/`.

## Goal

Ship `provedex-pipecat` as a PyPI package that signs every Pipecat `Frame` flowing through a voice-agent pipeline. The signing path is one POST per signed event to the localhost `provedex-agent` sidecar. The Pipecat user wires one processor into the pipeline and gets a hash-chained signed ledger as output.

## Why Pipecat first

- Voice agents in healthcare and finance are the regulated-AI wedge. Pipecat is the production voice stack used by those teams.
- No competitor (Aira, asqav, SidClaw, Agent Passport, AgentMint) has shipped a Pipecat integration. First-mover lane.
- Pipecat's Frame model is tight and well-typed. Easier to ship cleanly than LangChain's callback explosion.

## Decisions locked in brainstorming

1. **Event schema:** Map Pipecat frames to existing event-schema-v1 variants. No spec changes, no ADR. Per `docs/CLAUDE.md` rule, event-schema-v1 is frozen once binding code ships. The seven existing variants cover Pipecat semantically with some impedance documented below.
2. **Package split:** One package now (`provedex-pipecat`) with an inlined private async HTTP client (`_client.py`). Extract a shared `provedex-client` package when the second binding lands.
3. **Backpressure:** Single background worker per processor, bounded `asyncio.Queue` (default 1000), drop-oldest on overflow with a rate-limited warning. Preserves pipeline order in the ledger.
4. **Integration test agent:** CI builds `provedex-agent` from source via cargo (~5 min added). pytest fixture spawns the binary on a random port with a sandboxed ledger.
5. **Agent port default:** `http://127.0.0.1:8765`. The original spec had `7777` which was a typo; the sidecar default is 8765.

## Architecture

### Module layout

```
bindings/python/provedex-pipecat/
  pyproject.toml
  README.md
  src/provedex_pipecat/
    __init__.py         exports ProvedexFrameProcessor, ProvedexConfig
    processor.py        ProvedexFrameProcessor(FrameProcessor)
    _client.py          AgentClient wrapping httpx.AsyncClient
    mapping.py          Frame -> AgentEvent pure functions
    config.py           ProvedexConfig dataclass + env loading
    _state.py           per-processor stateful correlation buffer
  tests/
    test_processor.py     mocked agent + golden POST shape per frame
    test_mapping.py       golden-file tests per frame type
    test_async_smoke.py   1000-frame burst, assert p99 producer block < 1ms
    test_integration.py   spawn real provedex-agent, run mock pipeline, verify ledger
    conftest.py           agent binary fixture (cargo build + spawn)
  examples/
    voice_agent_basic.py  full Pipecat pipeline with Provedex
```

### Component responsibilities

| Module | Responsibility | LOC estimate |
|--------|----------------|--------------|
| `processor.py` | FrameProcessor subclass, queue, worker task lifecycle, EndFrame draining | ~150 |
| `_client.py` | httpx-based async client for POST /v1/sign | ~60 |
| `mapping.py` | Pure functions Frame -> dict[str, Any] matching AgentEvent JSON shape | ~120 |
| `config.py` | ProvedexConfig dataclass, env loading | ~40 |
| `_state.py` | LLMMessagesFrame + LLMFullResponseEndFrame pairing, frame.id dedup | ~50 |

### Frame to AgentEvent mapping

| Pipecat Frame | AgentEvent variant | Fields populated |
|---------------|---------------------|------------------|
| `StartFrame` | `SessionStarted` | `agent_id`, `model_id` (both from config), `session_id` (config or uuid) |
| `EndFrame` | `SessionEnded` | `reason = "pipeline_end"`, `summary_sha256 = sha256("")` |
| `TranscriptionFrame` (final) | `UtteranceCaptured` | `audio_sha256 = sha256(transcript bytes)`, `transcript`, `lang`, `duration_ms = 0` if unknown |
| `LLMMessagesFrame` + `LLMFullResponseEndFrame` (paired) | `ModelInvoked` | `model_id` (from config or inferred), `prompt_sha256 = sha256(canonical_json(messages))`, `response_sha256 = sha256(end_frame.text)`, `prompt_tokens = 0` if unknown, `response_tokens = 0` if unknown |
| `TextFrame` (final, post-LLM, no end-frame pairing) | `UtteranceSpoken` | `text_sha256 = sha256(text)`, `text`, `audio_sha256 = sha256(b"")` |
| `FunctionCallInProgressFrame` | `ToolCalled` | `tool_name`, `args_sha256 = sha256(canonical_json(arguments))`, `args_redacted = arguments` |
| `FunctionCallResultFrame` | `ToolReturned` | `tool_name`, `result_sha256 = sha256(canonical_json(result))`, `latency_ms` (measured if start-frame timestamp captured), `success` |

Skip: `AudioRawFrame` (too high frequency), `InterimTranscriptionFrame` (not final), `MetricsFrame` (telemetry, not decision), `SystemFrame` subclasses (control flow), `LLMFullResponseStartFrame` (used internally for pairing only).

### Hashing semantics

The binding signs the bytes it sees. When raw audio is not in the binding's hands (the STT layer consumed it before the transcription frame arrived), `audio_sha256` is the sha256 of the transcript bytes, not the original audio. README documents this: it is the bytes the binding committed to. Operators who need raw-audio hashing chain a custom processor pre-STT that emits the raw-audio hash through a separate event.

### Backpressure and ordering

`process_frame` is hot path. It does:

1. Look up the frame type in the include list. If skipped, return immediately.
2. Translate frame to event dict via `mapping.py`. Pure CPU, microseconds.
3. `queue.put_nowait(event_dict)`. If full, drop the oldest item from the queue (use a custom deque-backed structure since `asyncio.Queue` does not support drop-oldest natively), increment `dropped_total`, emit a rate-limited warning.
4. Return.

Hot-path budget: < 5 ms p99 added to the frame's pass-through latency. Real number measured in `test_async_smoke.py`.

Background worker task:

1. Awaits `queue.get`.
2. POSTs to agent. 2-second timeout (configurable).
3. On success: log debug.
4. On failure (any 4xx/5xx/network/timeout): apply `on_sign_failure` policy (warn / raise / silent), increment counter, do not retry.
5. Loop.

EndFrame handling: when processor receives EndFrame, the processor sends the corresponding SessionEnded event to the queue, then stops accepting new frames and waits up to `shutdown_drain_seconds` (default 5) for the queue to drain. Forwards EndFrame downstream after drain completes (even on timeout).

### Error handling

| Failure | Behaviour |
|---------|-----------|
| Agent unreachable (ConnectionRefused) | warn + drop, `provedex_sign_dropped_total` += 1 |
| Agent slow (timeout) | warn + drop |
| Agent 4xx | log error including response body, apply on_sign_failure policy |
| Agent 5xx | warn + drop |
| Queue overflow | drop oldest, rate-limited warning (max 1 per second), `provedex_sign_overflow_total` += 1 |
| Mapping failure (unexpected frame shape) | log warning, drop event, do not raise from process_frame |

`on_sign_failure` modes:
- `warn` (default): log warning, continue.
- `raise`: raise the underlying exception out of the worker, kills the pipeline. For test environments and strict-mode customers.
- `silent`: no log, no raise. Only counters move.

### Configuration

```python
@dataclass
class ProvedexConfig:
    agent_url: str = field(default_factory=lambda: os.getenv("PROVEDEX_AGENT_URL", "http://127.0.0.1:8765"))
    session_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str = "pipecat-agent"
    model_id: str = "unknown"
    include_frames: list[type] | None = None
    on_sign_failure: Literal["warn", "raise", "silent"] = "warn"
    queue_size: int = 1000
    request_timeout_seconds: float = 2.0
    shutdown_drain_seconds: float = 5.0
```

Env-first with constructor override. `include_frames = None` means the default include list defined in `mapping.py`.

### Public API

```python
from provedex_pipecat import ProvedexFrameProcessor, ProvedexConfig

processor = ProvedexFrameProcessor(
    config=ProvedexConfig(
        agent_url="http://127.0.0.1:8765",
        session_id="optional-correlator",
        agent_id="my-voice-agent",
        model_id="llama3.2:3b",
    )
)

pipeline = Pipeline([
    transport.input(),
    stt,
    processor,
    context_aggregator.user(),
    llm,
    tts,
    transport.output(),
])
```

Single processor instance placed once. Frame ID dedup table inside the processor handles the edge case where Pipecat routes the same frame through twice. Multi-placement (one instance per pipeline stage) is supported via the dedup table.

## Tests

| Layer | Approach |
|-------|----------|
| Unit | pytest + respx (httpx mocking). Golden JSON fixtures per frame type. Stateful correlation tested with LLMMessagesFrame + LLMFullResponseEndFrame pair. |
| Async smoke | Local mock with 1 ms artificial latency, fire 1000 frames, assert producer p99 block < 1 ms and zero drops at default queue size. |
| Integration | pytest fixture: `cargo build --release -p provedex-agent`, spawn binary on random port with sandboxed ledger directory, drive a mock pipeline that emits each supported frame type, then run `provedex verify` via subprocess. Must exit 0. |

CI: add a `bindings-python` job. Python 3.11 + Rust toolchain (already pinned in `rust-toolchain.toml`). The job:
1. Builds provedex-agent in release.
2. Installs the binding with dev deps.
3. Runs pytest with `-m "not slow"` for fast tests + `-m integration` for the agent-spawn lane.

## Package metadata

| Field | Value |
|-------|-------|
| Name | `provedex-pipecat` |
| Version | `0.1.0` |
| License | Apache-2.0 |
| Python | `>= 3.11` |
| Runtime deps | `pipecat-ai >= 0.0.40, < 0.1.0`, `httpx >= 0.27`, `pydantic >= 2` |
| Dev deps | `pytest`, `pytest-asyncio`, `respx`, `ruff`, `mypy` |
| Build backend | hatchling |

## Documentation

README sections:

1. Five-line quickstart (pip install, import, instantiate, wire, run).
2. Frame mapping table (verbatim from this design).
3. Configuration reference (env vars, constructor args, failure modes).
4. Latency budget with measured numbers from `test_async_smoke.py`.
5. Failure modes: agent unreachable, agent slow, agent rate-limited.
6. Architecture note: this binding does not contain the signing primitive. The primitive is the Rust agent at github.com/provedex/provedex. The binding translates Pipecat frames into AgentEvent shapes per event-schema-v1.
7. Verifying the ledger with `provedex verify`.
8. Regulatory context paragraph: EU AI Act Article 12 (Aug 2, 2026), Colorado AI Act (Feb 1, 2026), HIPAA audit-log requirements, FINRA 2026 examination priority.

## Risks

| Risk | Mitigation |
|------|------------|
| Pipecat version churn (pre-1.0 framework) | Pin `pipecat-ai >= 0.0.40, < 0.1.0`. Add CI check. |
| TranscriptionFrame field shape differs from assumption | Inspect during implementation against real pipecat install. Adjust mapping.py before tests land. |
| Same frame routed twice through multi-placed processor | Internal `frame.id` dedup table inside processor. |
| Rust toolchain in Python CI lane adds ~5 min | Accepted per brainstorming decision; real signal worth the time. |

## Out of scope

- LangChain integration (next sprint)
- LangGraph integration (sprint after that)
- CrewAI, MCP, TypeScript / Node bindings (later)
- Frame-level batching (premature optimization)
- Custom event variants outside event-schema-v1 (would require ADR)
- Extracting `provedex-client` (deferred until second binding)
- Prometheus metrics export (counters are exposed on the processor instance for the operator to scrape if needed; no built-in exporter)

## References

- Agent HTTP API: `docs/spec/openapi.yaml`
- AgentEvent schema: `docs/spec/event-schema-v1.md`
- Canonical JSON: `docs/spec/canonical-json.md`
- Signature scheme: `docs/spec/signature-scheme.md`
- ADR-0004 sidecar-as-default: `docs/adr/0004-sidecar-as-default-integration.md`
- Pipecat docs: https://docs.pipecat.ai
- Pipecat FrameProcessor base: https://docs.pipecat.ai/server/base-classes/frame-processor
