"""Configuration for the Provedex LangChain binding."""

from __future__ import annotations

import os
import uuid
from typing import Literal

from pydantic import BaseModel, Field, field_validator

OnSignFailure = Literal["warn", "raise", "silent"]


class ProvedexConfig(BaseModel):
    """Configuration for ProvedexCallbackHandler.

    Env-first with constructor overrides. PROVEDEX_AGENT_URL is the only
    runtime-discovered field; everything else is set explicitly by the operator.
    """

    agent_url: str = Field(
        default_factory=lambda: os.getenv("PROVEDEX_AGENT_URL", "http://127.0.0.1:8765")
    )
    session_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    agent_id: str = "langchain-agent"
    model_id: str = "unknown"
    include_callbacks: list[str] | None = None
    on_sign_failure: OnSignFailure = "warn"
    queue_size: int = Field(default=1000, ge=1)
    request_timeout_seconds: float = Field(default=2.0, gt=0)
    shutdown_drain_seconds: float = Field(default=5.0, ge=0)

    @field_validator("agent_url")
    @classmethod
    def url_must_be_http(cls, v: str) -> str:
        if not v.startswith(("http://", "https://")):
            raise ValueError(f"agent_url must start with http:// or https://, got {v!r}")
        return v
