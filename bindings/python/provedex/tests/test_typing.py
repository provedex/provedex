from pathlib import Path

import provedex


def test_typed_usage_compiles_and_runs(tmp_path: Path) -> None:
    kp = provedex.SigningKeypair.generate()
    s = provedex.Session.open(
        keypair=kp, ledger_path=str(tmp_path / "l.ndjson"), session_id="s1"
    )
    signed = s.record(
        provedex.events.session_started(agent_id="a", model_id="m", session_id="s1")
    )
    report = provedex.verify_chain([signed])
    assert report.ok is True
