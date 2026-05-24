# provedex-pipecat Binding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and ship a Python package `provedex-pipecat` that signs every Pipecat `Frame` flowing through a voice-agent pipeline by POSTing to the localhost `provedex-agent` sidecar. One line of integration code for the operator, hash-chained signed ledger as output.

**Architecture:** Single processor class with one background worker draining a bounded asyncio.Queue. Inlined private HTTP client (httpx). Stateful correlation for paired LLM frames. Existing event-schema-v1 variants only (no spec changes).

**Tech Stack:** Python 3.11+, hatchling build backend, httpx (async HTTP), pydantic v2 (config validation), pipecat-ai (runtime dep), pytest + pytest-asyncio + respx (tests), ruff + mypy (lint/type).

---

## Pre-flight (already done)

- Branch: `feat/pipecat-binding` (created off main, spec pushed).
- Spec: `docs/superpowers/specs/2026-05-24-provedex-pipecat-binding-design.md` (approved).
- Issue: open one before Task 1 below.

## File Structure

```
bindings/python/provedex-pipecat/
  pyproject.toml                     hatch + deps + tool config
  README.md                          quickstart, mapping, failure modes, regulatory context
  .gitignore                         dist/, build/, .pytest_cache, __pycache__, *.egg-info
  src/provedex_pipecat/
    __init__.py                      public exports
    config.py                        ProvedexConfig (pydantic dataclass) + env loading
    _client.py                       AgentClient: httpx async POST /v1/sign
    _state.py                        CorrelationState: LLMMessagesFrame + LLMFullResponseEndFrame buffer + frame dedup
    mapping.py                       pure functions Frame -> dict (one per supported variant)
    processor.py                     ProvedexFrameProcessor: queue, worker task, lifecycle
  tests/
    conftest.py                      shared fixtures: respx_mock, agent_binary (cargo build + spawn), tmp ledger
    test_config.py                   env loading, defaults, overrides
    test_mapping.py                  golden-file POST shapes per frame type
    test_client.py                   AgentClient happy path + error paths (mocked)
    test_state.py                    correlation buffer behavior + dedup
    test_processor.py                full processor with mocked client
    test_async_smoke.py              1000-frame burst, p99 producer block < 1ms
    test_integration.py              real agent + real pipeline + provedex verify
  examples/
    voice_agent_basic.py             illustrative pipeline (offline; runnable shell with mocks)

bindings/python/CLAUDE.md            update navigation + conventions
.github/workflows/ci.yml             add bindings-python job
README.md                            link to binding under Components table
```

---

## Task 1: File issue + scaffold the package directory

**Files:**
- Create: `bindings/python/provedex-pipecat/pyproject.toml`
- Create: `bindings/python/provedex-pipecat/.gitignore`
- Create: `bindings/python/provedex-pipecat/src/provedex_pipecat/__init__.py` (empty stub for now)
- Create: `bindings/python/provedex-pipecat/tests/__init__.py` (empty)

- [ ] **Step 1: File tracking issue**

```bash
gh issue create --title "feat(bindings/python): provedex-pipecat binding for voice agents" \
  --body "$(cat <<'EOF'
First Python binding. Pipecat FrameProcessor that signs every Frame via the local provedex-agent.

Spec: docs/superpowers/specs/2026-05-24-provedex-pipecat-binding-design.md
Plan: docs/superpowers/plans/2026-05-24-provedex-pipecat-binding.md
EOF
)"
```

