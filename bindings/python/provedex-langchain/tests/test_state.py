import time
from uuid import uuid4

from provedex_langchain._state import CorrelationState


def test_initial_state_is_empty():
    s = CorrelationState()
    assert s.llm_buffer == {}
    assert s.tool_buffer == {}


def test_buffer_llm_and_take_clears():
    s = CorrelationState()
    run_id = uuid4()
    s.buffer_llm_start(run_id, model_id="gpt-4o", prompt_payload=["hi"])
    snapshot = s.take_llm(run_id)
    assert snapshot["model_id"] == "gpt-4o"
    assert snapshot["prompt_payload"] == ["hi"]
    assert "start_time" in snapshot
    assert s.take_llm(run_id) is None


def test_buffer_tool_and_take_clears():
    s = CorrelationState()
    run_id = uuid4()
    s.buffer_tool_start(run_id, tool_name="search", args={"q": "x"})
    snapshot = s.take_tool(run_id)
    assert snapshot["tool_name"] == "search"
    assert snapshot["args"] == {"q": "x"}
    assert "start_time" in snapshot
    assert s.take_tool(run_id) is None


def test_take_unknown_run_id_returns_none():
    s = CorrelationState()
    assert s.take_llm(uuid4()) is None
    assert s.take_tool(uuid4()) is None


def test_start_time_is_monotonic():
    s = CorrelationState()
    run_id = uuid4()
    before = time.monotonic()
    s.buffer_llm_start(run_id, model_id="m", prompt_payload=[])
    after = time.monotonic()
    snap = s.take_llm(run_id)
    assert before <= snap["start_time"] <= after
