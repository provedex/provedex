import hashlib
import json

import pytest
from pipecat.frames.frames import (
    EndFrame,
    Frame,
    FunctionCallInProgressFrame,
    FunctionCallResultFrame,
    LLMFullResponseEndFrame,
    LLMMessagesFrame,
    StartFrame,
    TextFrame,
    TranscriptionFrame,
)

from provedex_pipecat.config import ProvedexConfig
from provedex_pipecat._state import CorrelationState
from provedex_pipecat.mapping import frame_to_event


def _sha256_hex(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


@pytest.fixture
def config():
    return ProvedexConfig(
        agent_url="http://127.0.0.1:8765",
        session_id="test-session",
        agent_id="test-agent",
        model_id="test-model",
    )


@pytest.fixture
def state():
    return CorrelationState()


def test_start_frame_maps_to_session_started(config, state):
    event = frame_to_event(StartFrame(), config, state)
    assert event == {
        "type": "SessionStarted",
        "payload": {
            "agent_id": "test-agent",
            "model_id": "test-model",
            "session_id": "test-session",
        },
    }


def test_end_frame_maps_to_session_ended(config, state):
    event = frame_to_event(EndFrame(), config, state)
    assert event == {
        "type": "SessionEnded",
        "payload": {
            "reason": "pipeline_end",
            "summary_sha256": _sha256_hex(b""),
        },
    }


def test_transcription_frame_maps_to_utterance_captured(config, state):
    frame = TranscriptionFrame(text="hello world", user_id="u1", timestamp="2026-05-24T00:00:00Z", language="en-US")
    event = frame_to_event(frame, config, state)
    assert event["type"] == "UtteranceCaptured"
    assert event["payload"]["transcript"] == "hello world"
    assert event["payload"]["lang"] == "en-US"
    assert event["payload"]["audio_sha256"] == _sha256_hex(b"hello world")
    assert event["payload"]["duration_ms"] == 0


def test_function_call_in_progress_maps_to_tool_called(config, state):
    frame = FunctionCallInProgressFrame(
        function_name="get_weather",
        tool_call_id="call_1",
        arguments={"city": "NYC"},
    )
    event = frame_to_event(frame, config, state)
    assert event["type"] == "ToolCalled"
    assert event["payload"]["tool_name"] == "get_weather"
    assert event["payload"]["args_redacted"] == {"city": "NYC"}
    expected_args_hash = _sha256_hex(json.dumps({"city": "NYC"}, sort_keys=True, separators=(",", ":")).encode())
    assert event["payload"]["args_sha256"] == expected_args_hash


def test_function_call_result_maps_to_tool_returned(config, state):
    frame = FunctionCallResultFrame(
        function_name="get_weather",
        tool_call_id="call_1",
        arguments={"city": "NYC"},
        result={"temp": 72},
    )
    event = frame_to_event(frame, config, state)
    assert event["type"] == "ToolReturned"
    assert event["payload"]["tool_name"] == "get_weather"
    assert event["payload"]["success"] is True
    assert event["payload"]["latency_ms"] == 0
    expected_result_hash = _sha256_hex(
        json.dumps({"temp": 72}, sort_keys=True, separators=(",", ":")).encode()
    )
    assert event["payload"]["result_sha256"] == expected_result_hash


def test_llm_messages_frame_alone_does_not_emit(config, state):
    messages = [{"role": "user", "content": "hi"}]
    event = frame_to_event(LLMMessagesFrame(messages=messages), config, state)
    assert event is None  # buffered, no signed event yet


def test_llm_messages_then_full_response_end_pairs(config, state):
    messages = [{"role": "user", "content": "hi"}]
    frame_to_event(LLMMessagesFrame(messages=messages), config, state)  # buffer

    end = LLMFullResponseEndFrame()
    # Pipecat carries the final text on a TextFrame between Start and End.
    # The state buffers the TextFrame text alongside the messages.
    state.buffer_response_text("hello back")

    event = frame_to_event(end, config, state)
    assert event["type"] == "ModelInvoked"
    assert event["payload"]["model_id"] == "test-model"
    expected_prompt = _sha256_hex(
        json.dumps(messages, sort_keys=True, separators=(",", ":")).encode()
    )
    assert event["payload"]["prompt_sha256"] == expected_prompt
    assert event["payload"]["response_sha256"] == _sha256_hex(b"hello back")
    assert event["payload"]["prompt_tokens"] == 0
    assert event["payload"]["response_tokens"] == 0


def test_text_frame_without_pairing_maps_to_utterance_spoken(config, state):
    event = frame_to_event(TextFrame(text="hi there"), config, state)
    assert event["type"] == "UtteranceSpoken"
    assert event["payload"]["text"] == "hi there"
    assert event["payload"]["text_sha256"] == _sha256_hex(b"hi there")
    assert event["payload"]["audio_sha256"] == _sha256_hex(b"")


def test_unknown_frame_returns_none(config, state):
    class WeirdFrame(Frame):
        pass

    assert frame_to_event(WeirdFrame(), config, state) is None
