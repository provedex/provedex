# provedex-langchain Binding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and ship a Python package `provedex-langchain` that signs every LLM call, tool call, and operator-declared session boundary inside a LangChain pipeline (and, by inheritance, LangGraph) by POSTing to the localhost `provedex-agent` sidecar. One handler class, hash-chained Ed25519-signed audit ledger as output.

**Architecture:** Single `ProvedexCallbackHandler` with dual inheritance from `AsyncCallbackHandler` and `BaseCallbackHandler`. One background worker drains a bounded `asyncio.Queue`. Callbacks paired by LangChain's `run_id` UUID for correlation. Existing event-schema-v1 variants only (no spec changes).

**Tech Stack:** Python 3.11+, hatchling build backend, httpx (async HTTP), pydantic v2 (config validation), langchain-core (handler interface), pytest + pytest-asyncio + respx (tests), ruff + mypy (lint / type).

---

## Pre-flight (already done)

- Branch: `feat/langchain-binding` (created off main, spec pushed).
- Spec: `docs/superpowers/specs/2026-05-24-provedex-langchain-binding-design.md` (approved).
- Issue: open one before Task 1 below.

## File Structure

```
bindings/python/provedex-langchain/
  pyproject.toml                       hatch + deps + tool config
  README.md                            quickstart, mapping, failure modes, regulatory context
  RELEASING.md                         PyPI publish recipe
  .gitignore                           dist/, build/, .pytest_cache, __pycache__, *.egg-info
  src/provedex_langchain/
    __init__.py                        public exports
    config.py                          ProvedexConfig (pydantic) + env loading
    _client.py                         AgentClient: httpx async POST /v1/sign
    _state.py                          CorrelationState: run_id keyed buffer + dedup
    mapping.py                         pure functions: per-callback args -> AgentEvent dict
    handler.py                         ProvedexCallbackHandler (sync + async)
  tests/
    conftest.py                        shared fixtures: respx_mock, agent_binary, agent
    test_config.py                     env loading, defaults, overrides
    test_mapping.py                    golden POST shapes per callback
    test_client.py                     AgentClient happy + error paths
    test_state.py                      run_id buffer + dedup behavior
    test_handler_sync.py               sync chain end to end with mocked agent
    test_handler_async.py              async chain end to end with mocked agent
    test_session.py                    SessionStarted / SessionEnded lifecycle paths
    test_async_smoke.py                1000-callback burst, p99 producer block < 1ms
    test_integration.py                real agent + real LangChain + real LangGraph + provedex verify
  examples/
    langchain_basic.py                 minimal LCEL pipeline with the handler
    langgraph_basic.py                 minimal LangGraph pipeline with the handler

.github/workflows/ci.yml               extend bindings-python job to cover both bindings
README.md                              add provedex-langchain row to Components table
```

---

## Task 1: File issue + scaffold the package directory

**Files:**
- Create: `bindings/python/provedex-langchain/pyproject.toml`
- Create: `bindings/python/provedex-langchain/.gitignore`
- Create: `bindings/python/provedex-langchain/src/provedex_langchain/__init__.py` (version stub)
- Create: `bindings/python/provedex-langchain/tests/__init__.py` (empty)
- Create: `bindings/python/provedex-langchain/README.md` (3-line stub, satisfies hatchling; replaced in Task 10)

- [ ] **Step 1: File tracking issue**

```bash
gh issue create --title "feat(bindings/python): provedex-langchain binding for LangChain + LangGraph" \
  --body "$(cat <<'EOF'
Second Python binding. LangChain BaseCallbackHandler + AsyncCallbackHandler that signs every LLM / tool callback via the local provedex-agent. Covers LangGraph by inheritance (LangGraph fires LangChain callbacks).

Spec: docs/superpowers/specs/2026-05-24-provedex-langchain-binding-design.md
Plan: docs/superpowers/plans/2026-05-24-provedex-langchain-binding.md
EOF
)"
```

Note the issue number for the final PR.

- [ ] **Step 2: Write pyproject.toml**

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "provedex-langchain"
version = "0.1.0"
description = "LangChain BaseCallbackHandler that signs every LLM and tool callback via the Provedex sidecar. Covers LangGraph by inheritance."
readme = "README.md"
requires-python = ">=3.11"
license = { text = "Apache-2.0" }
authors = [
    { name = "Aditya Suresh", email = "adi@provedex.io" },
]
keywords = ["langchain", "langgraph", "audit", "signing", "compliance", "ed25519", "provedex"]
classifiers = [
    "Development Status :: 4 - Beta",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: Apache Software License",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
    "Topic :: Security :: Cryptography",
]
dependencies = [
    "langchain-core>=0.3,<0.4",
    "httpx>=0.27",
    "pydantic>=2.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=8.0",
    "pytest-asyncio>=0.23",
    "respx>=0.21",
    "ruff>=0.5",
    "mypy>=1.10",
    "langchain>=0.3,<0.4",
    "langchain-openai>=0.2",
    "langgraph>=0.2",
]

[project.urls]
Homepage = "https://github.com/provedex/provedex"
Repository = "https://github.com/provedex/provedex"
Issues = "https://github.com/provedex/provedex/issues"

[tool.hatch.build.targets.wheel]
packages = ["src/provedex_langchain"]

[tool.ruff]
line-length = 100
target-version = "py311"

[tool.ruff.lint]
select = ["E", "F", "I", "B", "UP", "ASYNC"]

[tool.ruff.lint.per-file-ignores]
# Tests legitimately invoke cargo + provedex CLI via blocking subprocess.
"tests/*" = ["ASYNC221"]

[tool.mypy]
python_version = "3.11"
strict = true
ignore_missing_imports = true

[tool.pytest.ini_options]
asyncio_mode = "auto"
markers = [
    "integration: requires real provedex-agent binary",
    "slow: takes > 1s",
]
```

- [ ] **Step 3: Write .gitignore**

```
dist/
build/
*.egg-info/
__pycache__/
.pytest_cache/
.ruff_cache/
.mypy_cache/
.coverage
htmlcov/
*.pyc
```

- [ ] **Step 4: Write empty / stub files**

`src/provedex_langchain/__init__.py`:

```python
"""Provedex binding for LangChain (and LangGraph by inheritance)."""

__version__ = "0.1.0"
```

`tests/__init__.py`: empty.

`README.md` (3-line stub for hatchling):

```markdown
# provedex-langchain

LangChain callback handler that signs every LLM and tool call via the Provedex sidecar.

Full README ships in v0.1.0; for now see [docs/superpowers/specs/2026-05-24-provedex-langchain-binding-design.md](../../../docs/superpowers/specs/2026-05-24-provedex-langchain-binding-design.md).
```

- [ ] **Step 5: Verify pyproject parses**

```bash
cd bindings/python/provedex-langchain
python3 -c "import tomllib; tomllib.loads(open('pyproject.toml').read()); print('ok')"
```

Expected: `ok`.

- [ ] **Step 6: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain
git commit -m "chore(bindings/python): scaffold provedex-langchain package"
```

---

## Task 2: ProvedexConfig with env loading

**Files:**
- Create: `bindings/python/provedex-langchain/src/provedex_langchain/config.py`
- Create: `bindings/python/provedex-langchain/tests/test_config.py`

- [ ] **Step 1: Create venv + install dev deps**

```bash
cd /Users/adi/Desktop/provedex/bindings/python/provedex-langchain
python3 -m venv .venv
.venv/bin/pip install --upgrade pip
.venv/bin/pip install -e ".[dev]"
```

- [ ] **Step 2: Write failing test**

`tests/test_config.py`:

