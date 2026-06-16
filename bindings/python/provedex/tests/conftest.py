import shutil
import subprocess
from pathlib import Path

import pytest

# tests/ -> provedex/ -> python/ -> bindings/ -> repo root
_REPO_ROOT = Path(__file__).resolve().parents[4]


@pytest.fixture(scope="session")
def provedex_cli() -> str:
    """Build the provedex CLI once and return its path. Skips integration tests
    if cargo is unavailable."""
    if shutil.which("cargo") is None:
        pytest.skip("cargo not available; cannot build the provedex CLI")
    subprocess.run(
        ["cargo", "build", "--release", "-p", "provedex-cli"],
        cwd=_REPO_ROOT,
        check=True,
    )
    binary = _REPO_ROOT / "target" / "release" / "provedex"
    assert binary.exists(), f"CLI not found at {binary}"
    return str(binary)
