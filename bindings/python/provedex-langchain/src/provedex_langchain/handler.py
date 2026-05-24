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


class _Awaitable:
    """Trivial awaitable returned by sync callback methods.

    Sync callbacks enqueue events directly; the caller may discard this.
    Returning it lets async callers do `await handler.on_llm_start(...)` without
    needing a second async definition for every method. Unlike a coroutine,
    discarding this object does not produce a RuntimeWarning.
    """

    __slots__ = ()

    def __await__(self):
        return iter(())


_DONE = _Awaitable()


class ProvedexCallbackHandler(AsyncCallbackHandler, BaseCallbackHandler):
    """LangChain callback handler that signs every LLM and tool call.

    Implements both sync (BaseCallbackHandler) and async (AsyncCallbackHandler)
    callback interfaces. Sync `on_*` methods enqueue directly and return an
    awaitable no-op so async callers can `await` them unchanged. A single
    background asyncio worker drains a bounded deque and POSTs to /v1/sign.

    Session lifecycle is operator-driven via start_session() / end_session()
    or the session() context manager.
    """

    # Required by LangChain's callback dispatch to surface raise mode.
    raise_error: bool = False

    def __init__(self, *, config: ProvedexConfig) -> None:
        # BaseCallbackHandler is not a dataclass; plain __init__ is fine.
        super().__init__()
        self._config = config
        self._client = AgentClient(
            base_url=config.agent_url,
            timeout=config.request_timeout_seconds,
        )
        self._state = CorrelationState()
        self._queue: deque[dict[str, Any]] = deque(maxlen=config.queue_size)
        self._wakeup = asyncio.Event()
        self._worker_task: asyncio.Task[None] | None = None
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

    def session(self, reason: str = "operator_session") -> "_SessionContext":
        """Return a context manager that wraps a session lifecycle.

        Both `with handler.session(reason):` and
        `async with handler.session(reason):` are supported.
        """
        return _SessionContext(self, reason)

    # --- sync callbacks (BaseCallbackHandler) -----------------------------
    #
    # Each method enqueues synchronously and returns _noop() so that async
    # callers can `await handler.on_*(...)` without needing a second async
    # definition. LangChain's CallbackManager calls these from sync or async
    # context; returning a coroutine from a sync method is harmless when the
    # return value is discarded.

    def on_llm_start(  # type: ignore[override]
        self,
        serialized: dict[str, Any],
        prompts: list[str],
        *,
        run_id: UUID,
        **kwargs: Any,
    ) -> "_Awaitable":
        model_id = self._derive_model_id(serialized)
        self._state.buffer_llm_start(
            run_id, model_id=model_id, prompt_payload=prompts
        )
        return _DONE

    def on_chat_model_start(  # type: ignore[override]
        self,
        serialized: dict[str, Any],
        messages: list[list[BaseMessage]],
        *,
        run_id: UUID,
        **kwargs: Any,
    ) -> "_Awaitable":
        model_id = self._derive_model_id(serialized)
        flattened = [
            {"type": m.type, "content": m.content}
            for batch in messages
            for m in batch
        ]
        self._state.buffer_llm_start(
            run_id, model_id=model_id, prompt_payload=flattened
        )
        return _DONE

    def on_llm_end(  # type: ignore[override]
        self, response: LLMResult, *, run_id: UUID, **kwargs: Any
    ) -> "_Awaitable":
        self._emit_model_invoked(run_id, response)
        return _DONE

    def on_llm_error(  # type: ignore[override]
        self, error: BaseException, *, run_id: UUID, **kwargs: Any
    ) -> "_Awaitable":
        self._emit_model_invoked_error(run_id, error)
        return _DONE

    def on_tool_start(  # type: ignore[override]
        self,
        serialized: dict[str, Any],
        input_str: str,
        *,
        run_id: UUID,
        inputs: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> "_Awaitable":
        tool_name = serialized.get("name", "unknown")
        args = inputs if inputs is not None else input_str
        self._state.buffer_tool_start(run_id, tool_name=tool_name, args=args)
        self._enqueue(tool_called_event(tool_name=tool_name, args=args))
        return _DONE

    def on_tool_end(  # type: ignore[override]
        self, output: Any, *, run_id: UUID, **kwargs: Any
    ) -> "_Awaitable":
        self._emit_tool_returned(run_id, output, success=True)
        return _DONE

    def on_tool_error(  # type: ignore[override]
        self, error: BaseException, *, run_id: UUID, **kwargs: Any
    ) -> "_Awaitable":
        description = f"{type(error).__name__}: {error}"
        self._emit_tool_returned(run_id, description, success=False)
        return _DONE

    # --- helpers ----------------------------------------------------------

    def _derive_model_id(self, serialized: dict[str, Any]) -> str:
        """LangChain's serialized.id is a path list ending in the class name.
        Fall back to the configured default if the path is absent or empty.
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
        # Wake the worker without blocking the caller.
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

    def __exit__(self, exc_type: Any, exc: Any, tb: Any) -> None:
        reason = self._reason if exc is None else f"exception:{exc_type.__name__}"
        self._handler.end_session(reason=reason)

    async def __aenter__(self) -> ProvedexCallbackHandler:
        await self._handler.start()
        self._handler.start_session()
        return self._handler

    async def __aexit__(self, exc_type: Any, exc: Any, tb: Any) -> None:
        reason = self._reason if exc is None else f"exception:{exc_type.__name__}"
        self._handler.end_session(reason=reason)