```python
import pytest
from pydantic import ValidationError

from provedex_langchain.config import ProvedexConfig


def test_defaults_with_no_env(monkeypatch):
    monkeypatch.delenv("PROVEDEX_AGENT_URL", raising=False)
    cfg = ProvedexConfig()
    assert cfg.agent_url == "http://127.0.0.1:8765"
    assert cfg.agent_id == "langchain-agent"
    assert cfg.model_id == "unknown"
    assert cfg.queue_size == 1000
    assert cfg.request_timeout_seconds == 2.0
    assert cfg.shutdown_drain_seconds == 5.0
    assert cfg.on_sign_failure == "warn"
    assert cfg.session_id
    assert cfg.include_callbacks is None


def test_env_overrides_url(monkeypatch):
    monkeypatch.setenv("PROVEDEX_AGENT_URL", "http://10.0.0.5:9999")
    cfg = ProvedexConfig()
    assert cfg.agent_url == "http://10.0.0.5:9999"


def test_constructor_overrides_env(monkeypatch):
    monkeypatch.setenv("PROVEDEX_AGENT_URL", "http://7.7.7.7:7777")
    cfg = ProvedexConfig(agent_url="http://1.2.3.4:1234")
    assert cfg.agent_url == "http://1.2.3.4:1234"


def test_on_sign_failure_invalid_rejected():
    with pytest.raises(ValidationError):
        ProvedexConfig(on_sign_failure="explode")


def test_agent_url_must_be_http():
    with pytest.raises(ValidationError):
        ProvedexConfig(agent_url="ws://127.0.0.1:8765")
```

- [ ] **Step 3: Run, confirm fails**

```bash
.venv/bin/pytest tests/test_config.py -v
```

Expected: ModuleNotFoundError on `provedex_langchain.config`.

- [ ] **Step 4: Implement config.py**

```python
"""Configuration for the Provedex LangChain binding."""

from __future__ import annotations

import os
import uuid
from typing import Literal

from pydantic import BaseModel, Field, field_validator

OnSignFailure = Literal["warn", "raise", "silent"]


class ProvedexConfig(BaseModel):
    """Configuration for ProvedexCallbackHandler.

    Env-first with constructor overrides. PROVEDEX_AGENT_URL is the only
    runtime-discovered field; everything else is set explicitly by the operator.
    """

    agent_url: str = Field(
        default_factory=lambda: os.getenv("PROVEDEX_AGENT_URL", "http://127.0.0.1:8765")
    )
    session_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str = "langchain-agent"
    model_id: str = "unknown"
    include_callbacks: list[str] | None = None
    on_sign_failure: OnSignFailure = "warn"
    queue_size: int = Field(default=1000, ge=1)
    request_timeout_seconds: float = Field(default=2.0, gt=0)
    shutdown_drain_seconds: float = Field(default=5.0, ge=0)

    @field_validator("agent_url")
    @classmethod
    def url_must_be_http(cls, v: str) -> str:
        if not v.startswith(("http://", "https://")):
            raise ValueError(f"agent_url must start with http:// or https://, got {v!r}")
        return v
```

- [ ] **Step 5: Run tests, confirm 5 pass**

```bash
.venv/bin/pytest tests/test_config.py -v
```

Expected: 5 passed.

- [ ] **Step 6: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain/src/provedex_langchain/config.py \
        bindings/python/provedex-langchain/tests/test_config.py
git commit -m "feat(langchain): ProvedexConfig with env + pydantic validation"
```

---

## Task 3: CorrelationState (run_id keyed buffer + dedup)

**Files:**
- Create: `bindings/python/provedex-langchain/src/provedex_langchain/_state.py`
- Create: `bindings/python/provedex-langchain/tests/test_state.py`

- [ ] **Step 1: Write failing test**

`tests/test_state.py`:

```python
import time
from uuid import uuid4

from provedex_langchain._state import CorrelationState


def test_initial_state_is_empty():
    s = CorrelationState()
    assert s.llm_buffer == {}
    assert s.tool_buffer == {}


def test_buffer_llm_and_take_clears():
    s = CorrelationState()
    run_id = uuid4()
    s.buffer_llm_start(run_id, model_id="gpt-4o", prompt_payload=["hi"])
    snapshot = s.take_llm(run_id)
    assert snapshot["model_id"] == "gpt-4o"
    assert snapshot["prompt_payload"] == ["hi"]
    assert "start_time" in snapshot
    assert s.take_llm(run_id) is None


def test_buffer_tool_and_take_clears():
    s = CorrelationState()
    run_id = uuid4()
    s.buffer_tool_start(run_id, tool_name="search", args={"q": "x"})
    snapshot = s.take_tool(run_id)
    assert snapshot["tool_name"] == "search"
    assert snapshot["args"] == {"q": "x"}
    assert "start_time" in snapshot
    assert s.take_tool(run_id) is None


def test_take_unknown_run_id_returns_none():
    s = CorrelationState()
    assert s.take_llm(uuid4()) is None
    assert s.take_tool(uuid4()) is None


def test_start_time_is_monotonic():
    s = CorrelationState()
    run_id = uuid4()
    before = time.monotonic()
    s.buffer_llm_start(run_id, model_id="m", prompt_payload=[])
    after = time.monotonic()
    snap = s.take_llm(run_id)
    assert before <= snap["start_time"] <= after
```

- [ ] **Step 2: Run, confirm fails**

```bash
.venv/bin/pytest tests/test_state.py -v
```

Expected: ImportError.

- [ ] **Step 3: Implement `_state.py`**

```python
"""Per-handler correlation buffer keyed on LangChain run_id."""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from typing import Any
from uuid import UUID


@dataclass
class CorrelationState:
    """Buffer in-flight LLM and tool calls keyed by LangChain run_id.

    LangChain emits paired callbacks (on_llm_start + on_llm_end, on_tool_start +
    on_tool_end) and assigns a UUID4 run_id to each pair. We buffer the start
    payload, then pair it with the end payload when the second callback fires.
    """

    llm_buffer: dict[UUID, dict[str, Any]] = field(default_factory=dict)
    tool_buffer: dict[UUID, dict[str, Any]] = field(default_factory=dict)

    def buffer_llm_start(
        self, run_id: UUID, *, model_id: str, prompt_payload: Any
    ) -> None:
        self.llm_buffer[run_id] = {
            "model_id": model_id,
            "prompt_payload": prompt_payload,
            "start_time": time.monotonic(),
        }

    def take_llm(self, run_id: UUID) -> dict[str, Any] | None:
        return self.llm_buffer.pop(run_id, None)

    def buffer_tool_start(
        self, run_id: UUID, *, tool_name: str, args: Any
    ) -> None:
        self.tool_buffer[run_id] = {
            "tool_name": tool_name,
            "args": args,
            "start_time": time.monotonic(),
        }

    def take_tool(self, run_id: UUID) -> dict[str, Any] | None:
        return self.tool_buffer.pop(run_id, None)
```

- [ ] **Step 4: Run, confirm 5 pass**

```bash
.venv/bin/pytest tests/test_state.py -v
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain/src/provedex_langchain/_state.py \
        bindings/python/provedex-langchain/tests/test_state.py
git commit -m "feat(langchain): CorrelationState run_id buffer for llm + tool pairing"
```

---

## Task 4: AgentClient async HTTP wrapper

**Files:**
- Create: `bindings/python/provedex-langchain/src/provedex_langchain/_client.py`
- Create: `bindings/python/provedex-langchain/tests/test_client.py`

Cloned from the pipecat binding's `_client.py` with identical behavior.

- [ ] **Step 1: Write failing test**

`tests/test_client.py`:

```python
import httpx
import pytest
import respx

from provedex_langchain._client import AgentClient, SignError


@pytest.fixture
def event():
    return {
        "type": "SessionStarted",
        "payload": {"agent_id": "a", "model_id": "m", "session_id": "s"},
    }


@pytest.mark.asyncio
@respx.mock
async def test_sign_happy_path(event):
    respx.post("http://127.0.0.1:8765/v1/sign").mock(
        return_value=httpx.Response(200, json={"seq": 0, "self_hash": "deadbeef"})
    )
    client = AgentClient(base_url="http://127.0.0.1:8765", timeout=2.0)
    try:
        await client.sign(event)
    finally:
        await client.aclose()


@pytest.mark.asyncio
@respx.mock
async def test_sign_400_raises(event):
    respx.post("http://127.0.0.1:8765/v1/sign").mock(
        return_value=httpx.Response(400, text="bad event")
    )
    client = AgentClient(base_url="http://127.0.0.1:8765", timeout=2.0)
    try:
        with pytest.raises(SignError) as ei:
            await client.sign(event)
        assert "400" in str(ei.value)
    finally:
        await client.aclose()


@pytest.mark.asyncio
@respx.mock
async def test_sign_connection_error_raises(event):
    respx.post("http://127.0.0.1:8765/v1/sign").mock(
        side_effect=httpx.ConnectError("refused")
    )
    client = AgentClient(base_url="http://127.0.0.1:8765", timeout=2.0)
    try:
        with pytest.raises(SignError):
            await client.sign(event)
    finally:
        await client.aclose()
