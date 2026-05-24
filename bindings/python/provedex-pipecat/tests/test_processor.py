"""Tests for ProvedexFrameProcessor."""

import json
from collections import Counter

import httpx
import respx
from pipecat.frames.frames import EndFrame, StartFrame, TranscriptionFrame

from provedex_pipecat import ProvedexConfig, ProvedexFrameProcessor


@respx.mock
async def test_processor_signs_start_then_end():
    posted = []

    def record(request):
        posted.append(json.loads(request.content))
        return httpx.Response(200, json={"seq": 0, "self_hash": "x"})

    respx.post("http://127.0.0.1:8765/v1/sign").mock(side_effect=record)

    cfg = ProvedexConfig(agent_url="http://127.0.0.1:8765", session_id="s1")
    proc = ProvedexFrameProcessor(config=cfg)
    await proc.start()

    await proc.handle_frame(StartFrame())
    await proc.handle_frame(
        TranscriptionFrame(text="hi", user_id="u", timestamp="t", language=None)
    )
    await proc.handle_frame(EndFrame())

    await proc.stop()

    types = Counter(body["event"]["type"] for body in posted)
    assert types["SessionStarted"] == 1
    assert types["UtteranceCaptured"] == 1
    assert types["SessionEnded"] == 1


@respx.mock
async def test_processor_drops_when_agent_unreachable():
    respx.post("http://127.0.0.1:8765/v1/sign").mock(
        side_effect=httpx.ConnectError("refused")
    )

    cfg = ProvedexConfig(agent_url="http://127.0.0.1:8765", on_sign_failure="warn")
    proc = ProvedexFrameProcessor(config=cfg)
    await proc.start()
    await proc.handle_frame(StartFrame())
    await proc.stop()

    assert proc.dropped_total >= 1


async def test_processor_dedup_same_frame_not_double_signed():
    cfg = ProvedexConfig(agent_url="http://127.0.0.1:8765")
    proc = ProvedexFrameProcessor(config=cfg)
    await proc.start()

    frame = StartFrame()
    await proc.handle_frame(frame)
    await proc.handle_frame(frame)  # same instance

    # Exactly one enqueue: dedup blocks the second handle_frame call.
    # The single enqueued event is either signed (if worker beat stop)
    # or dropped (if connection refused before stop). Total = 1.
    await proc._drain_with_timeout()
    assert proc.signed_total + proc.dropped_total == 1

    await proc.stop()