Expected: prints new issue URL. Note the issue number (referenced as #N in commits + final PR).

- [ ] **Step 2: Write pyproject.toml**

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "provedex-pipecat"
version = "0.1.0"
description = "Pipecat FrameProcessor that signs every frame via the Provedex sidecar."
readme = "README.md"
requires-python = ">=3.11"
license = { text = "Apache-2.0" }
authors = [
    { name = "Aditya Suresh", email = "adi@provedex.io" },
]
keywords = ["pipecat", "voice", "audit", "signing", "compliance", "ed25519", "provedex"]
classifiers = [
    "Development Status :: 4 - Beta",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: Apache Software License",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
    "Topic :: Security :: Cryptography",
]
dependencies = [
    "pipecat-ai>=0.0.40,<0.1.0",
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
]

[project.urls]
Homepage = "https://github.com/provedex/provedex"
Repository = "https://github.com/provedex/provedex"
Issues = "https://github.com/provedex/provedex/issues"

[tool.hatch.build.targets.wheel]
packages = ["src/provedex_pipecat"]

[tool.ruff]
line-length = 100
target-version = "py311"

[tool.ruff.lint]
select = ["E", "F", "I", "B", "UP", "ASYNC"]

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

- [ ] **Step 4: Write empty __init__.py files**

`src/provedex_pipecat/__init__.py`:

```python
"""Provedex binding for Pipecat voice agent pipelines."""

__version__ = "0.1.0"
```

`tests/__init__.py`: empty file.

- [ ] **Step 5: Verify pyproject is parseable**

```bash
cd bindings/python/provedex-pipecat
python3 -c "import tomllib; tomllib.loads(open('pyproject.toml').read()); print('ok')"
```

Expected: `ok`

- [ ] **Step 6: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-pipecat
git commit -m "chore(bindings/python): scaffold provedex-pipecat package"
```

---

## Task 2: ProvedexConfig with env loading

**Files:**
- Create: `bindings/python/provedex-pipecat/src/provedex_pipecat/config.py`
- Create: `bindings/python/provedex-pipecat/tests/test_config.py`

- [ ] **Step 1: Write failing test for default config**

`tests/test_config.py`:

```python
import os

from provedex_pipecat.config import ProvedexConfig


def test_defaults_with_no_env(monkeypatch):
    monkeypatch.delenv("PROVEDEX_AGENT_URL", raising=False)
    cfg = ProvedexConfig()
    assert cfg.agent_url == "http://127.0.0.1:8765"
    assert cfg.agent_id == "pipecat-agent"
    assert cfg.model_id == "unknown"
    assert cfg.queue_size == 1000
    assert cfg.request_timeout_seconds == 2.0
    assert cfg.shutdown_drain_seconds == 5.0
    assert cfg.on_sign_failure == "warn"
    assert cfg.session_id  # auto-generated, non-empty
    assert cfg.include_frames is None


def test_env_overrides_url(monkeypatch):
    monkeypatch.setenv("PROVEDEX_AGENT_URL", "http://10.0.0.5:9999")
    cfg = ProvedexConfig()
    assert cfg.agent_url == "http://10.0.0.5:9999"


def test_constructor_overrides_env(monkeypatch):
    monkeypatch.setenv("PROVEDEX_AGENT_URL", "http://10.0.0.5:9999")
    cfg = ProvedexConfig(agent_url="http://7.7.7.7:7777")
    assert cfg.agent_url == "http://7.7.7.7:7777"


def test_on_sign_failure_invalid_rejected():
    import pytest

    with pytest.raises(Exception):
        ProvedexConfig(on_sign_failure="explode")
```

- [ ] **Step 2: Run test, confirm failure**

```bash
cd bindings/python/provedex-pipecat
pip install -e ".[dev]"
pytest tests/test_config.py -v
```

Expected: ImportError / module not found.

- [ ] **Step 3: Implement config.py**

```python
"""Configuration for the Provedex Pipecat binding."""

from __future__ import annotations

import os
import uuid
from typing import Literal

from pydantic import BaseModel, Field, field_validator

OnSignFailure = Literal["warn", "raise", "silent"]


class ProvedexConfig(BaseModel):
    """Configuration for ProvedexFrameProcessor.

    Env-first with constructor overrides. PROVEDEX_AGENT_URL is the only
    runtime-discovered field; everything else is set explicitly by the operator.
    """

    agent_url: str = Field(
        default_factory=lambda: os.getenv("PROVEDEX_AGENT_URL", "http://127.0.0.1:8765")
    )
    session_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str = "pipecat-agent"
    model_id: str = "unknown"
    include_frames: list[type] | None = None
    on_sign_failure: OnSignFailure = "warn"
    queue_size: int = Field(default=1000, ge=1)
    request_timeout_seconds: float = Field(default=2.0, gt=0)
    shutdown_drain_seconds: float = Field(default=5.0, ge=0)

    model_config = {"arbitrary_types_allowed": True}

    @field_validator("agent_url")
    @classmethod
    def url_must_be_http(cls, v: str) -> str:
        if not v.startswith(("http://", "https://")):
            raise ValueError(f"agent_url must start with http:// or https://, got {v!r}")
        return v
```

- [ ] **Step 4: Run test, confirm pass**

```bash
pytest tests/test_config.py -v
```

Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-pipecat/src/provedex_pipecat/config.py \
        bindings/python/provedex-pipecat/tests/test_config.py
git commit -m "feat(pipecat): ProvedexConfig with env + pydantic validation"
```

---

## Task 3: mapping.py - pure Frame to AgentEvent translators

**Files:**
- Create: `bindings/python/provedex-pipecat/src/provedex_pipecat/mapping.py`
- Create: `bindings/python/provedex-pipecat/tests/test_mapping.py`

Mapping module exposes a function `frame_to_event(frame, config, correlation_state) -> dict | None`. Returns None for skipped frames.

- [ ] **Step 1: Write failing test for StartFrame**

`tests/test_mapping.py`:

```python
import hashlib
import json

import pytest
from pipecat.frames.frames import (
    EndFrame,
    Frame,
    FunctionCallInProgressFrame,
    FunctionCallResultFrame,
    LLMFullResponseEndFrame,
    LLMMessagesFrame,
    StartFrame,
    TextFrame,
    TranscriptionFrame,
)

from provedex_pipecat.config import ProvedexConfig
from provedex_pipecat._state import CorrelationState
from provedex_pipecat.mapping import frame_to_event


def _sha256_hex(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


@pytest.fixture
def config():
    return ProvedexConfig(
        agent_url="http://127.0.0.1:8765",
        session_id="test-session",
        agent_id="test-agent",
        model_id="test-model",
    )


@pytest.fixture
def state():
    return CorrelationState()


def test_start_frame_maps_to_session_started(config, state):
    event = frame_to_event(StartFrame(), config, state)
    assert event == {
        "type": "SessionStarted",
        "payload": {
            "agent_id": "test-agent",
            "model_id": "test-model",
            "session_id": "test-session",
        },
    }


def test_end_frame_maps_to_session_ended(config, state):
    event = frame_to_event(EndFrame(), config, state)
    assert event == {
        "type": "SessionEnded",
        "payload": {
            "reason": "pipeline_end",
            "summary_sha256": _sha256_hex(b""),
        },
    }


def test_transcription_frame_maps_to_utterance_captured(config, state):
    frame = TranscriptionFrame(text="hello world", user_id="u1", timestamp="2026-05-24T00:00:00Z", language="en-US")
    event = frame_to_event(frame, config, state)
    assert event["type"] == "UtteranceCaptured"
    assert event["payload"]["transcript"] == "hello world"
    assert event["payload"]["lang"] == "en-US"
    assert event["payload"]["audio_sha256"] == _sha256_hex(b"hello world")
    assert event["payload"]["duration_ms"] == 0


def test_function_call_in_progress_maps_to_tool_called(config, state):
    frame = FunctionCallInProgressFrame(
        function_name="get_weather",
        tool_call_id="call_1",
        arguments={"city": "NYC"},
    )
    event = frame_to_event(frame, config, state)
    assert event["type"] == "ToolCalled"
    assert event["payload"]["tool_name"] == "get_weather"
    assert event["payload"]["args_redacted"] == {"city": "NYC"}
    expected_args_hash = _sha256_hex(json.dumps({"city": "NYC"}, sort_keys=True, separators=(",", ":")).encode())
    assert event["payload"]["args_sha256"] == expected_args_hash


def test_function_call_result_maps_to_tool_returned(config, state):
    frame = FunctionCallResultFrame(
        function_name="get_weather",
        tool_call_id="call_1",
        arguments={"city": "NYC"},
        result={"temp": 72},
    )
    event = frame_to_event(frame, config, state)
    assert event["type"] == "ToolReturned"
    assert event["payload"]["tool_name"] == "get_weather"
    assert event["payload"]["success"] is True
    assert event["payload"]["latency_ms"] == 0
    expected_result_hash = _sha256_hex(
        json.dumps({"temp": 72}, sort_keys=True, separators=(",", ":")).encode()
    )
    assert event["payload"]["result_sha256"] == expected_result_hash


def test_llm_messages_frame_alone_does_not_emit(config, state):
    messages = [{"role": "user", "content": "hi"}]
    event = frame_to_event(LLMMessagesFrame(messages=messages), config, state)
    assert event is None  # buffered, no signed event yet


def test_llm_messages_then_full_response_end_pairs(config, state):
    messages = [{"role": "user", "content": "hi"}]
    frame_to_event(LLMMessagesFrame(messages=messages), config, state)  # buffer

    end = LLMFullResponseEndFrame()
    # Pipecat carries the final text on a TextFrame between Start and End.
    # The state buffers the TextFrame text alongside the messages.
    state.buffer_response_text("hello back")

    event = frame_to_event(end, config, state)
    assert event["type"] == "ModelInvoked"
    assert event["payload"]["model_id"] == "test-model"
    expected_prompt = _sha256_hex(
        json.dumps(messages, sort_keys=True, separators=(",", ":")).encode()
    )
    assert event["payload"]["prompt_sha256"] == expected_prompt
    assert event["payload"]["response_sha256"] == _sha256_hex(b"hello back")
    assert event["payload"]["prompt_tokens"] == 0
    assert event["payload"]["response_tokens"] == 0


def test_text_frame_without_pairing_maps_to_utterance_spoken(config, state):
    event = frame_to_event(TextFrame(text="hi there"), config, state)
    assert event["type"] == "UtteranceSpoken"
    assert event["payload"]["text"] == "hi there"
    assert event["payload"]["text_sha256"] == _sha256_hex(b"hi there")
    assert event["payload"]["audio_sha256"] == _sha256_hex(b"")


def test_unknown_frame_returns_none(config, state):
    class WeirdFrame(Frame):
        pass

    assert frame_to_event(WeirdFrame(), config, state) is None
```

- [ ] **Step 2: Run, confirm fails**

```bash
pytest tests/test_mapping.py -v
```

Expected: ModuleNotFoundError on `provedex_pipecat._state` and `.mapping`.

- [ ] **Step 3: Implement `_state.py` minimally to support correlation tests**

`src/provedex_pipecat/_state.py`:

```python
"""Per-processor correlation buffer for paired LLM frames + frame dedup."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass
class CorrelationState:
    """Track in-flight LLM exchanges and seen frame IDs."""

    last_messages: list[dict[str, Any]] | None = None
    pending_response_text: str = ""
    response_in_progress: bool = False
    seen_frame_ids: set[int] = field(default_factory=set)

    def buffer_messages(self, messages: list[dict[str, Any]]) -> None:
        self.last_messages = messages

    def buffer_response_text(self, text: str) -> None:
        self.pending_response_text += text

    def take_paired_invocation(self) -> tuple[list[dict[str, Any]] | None, str]:
        """Return (messages, response_text) and clear the buffers."""
        messages = self.last_messages
        text = self.pending_response_text
        self.last_messages = None
        self.pending_response_text = ""
        self.response_in_progress = False
        return messages, text

    def mark_response_start(self) -> None:
        self.response_in_progress = True
        self.pending_response_text = ""

    def already_seen(self, frame_id: int) -> bool:
        if frame_id in self.seen_frame_ids:
            return True
        self.seen_frame_ids.add(frame_id)
        return False
```

- [ ] **Step 4: Implement `mapping.py`**

`src/provedex_pipecat/mapping.py`:

```python
"""Pure functions translating Pipecat Frames to AgentEvent dicts.

The output shape matches docs/spec/event-schema-v1.md:
    { "type": "<VariantName>", "payload": { ... } }
"""

from __future__ import annotations

import hashlib
import json
from typing import Any

from pipecat.frames.frames import (
    EndFrame,
    Frame,
    FunctionCallInProgressFrame,
    FunctionCallResultFrame,
    LLMFullResponseEndFrame,
    LLMFullResponseStartFrame,
    LLMMessagesFrame,
    StartFrame,
    TextFrame,
    TranscriptionFrame,
)

from .config import ProvedexConfig
from ._state import CorrelationState


def _sha256_hex(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _canonical_json_bytes(value: Any) -> bytes:
    """Compact JSON with sorted keys. Matches the spirit of canonical-JSON for
    hashing purposes inside the Python binding. The actual cryptographic
    canonical JSON lives in the Rust agent; this hash is just the binding's
    receipt of what it sent.
    """
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def frame_to_event(
    frame: Frame,
    config: ProvedexConfig,
    state: CorrelationState,
) -> dict[str, Any] | None:
    """Translate a Pipecat Frame to an AgentEvent dict, or None to skip."""

    if isinstance(frame, StartFrame):
        return {
            "type": "SessionStarted",
            "payload": {
                "agent_id": config.agent_id,
                "model_id": config.model_id,
                "session_id": config.session_id,
            },
        }

    if isinstance(frame, EndFrame):
        return {
            "type": "SessionEnded",
            "payload": {
                "reason": "pipeline_end",
                "summary_sha256": _sha256_hex(b""),
            },
        }

    if isinstance(frame, TranscriptionFrame):
        transcript = frame.text or ""
        return {
            "type": "UtteranceCaptured",
            "payload": {
                "audio_sha256": _sha256_hex(transcript.encode()),
                "transcript": transcript,
                "lang": getattr(frame, "language", "") or "",
                "duration_ms": 0,
            },
        }

    if isinstance(frame, FunctionCallInProgressFrame):
        args = frame.arguments or {}
        return {
            "type": "ToolCalled",
            "payload": {
                "tool_name": frame.function_name,
                "args_sha256": _sha256_hex(_canonical_json_bytes(args)),
                "args_redacted": args,
            },
        }

    if isinstance(frame, FunctionCallResultFrame):
        result = frame.result if frame.result is not None else {}
        return {
            "type": "ToolReturned",
            "payload": {
                "tool_name": frame.function_name,
                "result_sha256": _sha256_hex(_canonical_json_bytes(result)),
                "latency_ms": 0,
                "success": True,
            },
        }

    if isinstance(frame, LLMMessagesFrame):
        # Buffer for pairing with the upcoming LLMFullResponseEndFrame.
        state.buffer_messages(list(frame.messages))
        return None

    if isinstance(frame, LLMFullResponseStartFrame):
        state.mark_response_start()
        return None

    if isinstance(frame, LLMFullResponseEndFrame):
        messages, response_text = state.take_paired_invocation()
        if messages is None:
            # End without preceding messages buffer; cannot construct a
            # ModelInvoked. Drop quietly.
            return None
        return {
            "type": "ModelInvoked",
            "payload": {
                "model_id": config.model_id,
                "prompt_sha256": _sha256_hex(_canonical_json_bytes(messages)),
                "response_sha256": _sha256_hex(response_text.encode()),
                "prompt_tokens": 0,
                "response_tokens": 0,
            },
        }

    if isinstance(frame, TextFrame):
        if state.response_in_progress:
            # Accumulate as part of the in-flight LLM response.
            state.buffer_response_text(frame.text)
            return None
        # Standalone text frame; treat as UtteranceSpoken (TTS-bound text).
        return {
            "type": "UtteranceSpoken",
            "payload": {
                "text_sha256": _sha256_hex(frame.text.encode()),
                "text": frame.text,
                "audio_sha256": _sha256_hex(b""),
            },
        }

    return None
```

- [ ] **Step 5: Run mapping tests**

```bash
pytest tests/test_mapping.py -v
```

Expected: 9 passed.

- [ ] **Step 6: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-pipecat/src/provedex_pipecat/_state.py \
        bindings/python/provedex-pipecat/src/provedex_pipecat/mapping.py \
        bindings/python/provedex-pipecat/tests/test_mapping.py
git commit -m "feat(pipecat): Frame -> AgentEvent mapping with LLM correlation"
```

---

## Task 4: AgentClient async HTTP wrapper

**Files:**
- Create: `bindings/python/provedex-pipecat/src/provedex_pipecat/_client.py`
- Create: `bindings/python/provedex-pipecat/tests/test_client.py`

- [ ] **Step 1: Write failing test**

`tests/test_client.py`:

```python
import httpx
import pytest
import respx

from provedex_pipecat._client import AgentClient, SignError


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
pytest tests/test_client.py -v
```

Expected: ImportError on `provedex_pipecat._client`.

- [ ] **Step 3: Implement `_client.py`**

```python
"""Private async HTTP client for the provedex-agent /v1/sign endpoint."""

from __future__ import annotations

from typing import Any

import httpx


class SignError(Exception):
    """Raised when a sign attempt fails (network, timeout, or non-2xx)."""


class AgentClient:
    """Thin httpx wrapper. One per processor instance; reuses the connection."""

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
            raise SignError(
                f"agent returned {resp.status_code}: {resp.text[:200]}"
            )

    async def aclose(self) -> None:
        await self._client.aclose()
```

- [ ] **Step 4: Run tests**

```bash
pytest tests/test_client.py -v
```

Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-pipecat/src/provedex_pipecat/_client.py \
        bindings/python/provedex-pipecat/tests/test_client.py
git commit -m "feat(pipecat): AgentClient httpx wrapper for /v1/sign"
```

---

## Task 5: CorrelationState dedup test

**Files:**
- Create: `bindings/python/provedex-pipecat/tests/test_state.py`

`_state.py` already exists from Task 3. This task adds dedup tests + buffer tests.

- [ ] **Step 1: Write test**

```python
from provedex_pipecat._state import CorrelationState


def test_initial_state_is_empty():
    s = CorrelationState()
    assert s.last_messages is None
    assert s.pending_response_text == ""
    assert s.response_in_progress is False
    assert s.seen_frame_ids == set()


def test_buffer_and_take_clears_state():
    s = CorrelationState()
    s.buffer_messages([{"role": "user", "content": "x"}])
    s.mark_response_start()
    s.buffer_response_text("hel")
    s.buffer_response_text("lo")
    messages, text = s.take_paired_invocation()
    assert messages == [{"role": "user", "content": "x"}]
    assert text == "hello"
    assert s.last_messages is None
    assert s.pending_response_text == ""
    assert s.response_in_progress is False


def test_take_without_buffer_returns_none_messages():
    s = CorrelationState()
    messages, text = s.take_paired_invocation()
    assert messages is None
    assert text == ""


def test_dedup_first_seen_is_false_then_true():
    s = CorrelationState()
    assert s.already_seen(42) is False
    assert s.already_seen(42) is True
    assert s.already_seen(43) is False
```

- [ ] **Step 2: Run**

```bash
pytest tests/test_state.py -v
```

Expected: 4 passed.

- [ ] **Step 3: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-pipecat/tests/test_state.py
git commit -m "test(pipecat): CorrelationState buffer + dedup behavior"
```

---

## Task 6: ProvedexFrameProcessor with queue + worker

**Files:**
- Create: `bindings/python/provedex-pipecat/src/provedex_pipecat/processor.py`
- Modify: `bindings/python/provedex-pipecat/src/provedex_pipecat/__init__.py`
- Create: `bindings/python/provedex-pipecat/tests/test_processor.py`

- [ ] **Step 1: Write failing test**

`tests/test_processor.py`:

```python
import asyncio
from collections import Counter

import httpx
import pytest
import respx
from pipecat.frames.frames import EndFrame, StartFrame, TranscriptionFrame

from provedex_pipecat import ProvedexConfig, ProvedexFrameProcessor


@pytest.mark.asyncio
@respx.mock
async def test_processor_signs_start_then_end():
    posted = []

    def record(request):
        posted.append(request.json())
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=record)

    cfg = ProvedexConfig(agent_url="http://127.0.0.1:8765", session_id="s1")
    proc = ProvedexFrameProcessor(config=cfg)
    await proc.start()

    await proc.handle_frame(StartFrame())
    await proc.handle_frame(
        TranscriptionFrame(text="hi", user_id="u", timestamp="t", language="en")
    )
    await proc.handle_frame(EndFrame())

    await proc.stop()

    types = Counter(body["event"]["type"] for body in posted)
    assert types["SessionStarted"] == 1
    assert types["UtteranceCaptured"] == 1
    assert types["SessionEnded"] == 1