```

- [ ] **Step 2: Run, confirm fails**

```bash
.venv/bin/pytest tests/test_client.py -v
```

Expected: ImportError.

- [ ] **Step 3: Implement `_client.py`**

```python
"""Private async HTTP client for the provedex-agent /v1/sign endpoint."""

from __future__ import annotations

from typing import Any

import httpx


class SignError(Exception):
    """Raised when a sign attempt fails (network, timeout, or non-2xx)."""


class AgentClient:
    """Thin httpx wrapper. One per handler instance; reuses the connection."""

    def __init__(self, base_url: str, timeout: float) -> None:
        self._base_url = base_url.rstrip("/")
        self._client = httpx.AsyncClient(
            base_url=self._base_url,
            timeout=httpx.Timeout(timeout, connect=timeout),
            headers={"content-type": "application/json"},
        )

    async def sign(self, event: dict[str, Any]) -> None:
        """POST {event: ...} to /v1/sign. Raises SignError on any failure."""
        try:
            resp = await self._client.post("/v1/sign", json={"event": event})
        except httpx.HTTPError as e:
            raise SignError(f"agent unreachable: {e}") from e
        if resp.status_code >= 400:
            raise SignError(f"agent returned {resp.status_code}: {resp.text[:200]}")

    async def aclose(self) -> None:
        await self._client.aclose()
```

- [ ] **Step 4: Run tests**

```bash
.venv/bin/pytest tests/test_client.py -v
```

Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain/src/provedex_langchain/_client.py \
        bindings/python/provedex-langchain/tests/test_client.py
git commit -m "feat(langchain): AgentClient httpx wrapper for /v1/sign"
```

---

## Task 5: mapping.py - per-callback to AgentEvent translators

**Files:**
- Create: `bindings/python/provedex-langchain/src/provedex_langchain/mapping.py`
- Create: `bindings/python/provedex-langchain/tests/test_mapping.py`

The mapping module exposes builder functions. Each returns a dict matching `{"type": <VariantName>, "payload": {...}}`.

- [ ] **Step 1: Write failing test**

`tests/test_mapping.py`:

```python
import hashlib
import json

from provedex_langchain.config import ProvedexConfig
from provedex_langchain.mapping import (
    session_started_event,
    session_ended_event,
    model_invoked_event,
    tool_called_event,
    tool_returned_event,
)


def _sha256_hex(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _canonical_json(value) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def _config():
    return ProvedexConfig(
        agent_url="http://127.0.0.1:8765",
        session_id="test-session",
        agent_id="test-agent",
        model_id="test-model",
    )


def test_session_started_event():
    cfg = _config()
    event = session_started_event(cfg)
    assert event == {
        "type": "SessionStarted",
        "payload": {
            "agent_id": "test-agent",
            "model_id": "test-model",
            "session_id": "test-session",
        },
    }


def test_session_ended_event():
    event = session_ended_event(reason="done")
    assert event == {
        "type": "SessionEnded",
        "payload": {
            "reason": "done",
            "summary_sha256": _sha256_hex(b""),
        },
    }


def test_model_invoked_event_with_token_counts():
    event = model_invoked_event(
        model_id="gpt-4o",
        prompt_payload=["hello"],
        response_text="hi there",
        prompt_tokens=5,
        response_tokens=2,
    )
    assert event["type"] == "ModelInvoked"
    assert event["payload"]["model_id"] == "gpt-4o"
    assert event["payload"]["prompt_sha256"] == _sha256_hex(_canonical_json(["hello"]))
    assert event["payload"]["response_sha256"] == _sha256_hex(b"hi there")
    assert event["payload"]["prompt_tokens"] == 5
    assert event["payload"]["response_tokens"] == 2


def test_model_invoked_event_defaults_token_counts_to_zero():
    event = model_invoked_event(
        model_id="gpt-4o",
        prompt_payload=["hello"],
        response_text="hi",
        prompt_tokens=None,
        response_tokens=None,
    )
    assert event["payload"]["prompt_tokens"] == 0
    assert event["payload"]["response_tokens"] == 0


def test_tool_called_event_with_dict_args():
    event = tool_called_event(tool_name="search", args={"q": "x"})
    assert event["type"] == "ToolCalled"
    assert event["payload"]["tool_name"] == "search"
    assert event["payload"]["args_redacted"] == {"q": "x"}
    assert event["payload"]["args_sha256"] == _sha256_hex(_canonical_json({"q": "x"}))


def test_tool_called_event_with_string_args():
    event = tool_called_event(tool_name="search", args="q=x")
    assert event["payload"]["args_redacted"] == {"input": "q=x"}
    assert event["payload"]["args_sha256"] == _sha256_hex(_canonical_json({"input": "q=x"}))


def test_tool_returned_event_success():
    event = tool_returned_event(
        tool_name="search",
        result={"hits": 3},
        latency_ms=42,
        success=True,
    )
    assert event["type"] == "ToolReturned"
    assert event["payload"]["tool_name"] == "search"
    assert event["payload"]["result_sha256"] == _sha256_hex(_canonical_json({"hits": 3}))
    assert event["payload"]["latency_ms"] == 42
    assert event["payload"]["success"] is True


def test_tool_returned_event_failure():
    event = tool_returned_event(
        tool_name="search",
        result="RuntimeError: boom",
        latency_ms=10,
        success=False,
    )
    assert event["payload"]["success"] is False
    assert event["payload"]["result_sha256"] == _sha256_hex(_canonical_json("RuntimeError: boom"))
```

- [ ] **Step 2: Run, confirm fails**

```bash
.venv/bin/pytest tests/test_mapping.py -v
```

Expected: ImportError on `provedex_langchain.mapping`.

- [ ] **Step 3: Implement `mapping.py`**

```python
"""Pure functions translating LangChain callback args into AgentEvent dicts.

The output shape matches docs/spec/event-schema-v1.md:
    { "type": "<VariantName>", "payload": { ... } }
"""

from __future__ import annotations

import hashlib
import json
from typing import Any

from .config import ProvedexConfig


def _sha256_hex(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _canonical_json_bytes(value: Any) -> bytes:
    """Compact JSON with sorted keys. The actual cryptographic canonical JSON
    lives in the Rust agent; this hash is the binding's receipt of what it sent.
    """
    return json.dumps(value, sort_keys=True, separators=(",", ":"), default=str).encode()


def session_started_event(config: ProvedexConfig) -> dict[str, Any]:
    return {
        "type": "SessionStarted",
        "payload": {
            "agent_id": config.agent_id,
            "model_id": config.model_id,
            "session_id": config.session_id,
        },
    }


def session_ended_event(reason: str) -> dict[str, Any]:
    return {
        "type": "SessionEnded",
        "payload": {
            "reason": reason,
            "summary_sha256": _sha256_hex(b""),
        },
    }


def model_invoked_event(
    *,
    model_id: str,
    prompt_payload: Any,
    response_text: str,
    prompt_tokens: int | None,
    response_tokens: int | None,
) -> dict[str, Any]:
    return {
        "type": "ModelInvoked",
        "payload": {
            "model_id": model_id,
            "prompt_sha256": _sha256_hex(_canonical_json_bytes(prompt_payload)),
            "response_sha256": _sha256_hex(response_text.encode()),
            "prompt_tokens": prompt_tokens if prompt_tokens is not None else 0,
            "response_tokens": response_tokens if response_tokens is not None else 0,
        },
    }


def tool_called_event(*, tool_name: str, args: Any) -> dict[str, Any]:
    args_redacted: Any
    if isinstance(args, dict):
        args_redacted = args
    else:
        args_redacted = {"input": str(args)}
    return {
        "type": "ToolCalled",
        "payload": {
            "tool_name": tool_name,
            "args_sha256": _sha256_hex(_canonical_json_bytes(args_redacted)),
            "args_redacted": args_redacted,
        },
    }


def tool_returned_event(
    *, tool_name: str, result: Any, latency_ms: int, success: bool
) -> dict[str, Any]:
    return {
        "type": "ToolReturned",
        "payload": {
            "tool_name": tool_name,
            "result_sha256": _sha256_hex(_canonical_json_bytes(result)),
            "latency_ms": latency_ms,
            "success": success,
        },
    }
```

