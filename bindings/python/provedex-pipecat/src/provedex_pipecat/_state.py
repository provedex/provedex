"""Per-processor correlation buffer for paired LLM frames + frame dedup."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass
class CorrelationState:
    """Track in-flight LLM exchanges and seen frame IDs."""

    last_messages: list[dict[str, Any]] | None = None
    pending_response_text: str = ""
    response_in_progress: bool = False
    seen_frame_ids: set[int] = field(default_factory=set)

    def buffer_messages(self, messages: list[dict[str, Any]]) -> None:
        self.last_messages = messages

    def buffer_response_text(self, text: str) -> None:
        self.pending_response_text += text

    def take_paired_invocation(self) -> tuple[list[dict[str, Any]] | None, str]:
        """Return (messages, response_text) and clear the buffers."""
        messages = self.last_messages
        text = self.pending_response_text
        self.last_messages = None
        self.pending_response_text = ""
        self.response_in_progress = False
        return messages, text

    def mark_response_start(self) -> None:
        self.response_in_progress = True
        self.pending_response_text = ""

    def already_seen(self, frame_id: int) -> bool:
        if frame_id in self.seen_frame_ids:
            return True
        self.seen_frame_ids.add(frame_id)
        return False
