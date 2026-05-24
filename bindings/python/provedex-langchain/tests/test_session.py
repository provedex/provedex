from collections import Counter
import json

import httpx
import pytest
import respx

from provedex_langchain import ProvedexCallbackHandler, ProvedexConfig


def _capture():
    posted = []

    def record(request):
        posted.append(json.loads(request.content))
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