- [ ] **Step 4: Run, confirm 8 pass**

```bash
.venv/bin/pytest tests/test_mapping.py -v
```

Expected: 8 passed.

- [ ] **Step 5: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain/src/provedex_langchain/mapping.py \
        bindings/python/provedex-langchain/tests/test_mapping.py
git commit -m "feat(langchain): callback -> AgentEvent mapping (pure functions)"
```

---

## Task 6: ProvedexCallbackHandler (dual inheritance, biggest task)

**Files:**
- Create: `bindings/python/provedex-langchain/src/provedex_langchain/handler.py`
- Modify: `bindings/python/provedex-langchain/src/provedex_langchain/__init__.py`
- Create: `bindings/python/provedex-langchain/tests/test_handler_sync.py`
- Create: `bindings/python/provedex-langchain/tests/test_handler_async.py`

- [ ] **Step 1: Write failing sync handler test**

`tests/test_handler_sync.py`:

```python
from collections import Counter

import httpx
import pytest
import respx
from uuid import uuid4

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig


@pytest.mark.asyncio
@respx.mock
async def test_sync_llm_start_end_emits_model_invoked():
    posted = []

    def record(request):
        posted.append(request.json())
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=record)

    handler = ProvedexCallbackHandler(config=ProvedexConfig(model_id="gpt-4o"))
    await handler.start()

    run_id = uuid4()
    handler.on_llm_start(
        serialized={"id": ["langchain", "llms", "openai", "gpt-4o"]},
        prompts=["hello"],
        run_id=run_id,
    )

    from langchain_core.outputs import Generation, LLMResult

    handler.on_llm_end(
        LLMResult(
            generations=[[Generation(text="hi there")]],
            llm_output={"token_usage": {"prompt_tokens": 5, "completion_tokens": 2}},
        ),
        run_id=run_id,
    )

    await handler.stop()

    types = Counter(body["event"]["type"] for body in posted)
    assert types["ModelInvoked"] == 1
    payload = next(body for body in posted if body["event"]["type"] == "ModelInvoked")["event"][
        "payload"
    ]
    assert payload["prompt_tokens"] == 5
    assert payload["response_tokens"] == 2


@pytest.mark.asyncio
@respx.mock
async def test_sync_tool_start_end_emits_called_and_returned():
    posted = []

    def record(request):
        posted.append(request.json())
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=record)

    handler = ProvedexCallbackHandler(config=ProvedexConfig())
    await handler.start()

    run_id = uuid4()
    handler.on_tool_start(
        serialized={"name": "search"},
        input_str='{"q": "x"}',
        run_id=run_id,
        inputs={"q": "x"},
    )
    handler.on_tool_end(output='{"hits": 3}', run_id=run_id)

    await handler.stop()

    types = Counter(body["event"]["type"] for body in posted)
    assert types["ToolCalled"] == 1
    assert types["ToolReturned"] == 1


@pytest.mark.asyncio
@respx.mock
async def test_sync_tool_error_emits_returned_with_success_false():
    posted = []

    def record(request):
        posted.append(request.json())
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=record)

    handler = ProvedexCallbackHandler(config=ProvedexConfig())
    await handler.start()

    run_id = uuid4()
    handler.on_tool_start(
        serialized={"name": "search"},
        input_str="q=x",
        run_id=run_id,
    )
    handler.on_tool_error(RuntimeError("boom"), run_id=run_id)

    await handler.stop()

    returned = next(body for body in posted if body["event"]["type"] == "ToolReturned")["event"][
        "payload"
    ]
    assert returned["success"] is False
```

- [ ] **Step 2: Write failing async handler test**

`tests/test_handler_async.py`:

```python
from collections import Counter
from uuid import uuid4

import httpx
import pytest
import respx
from langchain_core.outputs import Generation, LLMResult

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig


@pytest.mark.asyncio
@respx.mock
async def test_async_llm_callbacks_emit_model_invoked():
    posted = []

    def record(request):
        posted.append(request.json())
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=record)

    handler = ProvedexCallbackHandler(config=ProvedexConfig(model_id="llama3"))
    await handler.start()

    run_id = uuid4()
    await handler.on_llm_start(
        serialized={"id": ["langchain", "llms", "ollama", "llama3"]},
        prompts=["hello"],
        run_id=run_id,
    )
    await handler.on_llm_end(
        LLMResult(generations=[[Generation(text="hi")]], llm_output=None),
        run_id=run_id,
    )

    await handler.stop()

    types = Counter(body["event"]["type"] for body in posted)
    assert types["ModelInvoked"] == 1


@pytest.mark.asyncio
@respx.mock
async def test_async_drops_when_agent_unreachable():
    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=httpx.ConnectError("refused"))

    handler = ProvedexCallbackHandler(config=ProvedexConfig(on_sign_failure="warn"))
    await handler.start()

    run_id = uuid4()
    await handler.on_tool_start(
        serialized={"name": "search"}, input_str="q", run_id=run_id
    )
    await handler.on_tool_end(output="ok", run_id=run_id)

    await handler.stop()
    assert handler.dropped_total >= 1
```

LangChain note: `BaseCallbackHandler.on_llm_start` is sync; `AsyncCallbackHandler.on_llm_start` is async with the same name. Python's MRO resolves the async version when called with `await` and the sync version when called without. The handler implements both - the async one calls `await self._enqueue_async(...)`, the sync one calls `self._enqueue_sync(...)`. Internally both append to the same deque.

- [ ] **Step 3: Run, confirm fails**

```bash
.venv/bin/pytest tests/test_handler_sync.py tests/test_handler_async.py -v
```

Expected: ImportError on `ProvedexCallbackHandler`.

- [ ] **Step 4: Implement `handler.py`**

```python
"""ProvedexCallbackHandler: signs LangChain LLM / tool callbacks via the agent."""

from __future__ import annotations

import asyncio
import logging
import time
from collections import deque
from typing import Any
from uuid import UUID

from langchain_core.callbacks import AsyncCallbackHandler, BaseCallbackHandler
from langchain_core.messages import BaseMessage
from langchain_core.outputs import LLMResult

from ._client import AgentClient, SignError
from ._state import CorrelationState
from .config import ProvedexConfig
from .mapping import (
    model_invoked_event,
    session_ended_event,
    session_started_event,
    tool_called_event,
    tool_returned_event,
)

logger = logging.getLogger(__name__)


