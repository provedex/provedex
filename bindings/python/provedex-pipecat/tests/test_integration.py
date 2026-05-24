import subprocess
from pathlib import Path

import pytest
from pipecat.frames.frames import (
    EndFrame,
    FunctionCallInProgressFrame,
    FunctionCallResultFrame,
    LLMFullResponseEndFrame,
    LLMFullResponseStartFrame,
    LLMMessagesFrame,
    StartFrame,
    TextFrame,
    TranscriptionFrame,
)

from provedex_pipecat import ProvedexConfig, ProvedexFrameProcessor

REPO_ROOT = Path(__file__).resolve().parents[4]


@pytest.mark.integration
@pytest.mark.asyncio
async def test_full_pipeline_produces_valid_ledger(agent):
    cfg = ProvedexConfig(
        agent_url=agent["base_url"],
        session_id="int-test-session",
        agent_id="int-test-agent",
        model_id="int-test-model",
    )
    proc = ProvedexFrameProcessor(config=cfg)
    await proc.start()

    await proc.handle_frame(StartFrame())
    await proc.handle_frame(
        TranscriptionFrame(text="what's the weather", user_id="u", timestamp="t", language="en-US")
    )
    await proc.handle_frame(
        LLMMessagesFrame(messages=[{"role": "user", "content": "what's the weather"}])
    )
    await proc.handle_frame(LLMFullResponseStartFrame())
    await proc.handle_frame(TextFrame(text="It's 72 degrees."))
    await proc.handle_frame(LLMFullResponseEndFrame())
    await proc.handle_frame(
        FunctionCallInProgressFrame(
            function_name="get_weather", tool_call_id="c1", arguments={"city": "NYC"}
        )
    )
    await proc.handle_frame(
        FunctionCallResultFrame(
            function_name="get_weather",
            tool_call_id="c1",
            arguments={"city": "NYC"},
            result={"temp": 72},
        )
    )
    await proc.handle_frame(EndFrame())
    await proc.stop()

    # Verify the ledger chain integrity using the CLI.
    cli_path = REPO_ROOT / "target" / "release" / "provedex"
    if not cli_path.exists():
        subprocess.run(
            ["cargo", "build", "--release", "-p", "provedex-cli"],
            cwd=REPO_ROOT,
            check=True,
        )

    result = subprocess.run(
        [str(cli_path), "verify", "--ledger", str(agent["ledger"])],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, (
        f"provedex verify failed: stdout={result.stdout} stderr={result.stderr}"
    )
    assert proc.signed_total >= 5
    assert proc.dropped_total == 0