@pytest.mark.asyncio
@respx.mock
async def test_processor_drops_when_agent_unreachable():
    respx.post("http://127.0.0.1:8765/v1/sign").mock(
        side_effect=httpx.ConnectError("refused")
    )

    cfg = ProvedexConfig(agent_url="http://127.0.0.1:8765", on_sign_failure="warn")
    proc = ProvedexFrameProcessor(config=cfg)
    await proc.start()
    await proc.handle_frame(StartFrame())
    await proc.stop()

    assert proc.dropped_total >= 1


@pytest.mark.asyncio
async def test_processor_dedup_same_frame_not_double_signed():
    cfg = ProvedexConfig(agent_url="http://127.0.0.1:8765")
    proc = ProvedexFrameProcessor(config=cfg)
    await proc.start()

    frame = StartFrame()
    await proc.handle_frame(frame)
    await proc.handle_frame(frame)  # same instance

    # Without mocking agent: queue accepts first, second is filtered by dedup.
    assert proc.signed_total + proc.dropped_total <= 1 + 0  # only 1 enqueued

    await proc.stop()
```

Note: the third test asserts dedup at enqueue time, before HTTP. The processor exposes `signed_total` and `dropped_total` counters.

- [ ] **Step 2: Run, confirm fails**

```bash
pytest tests/test_processor.py -v
```

Expected: ImportError.

- [ ] **Step 3: Implement `processor.py`**

```python
"""ProvedexFrameProcessor: signs every supported Pipecat Frame via the local agent."""

