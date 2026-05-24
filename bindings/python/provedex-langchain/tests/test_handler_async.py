import json
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
        posted.append(json.loads(request.content))
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
