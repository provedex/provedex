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