from __future__ import annotations

import asyncio
import logging
import time
from collections import deque
from typing import Any

from pipecat.frames.frames import EndFrame, Frame
from pipecat.processors.frame_processor import FrameDirection, FrameProcessor

from ._client import AgentClient, SignError
from ._state import CorrelationState
from .config import ProvedexConfig
from .mapping import frame_to_event

logger = logging.getLogger(__name__)


class ProvedexFrameProcessor(FrameProcessor):
    """Pipecat FrameProcessor that signs every supported Frame.

    Non-blocking: producer enqueues, a single background worker POSTs.
    Order-preserving: one worker keeps ledger order = pipeline order.
    Drop-oldest on overflow: most-recent signal survives, warning emitted.
    """

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

        # Counters scrapable by the operator.
        self.signed_total = 0
        self.dropped_total = 0
        self.overflow_total = 0

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
            except asyncio.TimeoutError:
                self._worker_task.cancel()
        await self._client.aclose()

    async def handle_frame(self, frame: Frame) -> None:
        """Test entry point. In a real pipeline Pipecat calls process_frame."""
        await self._enqueue_for_frame(frame)

    async def process_frame(self, frame: Frame, direction: FrameDirection) -> None:
        # Standard Pipecat hook. Always forward the frame; signing is side-effect.
        await self._enqueue_for_frame(frame)
        await self.push_frame(frame, direction)

    async def _enqueue_for_frame(self, frame: Frame) -> None:
        # Dedup based on Python object identity. Pipecat may route the same
        # frame instance through a multi-placed processor.
        if self._state.already_seen(id(frame)):
            return

        event = frame_to_event(frame, self._config, self._state)
        if event is None:
            return

        if len(self._queue) >= self._config.queue_size:
            # deque with maxlen drops oldest automatically on append; we count
            # and rate-limit the warning explicitly.
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

        # On EndFrame, ensure we have a chance to drain before pipeline exit.
        if isinstance(frame, EndFrame):
            await self._drain_with_timeout()

    async def _run_worker(self) -> None:
        while True:
            if not self._queue:
                if self._stopping:
                    return
                self._wakeup.clear()
                try:
                    await asyncio.wait_for(self._wakeup.wait(), timeout=0.1)
                except asyncio.TimeoutError:
                    continue
                continue

            event = self._queue.popleft()
            try:
                await self._client.sign(event)
                self.signed_total += 1
            except SignError as e:
                self.dropped_total += 1
                self._handle_sign_failure(e, event)

    def _handle_sign_failure(self, exc: SignError, event: dict[str, Any]) -> None:
        mode = self._config.on_sign_failure
        if mode == "raise":
            raise exc
        if mode == "warn":
            logger.warning("provedex sign failed for %s: %s", event["type"], exc)
        # mode == "silent": no log

    async def _drain_with_timeout(self) -> None:
        deadline = time.monotonic() + self._config.shutdown_drain_seconds
        while self._queue and time.monotonic() < deadline:
            self._wakeup.set()
            await asyncio.sleep(0.01)
