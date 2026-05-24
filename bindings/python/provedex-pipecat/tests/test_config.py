import pytest
from pydantic import ValidationError

from provedex_pipecat.config import ProvedexConfig


def test_defaults_with_no_env(monkeypatch):
    monkeypatch.delenv("PROVEDEX_AGENT_URL", raising=False)
    cfg = ProvedexConfig()
    assert cfg.agent_url == "http://127.0.0.1:8765"
    assert cfg.agent_id == "pipecat-agent"
    assert cfg.model_id == "unknown"
    assert cfg.queue_size == 1000
    assert cfg.request_timeout_seconds == 2.0
    assert cfg.shutdown_drain_seconds == 5.0
    assert cfg.on_sign_failure == "warn"
    assert cfg.session_id  # auto-generated, non-empty
    assert cfg.include_frames is None


def test_env_overrides_url(monkeypatch):
    monkeypatch.setenv("PROVEDEX_AGENT_URL", "http://10.0.0.5:9999")
    cfg = ProvedexConfig()
    assert cfg.agent_url == "http://10.0.0.5:9999"


def test_constructor_overrides_env(monkeypatch):
    monkeypatch.setenv("PROVEDEX_AGENT_URL", "http://7.7.7.7:7777")
    cfg = ProvedexConfig(agent_url="http://1.2.3.4:1234")
    assert cfg.agent_url == "http://1.2.3.4:1234"


def test_on_sign_failure_invalid_rejected():
    with pytest.raises(ValidationError):
        ProvedexConfig(on_sign_failure="explode")


def test_agent_url_must_be_http():
    with pytest.raises(ValidationError):
        ProvedexConfig(agent_url="ws://127.0.0.1:8765")
