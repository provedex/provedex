"""Pure functions translating LangChain callback args into AgentEvent dicts.

The output shape matches docs/spec/event-schema-v1.md:
    { "type": "<VariantName>", "payload": { ... } }
"""

from __future__ import annotations

import hashlib
import json
from typing import Any

from .config import ProvedexConfig


def _sha256_hex(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _canonical_json_bytes(value: Any) -> bytes:
    """Compact JSON with sorted keys. The actual cryptographic canonical JSON
    lives in the Rust agent; this hash is the binding's receipt of what it sent.
    """
    return json.dumps(value, sort_keys=True, separators=(",", ":"), default=str).encode()


def session_started_event(config: ProvedexConfig) -> dict[str, Any]:
    return {
        "type": "SessionStarted",
        "payload": {
            "agent_id": config.agent_id,
            "model_id": config.model_id,
            "session_id": config.session_id,
        },
    }


def session_ended_event(reason: str) -> dict[str, Any]:
    return {
        "type": "SessionEnded",
        "payload": {
            "reason": reason,
            "summary_sha256": _sha256_hex(b""),
        },
    }


def model_invoked_event(
    *,
    model_id: str,
    prompt_payload: Any,
    response_text: str,
    prompt_tokens: int | None,
    response_tokens: int | None,
) -> dict[str, Any]:
    return {
        "type": "ModelInvoked",
        "payload": {
            "model_id": model_id,
            "prompt_sha256": _sha256_hex(_canonical_json_bytes(prompt_payload)),
            "response_sha256": _sha256_hex(response_text.encode()),
            "prompt_tokens": prompt_tokens if prompt_tokens is not None else 0,
            "response_tokens": response_tokens if response_tokens is not None else 0,
        },
    }


def tool_called_event(*, tool_name: str, args: Any) -> dict[str, Any]:
    args_redacted: Any
    if isinstance(args, dict):
        args_redacted = args
    else:
        args_redacted = {"input": str(args)}
    return {
        "type": "ToolCalled",
        "payload": {
            "tool_name": tool_name,
            "args_sha256": _sha256_hex(_canonical_json_bytes(args_redacted)),
            "args_redacted": args_redacted,
        },
    }


def tool_returned_event(
    *, tool_name: str, result: Any, latency_ms: int, success: bool
) -> dict[str, Any]:
    return {
        "type": "ToolReturned",
        "payload": {
            "tool_name": tool_name,
            "result_sha256": _sha256_hex(_canonical_json_bytes(result)),
            "latency_ms": latency_ms,
            "success": success,
        },
    }
