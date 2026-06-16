import provedex


def test_record_chains_seq_and_parent(tmp_path):
    kp = provedex.SigningKeypair.generate()
    ledger = str(tmp_path / "ledger.ndjson")
    s = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    assert s.session_id == "s1"
    assert s.pubkey_hex == kp.pubkey_hex

    a = s.record(provedex.events.session_started(agent_id="a", model_id="m", session_id="s1"))
    b = s.record(provedex.events.session_ended(reason="done", summary_sha256="x"))

    assert a.seq == 0
    assert a.parent_hash == provedex.GENESIS_PARENT_HASH
    assert b.seq == 1
    assert b.parent_hash == a.self_hash


def test_reopen_resumes_seq(tmp_path):
    kp = provedex.SigningKeypair.generate()
    ledger = str(tmp_path / "ledger.ndjson")

    s1 = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    s1.record(provedex.events.session_started(agent_id="a", model_id="m", session_id="s1"))

    s2 = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    c = s2.record(provedex.events.session_ended(reason="done", summary_sha256="x"))
    assert c.seq == 1
