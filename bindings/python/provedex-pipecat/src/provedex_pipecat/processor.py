"""ProvedexFrameProcessor: signs every supported Pipecat Frame via the local agent."""

from __future__ import annotations

import asyncio
import logging
import time
import weakref
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
        self._worker_task: asyncio.Task | None = None  # type: ignore[type-arg]
        self._stopping = False
        self._last_overflow_warn_ts: float = 0.0

        # Tracks frame identity using weak references so that GC-reclaimed
        # frame addresses cannot cause false-positive dedup hits for a new
        # frame that happens to reuse the same memory address.
        self._seen_frames: weakref.WeakValueDictionary[int, Frame] = (
            weakref.WeakValueDictionary()
        )

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
            except TimeoutError:
                self._worker_task.cancel()
        await self._client.aclose()

    async def handle_frame(self, frame: Frame) -> None:
        """Test entry point. In a real pipeline Pipecat calls process_frame."""
        await self._enqueue_for_frame(frame)

    async def process_frame(self, frame: Frame, direction: FrameDirection) -> None:
        """Standard Pipecat hook. Enqueues a sign and forwards the frame downstream."""
        await self._enqueue_for_frame(frame)
        await self.push_frame(frame, direction)

    async def _enqueue_for_frame(self, frame: Frame) -> None:
        # Dedup by Python object identity. Pipecat may route the same frame
        # instance through a multi-placed processor. We use a WeakValueDictionary
        # so GC-reclaimed frame addresses do not trigger false positives when a
        # new frame happens to reuse the same memory address.
        frame_id = id(frame)
        if frame_id in self._seen_frames:
            return
        self._seen_frames[frame_id] = frame

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
                self._handle_sign_failure(e, event)

    def _handle_sign_failure(self, exc: SignError, event: dict[str, Any]) -> None:
        mode = self._config.on_sign_failure
        if mode == "warn":
            logger.warning(
                "provedex sign failed for %s: %s",
                event.get("type", "<unknown>"),
                exc,
            )
        # mode == "silent": no log

    async def _drain_with_timeout(self) -> None:
        deadline = time.monotonic() + self._config.shutdown_drain_seconds
        while self._queue and time.monotonic() < deadline:
            self._wakeup.set()
            # yield to worker so it can drain the queue; do NOT use time.sleep here.
            await asyncio.sleep(0.01)
