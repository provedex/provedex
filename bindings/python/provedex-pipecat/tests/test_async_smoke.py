"""Async smoke tests for producer latency budget."""

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
