"""Pure functions translating Pipecat Frames to AgentEvent dicts.

The output shape matches docs/spec/event-schema-v1.md:
    { "type": "<VariantName>", "payload": { ... } }
"""

from __future__ import annotations

import hashlib
import json
from typing import Any

from pipecat.frames.frames import (
    EndFrame,
    Frame,
    FunctionCallInProgressFrame,
    FunctionCallResultFrame,
    LLMFullResponseEndFrame,
    LLMFullResponseStartFrame,
    LLMMessagesFrame,
    StartFrame,
    TextFrame,
    TranscriptionFrame,
)

from ._state import CorrelationState
from .config import ProvedexConfig


def _sha256_hex(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _canonical_json_bytes(value: Any) -> bytes:
    """Compact JSON with sorted keys. Matches the spirit of canonical-JSON for
    hashing purposes inside the Python binding. The actual cryptographic
    canonical JSON lives in the Rust agent; this hash is just the binding's
    receipt of what it sent.
    """
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def frame_to_event(
    frame: Frame,
    config: ProvedexConfig,
    state: CorrelationState,
) -> dict[str, Any] | None:
    """Translate a Pipecat Frame to an AgentEvent dict, or None to skip."""

    if isinstance(frame, StartFrame):
        return {
            "type": "SessionStarted",
            "payload": {
                "agent_id": config.agent_id,
                "model_id": config.model_id,
                "session_id": config.session_id,
            },
        }

    if isinstance(frame, EndFrame):
        return {
            "type": "SessionEnded",
            "payload": {
                "reason": "pipeline_end",
                "summary_sha256": _sha256_hex(b""),
            },
        }

    if isinstance(frame, TranscriptionFrame):
        transcript = frame.text or ""
        lang = getattr(frame, "language", "") or ""
        # Language may be a pipecat Language enum; .value gives the BCP-47 string.
        if hasattr(lang, "value"):
            lang = lang.value
        return {
            "type": "UtteranceCaptured",
            "payload": {
                "audio_sha256": _sha256_hex(transcript.encode()),
                "transcript": transcript,
                "lang": lang,
                "duration_ms": 0,
            },
        }

    if isinstance(frame, FunctionCallInProgressFrame):
        args = frame.arguments or {}
        return {
            "type": "ToolCalled",
            "payload": {
                "tool_name": frame.function_name,
                "args_sha256": _sha256_hex(_canonical_json_bytes(args)),
                "args_redacted": args,
            },
        }

    if isinstance(frame, FunctionCallResultFrame):
        result = frame.result if frame.result is not None else {}
        return {
            "type": "ToolReturned",
            "payload": {
                "tool_name": frame.function_name,
                "result_sha256": _sha256_hex(_canonical_json_bytes(result)),
                "latency_ms": 0,
                "success": True,
            },
        }

    if isinstance(frame, LLMMessagesFrame):
        # Buffer for pairing with the upcoming LLMFullResponseEndFrame.
        state.buffer_messages(list(frame.messages))
        return None

    if isinstance(frame, LLMFullResponseStartFrame):
        state.mark_response_start()
        return None

    if isinstance(frame, LLMFullResponseEndFrame):
        messages, response_text = state.take_paired_invocation()
        if messages is None:
            # End without preceding messages buffer; cannot construct a
            # ModelInvoked. Drop quietly.
            return None
        return {
            "type": "ModelInvoked",
            "payload": {
                "model_id": config.model_id,
                "prompt_sha256": _sha256_hex(_canonical_json_bytes(messages)),
                "response_sha256": _sha256_hex(response_text.encode()),
                "prompt_tokens": 0,
                "response_tokens": 0,
            },
        }

    if isinstance(frame, TextFrame):
        if state.response_in_progress:
            # Accumulate as part of the in-flight LLM response.
            state.buffer_response_text(frame.text)
            return None
        # Standalone text frame; treat as UtteranceSpoken (TTS-bound text).
        return {
            "type": "UtteranceSpoken",
            "payload": {
                "text_sha256": _sha256_hex(frame.text.encode()),
                "text": frame.text,
                "audio_sha256": _sha256_hex(b""),
            },
        }

    return None