class ProvedexCallbackHandler(AsyncCallbackHandler, BaseCallbackHandler):
    """LangChain callback handler that signs every LLM and tool call.

    Implements both sync (`on_*`) and async (`aon_*` via subclass) callback
    interfaces. Both delegate to the same enqueue path. A single background
    asyncio worker drains a bounded deque and POSTs to /v1/sign.

    Session lifecycle is operator-driven via start_session() / end_session()
    or the session() context manager. No auto-derivation from chain hierarchy.
    """

    raise_error: bool = False  # Required by LangChain to surface raise mode.

    def __init__(self, *, config: ProvedexConfig) -> None:
        super().__init__()
        self._config = config
        self._client = AgentClient(
            base_url=config.agent_url,
            timeout=config.request_timeout_seconds,
        )
        self._state = CorrelationState()
        self._queue: deque[dict[str, Any]] = deque(maxlen=config.queue_size)
        self._wakeup = asyncio.Event()
        self._worker_task: asyncio.Task | None = None
        self._stopping = False
        self._last_overflow_warn_ts: float = 0.0

        self.signed_total = 0
        self.dropped_total = 0
        self.overflow_total = 0

    # --- worker lifecycle -------------------------------------------------

    async def start(self) -> None:
        """Start the background worker. Idempotent."""
        if self._worker_task is None:
            self._worker_task = asyncio.create_task(self._run_worker())

    async def stop(self) -> None:
        """Drain the queue (up to shutdown_drain_seconds) and stop the worker."""
        self._stopping = True
        self._wakeup.set()
        if self._worker_task is not None:
            try:
                await asyncio.wait_for(
                    self._worker_task,
                    timeout=self._config.shutdown_drain_seconds,
                )
            except TimeoutError:
                self._worker_task.cancel()
        await self._client.aclose()

    # --- session lifecycle ------------------------------------------------

    def start_session(self) -> None:
        """Emit a SessionStarted event."""
        self._enqueue(session_started_event(self._config))

    def end_session(self, reason: str = "operator_end") -> None:
        """Emit a SessionEnded event."""
        self._enqueue(session_ended_event(reason=reason))

    def session(self, reason: str = "operator_session"):
        """Context manager: starts a session on enter, ends on exit.

        Sync and async usage both supported via _SessionContext.
        """
        return _SessionContext(self, reason)

    # --- sync callbacks (BaseCallbackHandler) -----------------------------

    def on_llm_start(
        self,
        serialized: dict[str, Any],
        prompts: list[str],
        *,
        run_id: UUID,
        **kwargs: Any,
    ) -> None:
        model_id = self._derive_model_id(serialized)
        self._state.buffer_llm_start(
            run_id, model_id=model_id, prompt_payload=prompts
        )

    def on_chat_model_start(
        self,
        serialized: dict[str, Any],
        messages: list[list[BaseMessage]],
        *,
        run_id: UUID,
        **kwargs: Any,
    ) -> None:
        model_id = self._derive_model_id(serialized)
        flattened = [
            {"type": m.type, "content": m.content}
            for batch in messages
            for m in batch
        ]
        self._state.buffer_llm_start(
            run_id, model_id=model_id, prompt_payload=flattened
        )

    def on_llm_end(
        self, response: LLMResult, *, run_id: UUID, **kwargs: Any
    ) -> None:
        self._emit_model_invoked(run_id, response)

    def on_llm_error(
        self, error: BaseException, *, run_id: UUID, **kwargs: Any
    ) -> None:
        self._emit_model_invoked_error(run_id, error)

    def on_tool_start(
        self,
        serialized: dict[str, Any],
        input_str: str,
        *,
        run_id: UUID,
        inputs: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> None:
        tool_name = serialized.get("name", "unknown")
        args = inputs if inputs is not None else input_str
        self._state.buffer_tool_start(run_id, tool_name=tool_name, args=args)
        self._enqueue(tool_called_event(tool_name=tool_name, args=args))

    def on_tool_end(self, output: Any, *, run_id: UUID, **kwargs: Any) -> None:
        self._emit_tool_returned(run_id, output, success=True)

    def on_tool_error(
        self, error: BaseException, *, run_id: UUID, **kwargs: Any
    ) -> None:
        description = f"{type(error).__name__}: {error}"
        self._emit_tool_returned(run_id, description, success=False)

    # --- async callbacks (AsyncCallbackHandler overrides) -----------------

    async def aon_llm_start(self, *args: Any, **kwargs: Any) -> None:
        self.on_llm_start(*args, **kwargs)

    async def aon_chat_model_start(self, *args: Any, **kwargs: Any) -> None:
        self.on_chat_model_start(*args, **kwargs)

    async def aon_llm_end(self, *args: Any, **kwargs: Any) -> None:
        self.on_llm_end(*args, **kwargs)

    async def aon_llm_error(self, *args: Any, **kwargs: Any) -> None:
        self.on_llm_error(*args, **kwargs)

    async def aon_tool_start(self, *args: Any, **kwargs: Any) -> None:
        self.on_tool_start(*args, **kwargs)

    async def aon_tool_end(self, *args: Any, **kwargs: Any) -> None:
        self.on_tool_end(*args, **kwargs)

    async def aon_tool_error(self, *args: Any, **kwargs: Any) -> None:
        self.on_tool_error(*args, **kwargs)

    # --- helpers ----------------------------------------------------------

    def _derive_model_id(self, serialized: dict[str, Any]) -> str:
        """LangChain's `serialized.id` is a path list ending in the class name.
        Fall back to the configured default if missing.
        """
        path = serialized.get("id")
        if isinstance(path, list) and path:
            return path[-1]
        return self._config.model_id

    def _emit_model_invoked(self, run_id: UUID, response: LLMResult) -> None:
        snap = self._state.take_llm(run_id)
        if snap is None:
            logger.warning("on_llm_end without prior on_llm_start (run_id=%s)", run_id)
            return
        try:
            response_text = response.generations[0][0].text
        except (IndexError, AttributeError):
            response_text = ""
        token_usage = (response.llm_output or {}).get("token_usage", {}) or {}
        prompt_tokens = token_usage.get("prompt_tokens")
        response_tokens = token_usage.get("completion_tokens") or token_usage.get(
            "response_tokens"
        )
        self._enqueue(
            model_invoked_event(
                model_id=snap["model_id"],
                prompt_payload=snap["prompt_payload"],
                response_text=response_text,
                prompt_tokens=prompt_tokens,
                response_tokens=response_tokens,
            )
        )

    def _emit_model_invoked_error(self, run_id: UUID, error: BaseException) -> None:
        snap = self._state.take_llm(run_id)
        if snap is None:
            return
        description = f"{type(error).__name__}: {error}"
        self._enqueue(
            model_invoked_event(
                model_id=snap["model_id"],
                prompt_payload=snap["prompt_payload"],
                response_text=description,
                prompt_tokens=None,
                response_tokens=None,
            )
        )

    def _emit_tool_returned(
        self, run_id: UUID, result: Any, *, success: bool
    ) -> None:
        snap = self._state.take_tool(run_id)
        if snap is None:
            logger.warning("on_tool_end without prior on_tool_start (run_id=%s)", run_id)
            return
        latency_ms = int((time.monotonic() - snap["start_time"]) * 1000)
        self._enqueue(
            tool_returned_event(
                tool_name=snap["tool_name"],
                result=result,
                latency_ms=latency_ms,
                success=success,
            )
        )

    # --- queue + worker ---------------------------------------------------

    def _enqueue(self, event: dict[str, Any]) -> None:
        if len(self._queue) >= self._config.queue_size:
            self.overflow_total += 1
            now = time.monotonic()
            if now - self._last_overflow_warn_ts > 1.0:
                self._last_overflow_warn_ts = now
                logger.warning(
                    "provedex sign queue overflow (total=%d); dropping oldest",
                    self.overflow_total,
                )
        self._queue.append(event)
        self._wakeup.set()

    async def _run_worker(self) -> None:
        while True:
            if not self._queue:
                if self._stopping:
                    return
                self._wakeup.clear()
                try:
                    await asyncio.wait_for(self._wakeup.wait(), timeout=0.1)
                except TimeoutError:
                    continue
                continue

            event = self._queue.popleft()
            try:
                await self._client.sign(event)
                self.signed_total += 1
            except SignError as e:
                if self._config.on_sign_failure == "raise":
                    logger.error(
                        "provedex sign failed (raise mode), worker stopping: %s", e
                    )
                    raise
                self.dropped_total += 1
                if self._config.on_sign_failure == "warn":
                    logger.warning(
                        "provedex sign failed for %s: %s",
                        event.get("type", "<unknown>"),
                        e,
                    )


class _SessionContext:
    """Dual sync / async context manager produced by handler.session()."""

    def __init__(self, handler: ProvedexCallbackHandler, reason: str) -> None:
        self._handler = handler
        self._reason = reason

    def __enter__(self) -> ProvedexCallbackHandler:
        self._handler.start_session()
        return self._handler

    def __exit__(self, exc_type, exc, tb) -> None:
        reason = self._reason if exc is None else f"exception:{exc_type.__name__}"
        self._handler.end_session(reason=reason)

    async def __aenter__(self) -> ProvedexCallbackHandler:
        await self._handler.start()
        self._handler.start_session()
        return self._handler

    async def __aexit__(self, exc_type, exc, tb) -> None:
        reason = self._reason if exc is None else f"exception:{exc_type.__name__}"
        self._handler.end_session(reason=reason)
```

- [ ] **Step 5: Update `__init__.py`**

```python
"""Provedex binding for LangChain (and LangGraph by inheritance)."""

from .config import ProvedexConfig
from .handler import ProvedexCallbackHandler

__version__ = "0.1.0"
__all__ = ["ProvedexCallbackHandler", "ProvedexConfig"]
```

- [ ] **Step 6: Run handler tests**

```bash
.venv/bin/pytest tests/test_handler_sync.py tests/test_handler_async.py -v
```

Expected: all 5 pass.

- [ ] **Step 7: Run full suite so far**

```bash
.venv/bin/pytest -v
```

Expected: 26 pass (5 config + 5 state + 3 client + 8 mapping + 3 sync handler + 2 async handler).

- [ ] **Step 8: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain/src/provedex_langchain/handler.py \
        bindings/python/provedex-langchain/src/provedex_langchain/__init__.py \
        bindings/python/provedex-langchain/tests/test_handler_sync.py \
        bindings/python/provedex-langchain/tests/test_handler_async.py
git commit -m "feat(langchain): ProvedexCallbackHandler with dual sync + async coverage"
```

---

## Task 7: Session context manager + lifecycle tests

**Files:**
- Create: `bindings/python/provedex-langchain/tests/test_session.py`

The `session` context manager is already implemented in Task 6's handler.py. This task tests the four paths: sync normal, sync exception, async normal, async exception.

- [ ] **Step 1: Write test**

```python
from collections import Counter

import httpx
import pytest
import respx

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig


def _capture():
    posted = []

    def record(request):
        posted.append(request.json())
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    return posted, record


@pytest.mark.asyncio
@respx.mock
async def test_sync_session_normal_exit():
    posted, record = _capture()
    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=record)

    handler = ProvedexCallbackHandler(config=ProvedexConfig())
    await handler.start()

    with handler.session("test-run"):
        pass

    await handler.stop()

    types = [body["event"]["type"] for body in posted]
    assert types == ["SessionStarted", "SessionEnded"]
    end_reason = posted[-1]["event"]["payload"]["reason"]
    assert end_reason == "test-run"


@pytest.mark.asyncio
@respx.mock
async def test_sync_session_exception_records_reason():
    posted, record = _capture()
    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=record)

    handler = ProvedexCallbackHandler(config=ProvedexConfig())
    await handler.start()

    with pytest.raises(RuntimeError):
        with handler.session("test-run"):
            raise RuntimeError("boom")

    await handler.stop()

    end_reason = posted[-1]["event"]["payload"]["reason"]
    assert "RuntimeError" in end_reason


@pytest.mark.asyncio
@respx.mock
async def test_async_session_normal_exit():
    posted, record = _capture()
    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=record)

    handler = ProvedexCallbackHandler(config=ProvedexConfig())

    async with handler.session("async-run"):
        pass

    await handler.stop()

    types = [body["event"]["type"] for body in posted]
    assert types == ["SessionStarted", "SessionEnded"]


@pytest.mark.asyncio
@respx.mock
async def test_async_session_exception_records_reason():
    posted, record = _capture()
    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=record)

    handler = ProvedexCallbackHandler(config=ProvedexConfig())

    with pytest.raises(ValueError):
        async with handler.session("async-run"):
            raise ValueError("bad")

    await handler.stop()

    end_reason = posted[-1]["event"]["payload"]["reason"]
    assert "ValueError" in end_reason
```

- [ ] **Step 2: Run**

```bash
.venv/bin/pytest tests/test_session.py -v
```

Expected: 4 passed.

- [ ] **Step 3: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain/tests/test_session.py
git commit -m "test(langchain): session context manager (sync + async, normal + exception)"
```

---

## Task 8: Async smoke test for producer latency budget

**Files:**
- Create: `bindings/python/provedex-langchain/tests/test_async_smoke.py`

- [ ] **Step 1: Write test**

```python
import asyncio
import statistics
import time
from uuid import uuid4

import httpx
import pytest
import respx
from langchain_core.outputs import Generation, LLMResult

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig


@pytest.mark.slow
@pytest.mark.asyncio
@respx.mock
async def test_producer_block_p99_under_one_ms():
    async def slow_responder(request):
        await asyncio.sleep(0.001)
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=slow_responder)

    handler = ProvedexCallbackHandler(config=ProvedexConfig(queue_size=2000))
    await handler.start()

    response = LLMResult(generations=[[Generation(text="ok")]], llm_output=None)
    blocks_us: list[float] = []
    for _ in range(1000):
        run_id = uuid4()
        t0 = time.perf_counter()
        handler.on_llm_start(
            serialized={"id": ["langchain", "llms", "openai", "gpt-4o"]},
            prompts=["x"],
            run_id=run_id,
        )
        handler.on_llm_end(response, run_id=run_id)
        blocks_us.append((time.perf_counter() - t0) * 1_000_000)

    await handler.stop()

    p50 = statistics.median(blocks_us)
    p99 = sorted(blocks_us)[int(0.99 * len(blocks_us))]
    print(f"\n  producer block (start+end pair): p50={p50:.1f}us p99={p99:.1f}us")
    assert p99 < 1000, f"p99 {p99:.1f}us exceeds 1ms budget"


