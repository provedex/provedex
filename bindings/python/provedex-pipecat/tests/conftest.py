import os
import socket
import subprocess
import time
from pathlib import Path

import httpx
import pytest

REPO_ROOT = Path(__file__).resolve().parents[4]  # tests -> provedex-pipecat -> python -> bindings -> repo


def _free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@pytest.fixture(scope="session")
def agent_binary() -> Path:
    """Build provedex-agent in release once per test session."""
    target = REPO_ROOT / "target" / "release" / "provedex-agent"
    if not target.exists():
        subprocess.run(
            ["cargo", "build", "--release", "-p", "provedex-agent"],
            cwd=REPO_ROOT,
            check=True,
        )
    return target


@pytest.fixture
def agent(agent_binary, tmp_path):
    """Spawn provedex-agent on a random port with a sandboxed ledger."""
    port = _free_port()
    ledger = tmp_path / "ledger.ndjson"
    key = tmp_path / "ed25519.key"

    env = os.environ.copy()
    env.update({
        "PROVEDEX_LEDGER": str(ledger),
        "PROVEDEX_KEY": str(key),
        "PROVEDEX_AGENT_LISTEN": f"127.0.0.1:{port}",
        "RUST_LOG": "warn",
    })
    proc = subprocess.Popen(
        [str(agent_binary), "--rate-limit-off"],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    base_url = f"http://127.0.0.1:{port}"
    for _ in range(50):
        try:
            r = httpx.get(f"{base_url}/v1/healthz", timeout=0.5)
            if r.status_code == 200:
                break
        except httpx.HTTPError:
            pass
        time.sleep(0.1)
    else:
        proc.kill()
        out, err = proc.communicate()
        raise RuntimeError(f"agent failed to start: {err.decode()[:500]}")

    yield {"base_url": base_url, "ledger": ledger}

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
