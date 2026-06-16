import provedex


def _build_ledger(tmp_path):
    kp = provedex.SigningKeypair.generate()
    ledger = str(tmp_path / "ledger.ndjson")
    s = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    events = [
        s.record(provedex.events.session_started(agent_id="a", model_id="m", session_id="s1")),
        s.record(provedex.events.session_ended(reason="done", summary_sha256="x")),
    ]
    return ledger, events


def test_verify_chain_ok_for_good_chain(tmp_path):
    _, events = _build_ledger(tmp_path)
    report = provedex.verify_chain(events)
    assert report.ok is True
    assert report.event_count == 2
    assert report.broken_at is None
    assert report.reason is None


def test_verify_file_ok(tmp_path):
    ledger, _ = _build_ledger(tmp_path)
    report = provedex.verify_file(ledger)
    assert report.ok is True
    assert report.event_count == 2


def test_verify_file_empty_for_missing(tmp_path):
    report = provedex.verify_file(str(tmp_path / "nope.ndjson"))
    assert report.ok is True
    assert report.event_count == 0