@pytest.mark.slow
@pytest.mark.asyncio
@respx.mock
async def test_zero_drops_at_default_queue_with_steady_load():
    async def fast_responder(request):
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=fast_responder)

    handler = ProvedexCallbackHandler(config=ProvedexConfig(queue_size=1000))
    await handler.start()

    response = LLMResult(generations=[[Generation(text="ok")]], llm_output=None)
    for i in range(500):
        run_id = uuid4()
        handler.on_llm_start(
            serialized={"id": ["langchain", "llms", "openai", "gpt-4o"]},
            prompts=["x"],
            run_id=run_id,
        )
        handler.on_llm_end(response, run_id=run_id)
        if i % 100 == 0:
            await asyncio.sleep(0.01)

    await handler.stop()
    assert handler.overflow_total == 0
```

- [ ] **Step 2: Run**

```bash
.venv/bin/pytest tests/test_async_smoke.py -v -s -m slow
```

Expected: 2 passed. Numbers printed.

- [ ] **Step 3: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain/tests/test_async_smoke.py
git commit -m "test(langchain): async smoke test for producer latency budget"
```

---

## Task 9: Integration test with real agent + LangChain + LangGraph

**Files:**
- Create: `bindings/python/provedex-langchain/tests/conftest.py`
- Create: `bindings/python/provedex-langchain/tests/test_integration.py`

- [ ] **Step 1: Write conftest.py (agent fixture)**

```python
import os
import socket
import subprocess
import time
from pathlib import Path

import httpx
import pytest

# tests -> provedex-langchain -> python -> bindings -> repo
REPO_ROOT = Path(__file__).resolve().parents[4]


def _free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@pytest.fixture(scope="session")
def agent_binary() -> Path:
    """Build provedex-agent in release once per test session."""
    target = REPO_ROOT / "target" / "release" / "provedex-agent"
    if not target.exists():
        subprocess.run(
            ["cargo", "build", "--release", "-p", "provedex-agent"],
            cwd=REPO_ROOT,
            check=True,
        )
    return target


@pytest.fixture
def agent(agent_binary, tmp_path):
    """Spawn provedex-agent on a random port with a sandboxed ledger."""
    port = _free_port()
    ledger = tmp_path / "ledger.ndjson"
    key = tmp_path / "ed25519.key"

    env = os.environ.copy()
    env.update(
        {
            "PROVEDEX_LEDGER": str(ledger),
            "PROVEDEX_KEY": str(key),
            "PROVEDEX_AGENT_LISTEN": f"127.0.0.1:{port}",
            "RUST_LOG": "warn",
        }
    )
    proc = subprocess.Popen(
        [str(agent_binary), "--rate-limit-off"],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    base_url = f"http://127.0.0.1:{port}"
    for _ in range(50):
        try:
            r = httpx.get(f"{base_url}/v1/healthz", timeout=0.5)
            if r.status_code == 200:
                break
        except httpx.HTTPError:
            pass
        time.sleep(0.1)
    else:
        proc.kill()
        out, err = proc.communicate()
        raise RuntimeError(f"agent failed to start: {err.decode()[:500]}")

    yield {"base_url": base_url, "ledger": ledger}

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
```

- [ ] **Step 2: Write integration test (LangChain + LangGraph pipelines)**

`tests/test_integration.py`:

