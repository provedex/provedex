import provedex
import pytest


def test_each_factory_builds_an_event():
    assert provedex.events.session_started(
        agent_id="a", model_id="m", session_id="s"
    ) is not None
    assert provedex.events.utterance_captured(
        audio_sha256="0" * 64, transcript="hi", lang="en", duration_ms=10
    ) is not None
    assert provedex.events.tool_called(
        tool_name="search", args_sha256="0" * 64, args_redacted={"q": "x"}
    ) is not None
    assert provedex.events.tool_returned(
        tool_name="search", result_sha256="0" * 64, latency_ms=5, success=True
    ) is not None
    assert provedex.events.model_invoked(
        model_id="m", prompt_sha256="0" * 64, response_sha256="0" * 64,
        prompt_tokens=5, response_tokens=2,
    ) is not None
    assert provedex.events.utterance_spoken(
        text_sha256="0" * 64, text="hello", audio_sha256="0" * 64
    ) is not None
    assert provedex.events.session_ended(
        reason="done", summary_sha256="0" * 64
    ) is not None


def test_from_dict_roundtrips_a_known_variant():
    d = {"type": "SessionEnded", "payload": {"reason": "done", "summary_sha256": "x"}}
    e = provedex.events.from_dict(d)
    assert e is not None


def test_from_dict_rejects_unknown_variant():
    with pytest.raises(provedex.SigningError):
        provedex.events.from_dict({"type": "NotAVariant", "payload": {}})
