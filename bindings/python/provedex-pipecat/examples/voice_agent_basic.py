"""Minimal Pipecat pipeline with Provedex signing.

This is an illustrative skeleton. Replace the placeholder transport, STT,
LLM, and TTS classes with the real Pipecat services from your stack
(twilio_transport.TwilioTransport, deepgram.DeepgramSTTService, etc.).

Run a local provedex-agent before starting this script:
    provedex-agent --rate-limit-off &
"""

import asyncio
import os

from pipecat.frames.frames import EndFrame, StartFrame, TranscriptionFrame, TextFrame
from provedex_pipecat import ProvedexConfig, ProvedexFrameProcessor


async def main() -> None:
    cfg = ProvedexConfig(
        agent_url=os.getenv("PROVEDEX_AGENT_URL", "http://127.0.0.1:8765"),
        agent_id="example-voice-agent",
        model_id="llama3.2:3b",
        session_id="example-session-001",
    )
    processor = ProvedexFrameProcessor(config=cfg)
    await processor.start()

    # Simulated pipeline events. Replace with real Pipecat pipeline composition.
    await processor.handle_frame(StartFrame())
    await processor.handle_frame(
        TranscriptionFrame(text="hello", user_id="caller", timestamp="t", language="en-US")
    )
    await processor.handle_frame(TextFrame(text="hello back"))
    await processor.handle_frame(EndFrame())

    await processor.stop()
    print(f"signed={processor.signed_total} dropped={processor.dropped_total}")


if __name__ == "__main__":
    asyncio.run(main())