```python
import subprocess
from pathlib import Path

import pytest
from langchain_core.language_models.fake_chat_models import FakeListChatModel
from langchain_core.prompts import ChatPromptTemplate

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig

REPO_ROOT = Path(__file__).resolve().parents[4]


def _provedex_cli() -> Path:
    cli = REPO_ROOT / "target" / "release" / "provedex"
    if not cli.exists():
        subprocess.run(
            ["cargo", "build", "--release", "-p", "provedex-cli"],
            cwd=REPO_ROOT,
            check=True,
        )
    return cli


@pytest.mark.integration
@pytest.mark.asyncio
async def test_langchain_pipeline_produces_valid_ledger(agent):
    cfg = ProvedexConfig(
        agent_url=agent["base_url"],
        session_id="int-langchain",
        agent_id="int-langchain-agent",
        model_id="fake-list",
    )
    handler = ProvedexCallbackHandler(config=cfg)

    llm = FakeListChatModel(responses=["hi there"])
    prompt = ChatPromptTemplate.from_template("Say hi: {topic}")
    chain = prompt | llm

    async with handler.session("test-request"):
        await chain.ainvoke({"topic": "ducks"}, config={"callbacks": [handler]})

    result = subprocess.run(
        [str(_provedex_cli()), "verify", "--ledger", str(agent["ledger"])],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, (
        f"provedex verify failed: stdout={result.stdout} stderr={result.stderr}"
    )
    assert handler.signed_total >= 3  # SessionStarted, ModelInvoked, SessionEnded
    assert handler.dropped_total == 0


@pytest.mark.integration
@pytest.mark.asyncio
async def test_langgraph_pipeline_produces_valid_ledger(agent):
    from langgraph.graph import END, START, StateGraph
    from typing_extensions import TypedDict

    class State(TypedDict):
        answer: str

    llm = FakeListChatModel(responses=["graph response"])

    async def node_a(state: State, config) -> State:
        resp = await llm.ainvoke("hi", config=config)
        return {"answer": resp.content}

    graph_builder = StateGraph(State)
    graph_builder.add_node("a", node_a)
    graph_builder.add_edge(START, "a")
    graph_builder.add_edge("a", END)
    graph = graph_builder.compile()

    cfg = ProvedexConfig(
        agent_url=agent["base_url"],
        session_id="int-langgraph",
        agent_id="int-langgraph-agent",
        model_id="fake-list",
    )
    handler = ProvedexCallbackHandler(config=cfg)

    async with handler.session("graph-run"):
        await graph.ainvoke({"answer": ""}, config={"callbacks": [handler]})

    result = subprocess.run(
        [str(_provedex_cli()), "verify", "--ledger", str(agent["ledger"])],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0
    assert handler.signed_total >= 3
    assert handler.dropped_total == 0
```

- [ ] **Step 3: Run integration tests**

```bash
.venv/bin/pytest tests/test_integration.py -v -m integration -s
```

Expected: 2 passed. First run builds the agent + CLI (~5 min).

- [ ] **Step 4: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain/tests/conftest.py \
        bindings/python/provedex-langchain/tests/test_integration.py
git commit -m "test(langchain): integration - real agent + LangChain + LangGraph + verify"
```

---

## Task 10: README + examples

**Files:**
- Modify (replace stub): `bindings/python/provedex-langchain/README.md`
- Create: `bindings/python/provedex-langchain/examples/langchain_basic.py`
- Create: `bindings/python/provedex-langchain/examples/langgraph_basic.py`

- [ ] **Step 1: Replace README.md**

Match the structure of the pipecat README. 9 sections (see spec). Plain ASCII, no AI-slop. Use the producer p50 / p99 numbers from Task 8's smoke test output for the latency budget section.

The reference structure: title + paragraph what + why, quickstart (5-line code block with pip install + chain.invoke), callback mapping table, configuration reference, session lifecycle, latency budget (with measured numbers), failure modes table, LangGraph note, architecture note, verifying the ledger, regulatory context.

Write 600-900 words. Run after writing:

```bash
LC_ALL=C grep -nP '[^\x00-\x7F]' bindings/python/provedex-langchain/README.md && echo FAIL || echo ascii
grep -niE '\b(robust|comprehensive|powerful|elegant|leveraging|cutting-edge|next-gen|seamless)\b' bindings/python/provedex-langchain/README.md || echo no-slop
```

Both should pass.

- [ ] **Step 2: Write `examples/langchain_basic.py`**

```python
"""Minimal LangChain LCEL pipeline with Provedex signing.

Run a local provedex-agent before starting:
    provedex-agent --rate-limit-off &
"""

import asyncio
import os

from langchain_core.language_models.fake_chat_models import FakeListChatModel
from langchain_core.prompts import ChatPromptTemplate

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig


async def main() -> None:
    cfg = ProvedexConfig(
        agent_url=os.getenv("PROVEDEX_AGENT_URL", "http://127.0.0.1:8765"),
        agent_id="example-langchain-agent",
        model_id="fake-list",
        session_id="example-session-001",
    )
    handler = ProvedexCallbackHandler(config=cfg)

    # Replace FakeListChatModel with ChatOpenAI(model="gpt-4o") or any real LLM.
    llm = FakeListChatModel(responses=["Hello back."])
    prompt = ChatPromptTemplate.from_template("Say hi to {name}.")
    chain = prompt | llm

    async with handler.session("example-request"):
        await chain.ainvoke({"name": "world"}, config={"callbacks": [handler]})

    print(f"signed={handler.signed_total} dropped={handler.dropped_total}")


if __name__ == "__main__":
    asyncio.run(main())
```

- [ ] **Step 3: Write `examples/langgraph_basic.py`**

```python
"""Minimal LangGraph pipeline with Provedex signing.

Demonstrates that LangGraph users get audit coverage automatically because
LangGraph propagates LangChain callbacks for every LLM and tool call.

Run a local provedex-agent before starting:
    provedex-agent --rate-limit-off &
"""

import asyncio
import os
from typing import TypedDict

from langchain_core.language_models.fake_chat_models import FakeListChatModel
from langgraph.graph import END, START, StateGraph

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig


class State(TypedDict):
    answer: str


async def main() -> None:
    cfg = ProvedexConfig(
        agent_url=os.getenv("PROVEDEX_AGENT_URL", "http://127.0.0.1:8765"),
        agent_id="example-langgraph-agent",
        model_id="fake-list",
    )
    handler = ProvedexCallbackHandler(config=cfg)

    llm = FakeListChatModel(responses=["Hello from the graph."])

    async def respond(state: State, config) -> State:
        resp = await llm.ainvoke("greet user", config=config)
        return {"answer": resp.content}

    graph_builder = StateGraph(State)
    graph_builder.add_node("respond", respond)
    graph_builder.add_edge(START, "respond")
    graph_builder.add_edge("respond", END)
    graph = graph_builder.compile()

    async with handler.session("graph-example"):
        await graph.ainvoke({"answer": ""}, config={"callbacks": [handler]})

    print(f"signed={handler.signed_total} dropped={handler.dropped_total}")


if __name__ == "__main__":
    asyncio.run(main())
```

- [ ] **Step 4: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain/README.md \
        bindings/python/provedex-langchain/examples/
git commit -m "docs(langchain): README + LangChain + LangGraph examples"
```

---

## Task 11: Link from root README

**Files:**
- Modify: `README.md` (repo root)

- [ ] **Step 1: Add the new row to the Components table**

In the Components table (around line 20-25), add after the existing `provedex-pipecat` row:

```markdown
| `provedex-langchain` (Python) | LangChain `CallbackHandler` that signs every LLM and tool call via the sidecar. Covers LangGraph by inheritance. PyPI. See [`bindings/python/provedex-langchain/`](bindings/python/provedex-langchain/README.md). | shipped |
```

- [ ] **Step 2: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add README.md
git commit -m "docs: link provedex-langchain from root README components table"
```

---

## Task 12: CI - extend bindings-python job to cover both packages

**Files:**
- Modify: `.github/workflows/ci.yml`

The existing `bindings-python (pytest)` job runs the pipecat suite. Extend it to also install + lint + test the langchain package. Either:

(a) Add steps to the existing job (sequential pipecat -> langchain in one job).
(b) Add a sibling job `bindings-python-langchain` (parallel).

Recommended: option (a) for simpler matrix; cargo build is shared and amortizes across both packages.

- [ ] **Step 1: Read the existing ci.yml**

```bash
cat .github/workflows/ci.yml
```

Identify the `bindings-python (pytest)` job and the steps under it.

- [ ] **Step 2: Add langchain steps after the pipecat steps**

For each existing step under `bindings-python` that operates inside `bindings/python/provedex-pipecat`, add a sibling step that operates inside `bindings/python/provedex-langchain`. Specifically add (in order, after the pipecat integration test step):

```yaml
      - name: Install langchain binding (dev deps)
        working-directory: bindings/python/provedex-langchain
        run: |
          pip install -e ".[dev]"
      - name: Lint (ruff) langchain
        working-directory: bindings/python/provedex-langchain
        run: ruff check src tests
      - name: Typecheck (mypy) langchain
        working-directory: bindings/python/provedex-langchain
        run: mypy src
      - name: Unit tests (langchain)
        working-directory: bindings/python/provedex-langchain
        run: pytest -v -m "not integration"
      - name: Integration tests (langchain)
        working-directory: bindings/python/provedex-langchain
        run: pytest -v -m integration
