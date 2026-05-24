from provedex_pipecat._state import CorrelationState


def test_initial_state_is_empty():
    s = CorrelationState()
    assert s.last_messages is None
    assert s.pending_response_text == ""
    assert s.response_in_progress is False
    assert s.seen_frame_ids == set()


def test_buffer_and_take_clears_state():
    s = CorrelationState()
    s.buffer_messages([{"role": "user", "content": "x"}])
    s.mark_response_start()
    s.buffer_response_text("hel")
    s.buffer_response_text("lo")
    messages, text = s.take_paired_invocation()
    assert messages == [{"role": "user", "content": "x"}]
    assert text == "hello"
    assert s.last_messages is None
    assert s.pending_response_text == ""
    assert s.response_in_progress is False


def test_take_without_buffer_returns_none_messages():
    s = CorrelationState()
    messages, text = s.take_paired_invocation()
    assert messages is None
    assert text == ""


def test_dedup_first_seen_is_false_then_true():
    s = CorrelationState()
    assert s.already_seen(42) is False
    assert s.already_seen(42) is True
    assert s.already_seen(43) is False
