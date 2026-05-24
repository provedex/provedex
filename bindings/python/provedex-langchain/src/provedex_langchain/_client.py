"""Private async HTTP client for the provedex-agent /v1/sign endpoint."""

from __future__ import annotations

from typing import Any

import httpx


class SignError(Exception):
    """Raised when a sign attempt fails (network, timeout, or non-2xx)."""


class AgentClient:
    """Thin httpx wrapper. One per handler instance; reuses the connection."""

    def __init__(self, base_url: str, timeout: float) -> None:
        self._base_url = base_url.rstrip("/")
        self._client = httpx.AsyncClient(
            base_url=self._base_url,
            timeout=httpx.Timeout(timeout, connect=timeout),
            headers={"content-type": "application/json"},
        )

    async def sign(self, event: dict[str, Any]) -> None:
        """POST {event: ...} to /v1/sign. Raises SignError on any failure."""
        try:
            resp = await self._client.post("/v1/sign", json={"event": event})
        except httpx.HTTPError as e:
            raise SignError(f"agent unreachable: {e}") from e
        if resp.status_code >= 400:
            raise SignError(f"agent returned {resp.status_code}: {resp.text[:200]}")

    async def aclose(self) -> None:
        await self._client.aclose()