```

The provedex-agent binary built earlier in the job is reused; no second cargo build needed.

- [ ] **Step 3: Verify yaml parses**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml').read()); print('ok')"
```

Expected: `ok`. If `pyyaml` is unavailable, install it locally first.

- [ ] **Step 4: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add .github/workflows/ci.yml
git commit -m "ci: extend bindings-python job to cover provedex-langchain"
```

---

## Task 13: RELEASING.md

**Files:**
- Create: `bindings/python/provedex-langchain/RELEASING.md`

- [ ] **Step 1: Write the file**

```markdown
# Release process for provedex-langchain

Pre-release checklist:

1. All tests pass locally and in CI.
2. `pyproject.toml` version bumped if shipping a new version.
3. Tag the binding release: `git tag langchain-vX.Y.Z` (binding-scoped prefix so it does not collide with the agent's `vX.Y.Z` tags).

Publish to PyPI:

\`\`\`
cd bindings/python/provedex-langchain
python -m pip install --upgrade build twine
python -m build
python -m twine check dist/*
python -m twine upload dist/*
\`\`\`

After publish:

1. Verify `pip install provedex-langchain` from a clean venv pulls the new version.
2. Confirm the README on PyPI renders correctly (long_description from `README.md`).
3. Update the `provedex-langchain` row in the root `README.md` Components table if anything material changed.

Yank policy: same as `provedex-pipecat`; see `bindings/python/provedex-pipecat/RELEASING.md` for the procedure.

Out of scope here: the Rust agent + CLI publish process lives in the root `RELEASING.md`.
```

- [ ] **Step 2: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-langchain/RELEASING.md
git commit -m "docs(langchain): RELEASING.md with PyPI publish recipe"
```

---

## Task 14: Self-review with code-review-provedex skill

- [ ] **Step 1: Run the local 5-gate Rust + 4-gate Python**

```bash
cd /Users/adi/Desktop/provedex
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
cargo deny check

cd bindings/python/provedex-langchain
.venv/bin/ruff check src tests
.venv/bin/mypy src
.venv/bin/pytest -v
.venv/bin/pytest -v -m integration
```

All nine must pass.

- [ ] **Step 2: ASCII + AI-slop scan on all changed files**

```bash
cd /Users/adi/Desktop/provedex
files=$(git diff --name-only main..HEAD | grep -v -E '^(Cargo\.lock)$')
for f in $files; do
  [ -f "$f" ] && LC_ALL=C grep -nP '[^\x00-\x7F]' "$f"
done
grep -niE '\b(robust|comprehensive|powerful|elegant|leveraging|cutting-edge|next-gen|seamless)\b' $files
```

Both should print nothing.

- [ ] **Step 3: Commit-trailer check**

```bash
git log main..HEAD --format=%B | grep -i co-authored-by
```

Should print nothing.

- [ ] **Step 4: Apply code-review-provedex skill against the diff**

Open `.claude/skills/code-review-provedex/SKILL.md`. Walk the diff against the auto-block invariants:

- No change to canonical-JSON encoding, hashed-field set, or GENESIS_PARENT_HASH in provedex-core (this branch should not touch provedex-core).
- All commit subjects conformant.
- All new pub items in provedex-langchain (`ProvedexCallbackHandler`, `ProvedexConfig`) have docstrings.

Fix anything that surfaces inline. Recommit if needed.

---

## Task 15: PR + merge + close issue

- [ ] **Step 1: Push the branch**

```bash
cd /Users/adi/Desktop/provedex
git push -u origin feat/langchain-binding
```

- [ ] **Step 2: Open PR using voice-aditya semi-formal register**

```bash
gh pr create --title "feat(bindings/python): provedex-langchain binding (LangChain + LangGraph)" \
  --body "<see below>"
```

PR body sections: Summary (1 paragraph + why), What changed (file-level bullets), Test plan (checklist of every gate), Closes #N (the issue from Task 1).

- [ ] **Step 3: Wait for CI green**

```bash
gh pr checks <PR_NUMBER> --watch
```

All 3 jobs: fmt+clippy+test, cargo audit+deny, bindings-python (pytest).

- [ ] **Step 4: Merge + close**

```bash
gh pr merge <PR_NUMBER> --squash --delete-branch
gh issue close <ISSUE_NUMBER>
```

- [ ] **Step 5: Pull main, verify clean**

```bash
git checkout main
git pull --ff-only
git log --oneline -3
```

---

## Self-review (writer's pass)

**Spec coverage:**

- One-package decision: Task 1 scaffolds a single `provedex-langchain` package.
- Dual sync + async inheritance: Task 6 implements both interfaces via `AsyncCallbackHandler, BaseCallbackHandler` MRO with sync and async methods that delegate to the same enqueue path.
- Explicit operator-driven session lifecycle: Task 6 (`start_session` / `end_session` / `session` context manager) + Task 7 (lifecycle tests, sync + async, normal + exception).
- Inlined HTTP client: Task 4 (clone from pipecat, no behavior change).
- CorrelationState run-id keyed buffer: Task 3 (`buffer_llm_start` / `take_llm` and tool equivalents) + Task 5 mapping uses them.
- Callback to AgentEvent mapping per the spec table: Task 5 (mapping pure functions) + Task 6 (handler invokes them).
- Tests: unit (Tasks 2, 3, 4, 5), sync handler (Task 6), async handler (Task 6), session lifecycle (Task 7), async smoke (Task 8), integration with both LangChain + LangGraph (Task 9).
- LangGraph covered by inheritance: Task 9 integration test runs both a LangChain LCEL chain and a LangGraph graph against the same handler.
- CI lane: Task 12 extends the existing bindings-python job.
- Documentation: Task 10 (README + examples), Task 11 (root README link), Task 13 (RELEASING.md).
- Self-review: Task 14 runs five Rust gates + four Python gates + ASCII + AI-slop + co-author trailer scans.

**Placeholder scan:** none. All steps have actual content.

**Type consistency:**

- `CorrelationState.buffer_llm_start(run_id, *, model_id, prompt_payload)` and `.take_llm(run_id)` used identically in `_state.py`, `test_state.py`, and the handler.
- `tool_called_event(tool_name=, args=)` and `tool_returned_event(tool_name=, result=, latency_ms=, success=)` consistent across mapping tests and handler.
- `ProvedexConfig` field names match across config.py, all tests, and the handler.
- `AgentClient.sign(event)` raises `SignError` consistently.
- `ProvedexCallbackHandler` is the public class name in `__init__.py`, every test import, the README, the examples.

No gaps. Plan ready.

---

## Risks during execution

| Risk | Mitigation |
|------|------------|
| LangChain 0.3 callback signatures differ from assumption | Task 6 step 4 uses defensive kwargs (`**kwargs`) and `serialized.get("name", "unknown")`. If a constructor rejects a kwarg, report BLOCKED with the exact error. |
| `FakeListChatModel` may emit different generation shape | Defensive `try/except (IndexError, AttributeError)` in `_emit_model_invoked` falls back to empty response text. Integration test verifies signed_total >= 3 not == 3 to absorb shape variance. |
| LangGraph 0.2 API renames | Pin in dev deps. CI catches breakage early. |
| `BaseCallbackHandler` and `AsyncCallbackHandler` MRO conflict on shared method name | Both are abstract base classes from `langchain_core.callbacks`. Dual inheritance is documented as supported. If conflict surfaces, separate the handler into two classes sharing a private `_Enqueuer` mixin. |
| Worker leaks across tests | Each test that calls `start()` calls `stop()` in the same scope. `pytest-asyncio` event loop is per-test, so any leaked task dies with the loop. |
| `_canonical_json_bytes` uses `default=str` for non-JSON-serializable types (e.g., BaseMessage); message content is serialized to a dict already in `on_chat_model_start`. Verify no LLM result smuggles a non-serializable object into prompt_payload. | Integration test runs a real chain through; if serialization fails, the unit tests catch it first. |