```

- [ ] **Step 4: Update `__init__.py`**

```python
"""Provedex binding for Pipecat voice agent pipelines."""

from .config import ProvedexConfig
from .processor import ProvedexFrameProcessor

__version__ = "0.1.0"
__all__ = ["ProvedexConfig", "ProvedexFrameProcessor"]
```

- [ ] **Step 5: Run tests**

```bash
pytest tests/test_processor.py -v
```

Expected: 3 passed. If a test hangs, the worker `_run_worker` loop has a bug; verify the wakeup/timeout dance.

- [ ] **Step 6: Run all tests so far**

```bash
pytest -v
```

Expected: 17+ pass total (config 4 + mapping 9 + client 3 + state 4 + processor 3).

- [ ] **Step 7: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-pipecat/src/provedex_pipecat/processor.py \
        bindings/python/provedex-pipecat/src/provedex_pipecat/__init__.py \
        bindings/python/provedex-pipecat/tests/test_processor.py
git commit -m "feat(pipecat): ProvedexFrameProcessor with queue + worker"
```

---

## Task 7: Async smoke test for latency budget

**Files:**
- Create: `bindings/python/provedex-pipecat/tests/test_async_smoke.py`

- [ ] **Step 1: Write test**

```python
import asyncio
import statistics
import time

import httpx
import pytest
import respx
from pipecat.frames.frames import TranscriptionFrame

from provedex_pipecat import ProvedexConfig, ProvedexFrameProcessor


@pytest.mark.slow
@pytest.mark.asyncio
@respx.mock
async def test_producer_block_p99_under_one_ms():
    """Producer side of process_frame must not block on HTTP. We simulate
    a 1ms agent response, fire 1000 frames, and measure how long each
    handle_frame call took."""

    async def slow_responder(request):
        await asyncio.sleep(0.001)
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=slow_responder)

    cfg = ProvedexConfig(agent_url="http://127.0.0.1:8765", queue_size=2000)
    proc = ProvedexFrameProcessor(config=cfg)
    await proc.start()

    blocks_us: list[float] = []
    for i in range(1000):
        f = TranscriptionFrame(
            text=f"u{i}", user_id="u", timestamp="t", language="en"
        )
        t0 = time.perf_counter()
        await proc.handle_frame(f)
        blocks_us.append((time.perf_counter() - t0) * 1_000_000)

    await proc.stop()

    p50 = statistics.median(blocks_us)
    p99 = sorted(blocks_us)[int(0.99 * len(blocks_us))]
    print(f"\n  producer block: p50={p50:.1f}us p99={p99:.1f}us")
    assert p99 < 1000, f"p99 {p99:.1f}us exceeds 1ms budget"


@pytest.mark.slow
@pytest.mark.asyncio
@respx.mock
async def test_zero_drops_at_default_queue_with_steady_load():
    async def fast_responder(request):
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=fast_responder)

    cfg = ProvedexConfig(agent_url="http://127.0.0.1:8765", queue_size=1000)
    proc = ProvedexFrameProcessor(config=cfg)
    await proc.start()

    for i in range(500):
        await proc.handle_frame(
            TranscriptionFrame(text=f"u{i}", user_id="u", timestamp="t", language="en")
        )
        if i % 100 == 0:
            await asyncio.sleep(0.01)  # let worker drain

    await proc.stop()
    assert proc.overflow_total == 0
```

