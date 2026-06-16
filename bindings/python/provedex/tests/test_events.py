import provedex
import pytest


def test_each_factory_builds_the_right_variant():
    assert "SessionStarted" in repr(
        provedex.events.session_started(agent_id="a", model_id="m", session_id="s")
    )
    assert "UtteranceCaptured" in repr(
        provedex.events.utterance_captured(
            audio_sha256="0" * 64, transcript="hi", lang="en", duration_ms=10
        )
    )
    assert "ToolCalled" in repr(
        provedex.events.tool_called(
            tool_name="search", args_sha256="0" * 64, args_redacted={"q": "x"}
        )
    )
    assert "ToolReturned" in repr(
        provedex.events.tool_returned(
            tool_name="search", result_sha256="0" * 64, latency_ms=5, success=True
        )
    )
    assert "ModelInvoked" in repr(
        provedex.events.model_invoked(
            model_id="m", prompt_sha256="0" * 64, response_sha256="0" * 64,
            prompt_tokens=5, response_tokens=2,
        )
    )
    assert "UtteranceSpoken" in repr(
        provedex.events.utterance_spoken(
            text_sha256="0" * 64, text="hello", audio_sha256="0" * 64
        )
    )
    assert "SessionEnded" in repr(
        provedex.events.session_ended(reason="done", summary_sha256="0" * 64)
    )


def test_from_dict_rebuilds_the_named_variant():
    e = provedex.events.from_dict(
        {"type": "SessionEnded", "payload": {"reason": "done", "summary_sha256": "x"}}
    )
    assert "SessionEnded" in repr(e)


def test_from_dict_rejects_unknown_variant():
    with pytest.raises(provedex.SigningError):
        provedex.events.from_dict({"type": "NotAVariant", "payload": {}})
