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
