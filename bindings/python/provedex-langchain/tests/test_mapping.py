import hashlib
import json

from provedex_langchain.config import ProvedexConfig
from provedex_langchain.mapping import (
    model_invoked_event,
    session_ended_event,
    session_started_event,
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
