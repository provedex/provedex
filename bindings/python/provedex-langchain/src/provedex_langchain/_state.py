"""Per-handler correlation buffer keyed on LangChain run_id."""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from typing import Any
from uuid import UUID


@dataclass
class CorrelationState:
    """Buffer in-flight LLM and tool calls keyed by LangChain run_id.

    LangChain emits paired callbacks (on_llm_start + on_llm_end, on_tool_start +
    on_tool_end) and assigns a UUID4 run_id to each pair. We buffer the start
    payload, then pair it with the end payload when the second callback fires.
    """

    llm_buffer: dict[UUID, dict[str, Any]] = field(default_factory=dict)
    tool_buffer: dict[UUID, dict[str, Any]] = field(default_factory=dict)

    def buffer_llm_start(
        self, run_id: UUID, *, model_id: str, prompt_payload: Any
    ) -> None:
        self.llm_buffer[run_id] = {
            "model_id": model_id,
            "prompt_payload": prompt_payload,
            "start_time": time.monotonic(),
        }

    def take_llm(self, run_id: UUID) -> dict[str, Any] | None:
        return self.llm_buffer.pop(run_id, None)

    def buffer_tool_start(
        self, run_id: UUID, *, tool_name: str, args: Any
    ) -> None:
        self.tool_buffer[run_id] = {
            "tool_name": tool_name,
            "args": args,
            "start_time": time.monotonic(),
        }

    def take_tool(self, run_id: UUID) -> dict[str, Any] | None:
        return self.tool_buffer.pop(run_id, None)
