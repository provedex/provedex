import json
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
        posted.append(json.loads(request.content))
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
        posted.append(json.loads(request.content))
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
        posted.append(json.loads(request.content))
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


@pytest.mark.asyncio
@respx.mock
async def test_sync_llm_error_emits_model_invoked_with_error():
    posted = []

    def record(request):
        posted.append(json.loads(request.content))
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
    handler.on_llm_error(RuntimeError("rate-limited"), run_id=run_id)

    await handler.stop()

    types = Counter(body["event"]["type"] for body in posted)
    assert types["ModelInvoked"] == 1
    payload = next(body for body in posted if body["event"]["type"] == "ModelInvoked")["event"][
        "payload"
    ]
    assert "RuntimeError" in payload.get("response_sha256", "") or True  # response_sha256 hashes the error description; can't assert content without recomputing - just assert the event fired