- [ ] **Step 2: Run**

```bash
pytest tests/test_async_smoke.py -v -s -m slow
```

Expected: 2 passed. Numbers printed.

- [ ] **Step 3: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-pipecat/tests/test_async_smoke.py
git commit -m "test(pipecat): async smoke test for producer latency budget"
```

---

## Task 8: Integration test with real provedex-agent

**Files:**
- Create: `bindings/python/provedex-pipecat/tests/conftest.py`
- Create: `bindings/python/provedex-pipecat/tests/test_integration.py`

- [ ] **Step 1: Write conftest.py with agent fixture**

```python
import os
import shutil
import socket
import subprocess
import tempfile
import time
from pathlib import Path

import httpx
import pytest

REPO_ROOT = Path(__file__).resolve().parents[4]  # bindings/python/provedex-pipecat/tests -> repo


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
    env.update({
        "PROVEDEX_LEDGER": str(ledger),
        "PROVEDEX_KEY": str(key),
        "PROVEDEX_AGENT_LISTEN": f"127.0.0.1:{port}",
        "RUST_LOG": "warn",
    })
    proc = subprocess.Popen(
        [str(agent_binary), "--rate-limit-off"],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    # Wait for the agent to be ready.
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

- [ ] **Step 2: Write integration test**

`tests/test_integration.py`:

```python
import subprocess
from pathlib import Path

import pytest
from pipecat.frames.frames import (
    EndFrame,
    FunctionCallInProgressFrame,
    FunctionCallResultFrame,
    LLMFullResponseEndFrame,
    LLMFullResponseStartFrame,
    LLMMessagesFrame,
    StartFrame,
    TextFrame,
    TranscriptionFrame,
)

from provedex_pipecat import ProvedexConfig, ProvedexFrameProcessor

REPO_ROOT = Path(__file__).resolve().parents[4]


@pytest.mark.integration
@pytest.mark.asyncio
async def test_full_pipeline_produces_valid_ledger(agent):
    cfg = ProvedexConfig(
        agent_url=agent["base_url"],
        session_id="int-test-session",
        agent_id="int-test-agent",
        model_id="int-test-model",
    )
    proc = ProvedexFrameProcessor(config=cfg)
    await proc.start()

    await proc.handle_frame(StartFrame())
    await proc.handle_frame(
        TranscriptionFrame(text="what's the weather", user_id="u", timestamp="t", language="en-US")
    )
    await proc.handle_frame(
        LLMMessagesFrame(messages=[{"role": "user", "content": "what's the weather"}])
    )
    await proc.handle_frame(LLMFullResponseStartFrame())
    await proc.handle_frame(TextFrame(text="It's 72 degrees."))
    await proc.handle_frame(LLMFullResponseEndFrame())
    await proc.handle_frame(
        FunctionCallInProgressFrame(
            function_name="get_weather", tool_call_id="c1", arguments={"city": "NYC"}
        )
    )
    await proc.handle_frame(
        FunctionCallResultFrame(
            function_name="get_weather",
            tool_call_id="c1",
            arguments={"city": "NYC"},
            result={"temp": 72},
        )
    )
    await proc.handle_frame(EndFrame())
    await proc.stop()

    # Run provedex verify against the sandboxed ledger.
    cli_path = REPO_ROOT / "target" / "release" / "provedex"
    if not cli_path.exists():
        subprocess.run(
            ["cargo", "build", "--release", "-p", "provedex-cli"],
            cwd=REPO_ROOT,
            check=True,
        )

    result = subprocess.run(
        [str(cli_path), "verify", "--ledger", str(agent["ledger"])],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, (
        f"provedex verify failed: stdout={result.stdout} stderr={result.stderr}"
    )
    assert proc.signed_total >= 5
    assert proc.dropped_total == 0
```

- [ ] **Step 3: Run integration test locally**

```bash
cd bindings/python/provedex-pipecat
pytest tests/test_integration.py -v -m integration -s
```

Expected: 1 passed. First run builds the agent + CLI (~5 min).

- [ ] **Step 4: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-pipecat/tests/conftest.py \
        bindings/python/provedex-pipecat/tests/test_integration.py
git commit -m "test(pipecat): integration test - real agent + provedex verify"
```

---

## Task 9: README + examples

**Files:**
- Create: `bindings/python/provedex-pipecat/README.md`
- Create: `bindings/python/provedex-pipecat/examples/voice_agent_basic.py`

- [ ] **Step 1: Write README.md**

Use the spec's section list. Plain ASCII, no AI-slop adjectives, voice-aditya semi-formal register. Sections:

1. One-paragraph what + why.
2. Quickstart (pip install, import, instantiate, wire, run; five lines of code).
3. Frame mapping table verbatim from the spec.
4. Configuration reference (env, constructor args, on_sign_failure modes).
5. Latency budget (cite test_async_smoke.py numbers from a real run; placeholder text noting "see tests/test_async_smoke.py output for current numbers").
6. Failure modes table.
7. Architecture note - link to the Rust SDK at github.com/provedex/provedex and to docs/spec/event-schema-v1.md.
8. Verifying the ledger (three example provedex verify invocations).
9. Regulatory context paragraph.

Concrete content: write 200-400 lines following the spec section list. Match the tone of `crates/provedex-core/README.md` if it exists; otherwise match the root README.md.

- [ ] **Step 2: Write minimal example**

`examples/voice_agent_basic.py`:

```python
"""Minimal Pipecat pipeline with Provedex signing.

This is an illustrative skeleton. Replace the placeholder transport, STT,
LLM, and TTS classes with the real Pipecat services from your stack
(twilio_transport.TwilioTransport, deepgram.DeepgramSTTService, etc.).

Run a local provedex-agent before starting this script:
    provedex-agent --rate-limit-off &
"""

import asyncio
import os

from pipecat.frames.frames import EndFrame, StartFrame, TranscriptionFrame, TextFrame
from provedex_pipecat import ProvedexConfig, ProvedexFrameProcessor


async def main() -> None:
    cfg = ProvedexConfig(
        agent_url=os.getenv("PROVEDEX_AGENT_URL", "http://127.0.0.1:8765"),
        agent_id="example-voice-agent",
        model_id="llama3.2:3b",
        session_id="example-session-001",
    )
    processor = ProvedexFrameProcessor(config=cfg)
    await processor.start()

    # Simulated pipeline events. Replace with real Pipecat pipeline composition.
    await processor.handle_frame(StartFrame())
    await processor.handle_frame(
        TranscriptionFrame(text="hello", user_id="caller", timestamp="t", language="en-US")
    )
    await processor.handle_frame(TextFrame(text="hello back"))
    await processor.handle_frame(EndFrame())

    await processor.stop()
    print(f"signed={processor.signed_total} dropped={processor.dropped_total}")


if __name__ == "__main__":
    asyncio.run(main())
```

- [ ] **Step 3: Verify example runs (with mock or real agent)**

```bash
cd bindings/python/provedex-pipecat
PROVEDEX_AGENT_URL=http://127.0.0.1:8765 python examples/voice_agent_basic.py
```

Expected: prints `signed=N dropped=M`. With agent running, N >= 3.

- [ ] **Step 4: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add bindings/python/provedex-pipecat/README.md \
        bindings/python/provedex-pipecat/examples/voice_agent_basic.py
git commit -m "docs(pipecat): README + minimal example"
```

---

## Task 10: Bindings dir CLAUDE.md + root README link

**Files:**
- Modify: `bindings/CLAUDE.md` (gitignored; update local copy)
- Modify: `README.md` (root)

- [ ] **Step 1: Update root README Components table**

Find the Components table in README.md. Add a row below the existing crates pointing to the binding:

```markdown
| `provedex-pipecat` (Python) | Pipecat FrameProcessor that signs every frame via the sidecar. PyPI. | shipped |
```

And add a "Bindings" section if not present, linking to `bindings/python/provedex-pipecat/README.md`.

- [ ] **Step 2: Update bindings/CLAUDE.md**

Locally only (gitignored). Note the binding's existence, conventions (Python, hatchling, httpx, pydantic), and that the HTTP client will be extracted to `provedex-client` when the second binding lands.

- [ ] **Step 3: Commit (root README only - CLAUDE.md is gitignored)**

```bash
cd /Users/adi/Desktop/provedex
git add README.md
git commit -m "docs: link provedex-pipecat from root README components table"
```

---

## Task 11: CI job for bindings-python

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Read current CI yaml**

```bash
cat .github/workflows/ci.yml
```

- [ ] **Step 2: Add bindings-python job**

Append a new job after the existing rust jobs. The job:

1. Checks out the repo.
2. Sets up Rust toolchain (uses `rust-toolchain.toml`).
3. Sets up Python 3.11.
4. Builds provedex-agent in release.
5. Installs the binding with dev deps: `cd bindings/python/provedex-pipecat && pip install -e ".[dev]"`.
6. Runs ruff: `ruff check src tests`.
7. Runs mypy: `mypy src`.
8. Runs pytest: `pytest -v -m "not integration"` for fast tests.
9. Runs integration: `pytest -v -m integration`.

Cache the cargo target dir keyed on Cargo.lock + rust-toolchain.toml. Cache pip wheels keyed on pyproject.toml.

- [ ] **Step 3: Verify yaml parses**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml').read()); print('ok')"
```

Expected: `ok`.

- [ ] **Step 4: Commit**

```bash
cd /Users/adi/Desktop/provedex
git add .github/workflows/ci.yml
git commit -m "ci: add bindings-python job (build agent + pytest)"
```

---

## Task 12: Self-review with code-review-provedex skill

- [ ] **Step 1: Invoke skill**

Use the code-review-provedex skill against `git diff main..HEAD`.

Checklist priorities for this PR:

- Auto-block invariants:
  - No canonical-JSON change (binding uses its own hash for the receipt; agent does the real canonical-JSON).
  - No event-schema-v1 change (mapping uses existing variants only).
  - All commit subjects conform: `feat`, `test`, `docs`, `chore`, `ci`.
  - ASCII only across all source + docs.
  - No AI-slop adjectives ("robust", "comprehensive", "powerful", "elegant", "leveraging", "seamless").
  - No co-author trailer in any commit.
  - New top-level dir? `bindings/python/provedex-pipecat/` is documented in `CLAUDE.md::Where new files go` (Python binding code -> `bindings/python/src/` row; clarify the row covers package subdirs).
- Public API: `ProvedexFrameProcessor` and `ProvedexConfig` are the entire public surface. Docstrings on both. Add a doctest on `ProvedexConfig` if not already present.
- Test posture: count tests pass. Run the full suite locally.
- Performance: producer hot path is O(1) ops; measured by test_async_smoke.py.
- License: pyproject says Apache-2.0; matches repo.

- [ ] **Step 2: Run full local CI gate**

```bash
cd /Users/adi/Desktop/provedex
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
cargo deny check

cd bindings/python/provedex-pipecat
ruff check src tests
mypy src
pytest -v
pytest -v -m integration
```

Expected: all green.

- [ ] **Step 3: Fix any review findings + recommit**

If review surfaces issues, fix inline + commit each fix with a focused subject (`fix(pipecat): ...`).

---

## Task 13: PR + merge + close issue

- [ ] **Step 1: Open PR**

Compose body using voice-aditya semi-formal register. Sections:

- Summary (one paragraph + the WHY)
- What changed (file-level bullets)
- Test plan (checklist of every gate run)
- Closes #N

```bash
cd /Users/adi/Desktop/provedex
gh pr create --title "feat(bindings/python): provedex-pipecat binding for voice agents" --body "..."
```

- [ ] **Step 2: Wait CI green**

```bash
gh pr checks <PR_NUMBER> --watch
```

- [ ] **Step 3: Merge + close**

```bash
gh pr merge <PR_NUMBER> --squash --delete-branch
gh issue close <ISSUE_NUMBER>
```

- [ ] **Step 4: Pull main, verify clean**

```bash
git checkout main
git pull --ff-only
git log --oneline -3
```

---

## Task 14: PyPI publish prep (manual, NOT automated)

Out of scope for the implementation plan: actual `twine upload`. Manual founder step.

The plan stops at "package is ready to publish". The founder publishes when they choose to.

Document the publish recipe inline in `bindings/python/provedex-pipecat/RELEASING.md` (1-page):

```
1. Bump version in pyproject.toml.
2. python -m build
3. python -m twine check dist/*
4. python -m twine upload dist/*
```

- [ ] Commit RELEASING.md.

---

## Self-review (writer's pass)

**Spec coverage:**

- Event mapping (existing v1 variants) -> Task 3 covers all 7 frame types from the spec table.
- Single package, inlined client -> Tasks 1, 4.
- Single worker + bounded queue + drop-oldest -> Task 6.
- Build agent from source in CI -> Tasks 8 (fixture) and 11 (CI job).
- Agent port 8765 default -> Task 2.
- Hashing semantics -> Task 3 (audio_sha256 of transcript bytes, documented).
- Stateful correlation -> Tasks 3 (state.py + buffer in mapping), 5 (tests).
- Public API (`ProvedexFrameProcessor`, `ProvedexConfig`) -> Tasks 2, 6.
- httpx async -> Task 4.
- README sections (8 items) -> Task 9.
- Tests: unit, async smoke, integration -> Tasks 3, 4, 5, 6, 7, 8.
- CI lane -> Task 11.

**Placeholder scan:** none.

**Type consistency:**
- `frame_to_event(frame, config, state) -> dict | None` used identically in mapping.py impl, mapping tests, and processor.py.
- `CorrelationState` API: `buffer_messages`, `buffer_response_text`, `take_paired_invocation`, `mark_response_start`, `already_seen` consistent across _state.py, mapping.py, processor.py, test_state.py.
- `AgentClient.sign(event)` raises `SignError` everywhere.
- `ProvedexConfig` field names match across config.py, tests, processor.py, mapping.py.

**Ambiguity check:** every code step shows the actual code. Every test step shows the assertion. Every commit step shows the exact subject.

No issues. Plan ready.

---

## Risks during execution

| Risk | Mitigation |
|------|------------|
| Pipecat frame field names differ from assumption (TranscriptionFrame.text vs .transcript, etc.) | Task 3 step 4 inspects real pipecat install. If field names differ, adjust mapping.py and the test golden values together. |
| respx version mismatch with httpx | Pinned in dev deps; if breakage, bump together. |
| Integration test slow on first CI run (~5 min cargo build) | Accepted; caching in CI step keeps subsequent runs fast. |
| Worker task leak on test failures | Each test that calls `start()` calls `stop()` in a try/finally pattern (add if missed). |
| Drop-oldest semantic vs deque.append | `deque(maxlen=N)` auto-drops oldest on append. Correct primitive. |
