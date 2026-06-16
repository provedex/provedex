import subprocess
from pathlib import Path

import provedex
import pytest


@pytest.mark.integration
def test_python_signed_ledger_verifies_with_rust_cli(tmp_path, provedex_cli):
    ledger = str(tmp_path / "ledger.ndjson")
    kp = provedex.SigningKeypair.generate()
    s = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    s.record(provedex.events.session_started(agent_id="a", model_id="m", session_id="s1"))
    s.record(
        provedex.events.model_invoked(
            model_id="m", prompt_sha256="a" * 64, response_sha256="b" * 64,
            prompt_tokens=5, response_tokens=2,
        )
    )
    s.record(provedex.events.session_ended(reason="done", summary_sha256="x"))

    result = subprocess.run(
        [provedex_cli, "verify", "--ledger", ledger],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "status: VALID" in result.stdout
    assert "events: 3" in result.stdout


def test_rust_signed_fixture_verifies_in_python():
    # Reverse direction: a ledger SIGNED BY RUST (the committed fixture) must
    # verify VALID through the Python binding. Needs no CLI, only the fixture.
    fixture = (
        Path(__file__).resolve().parents[4]
        / "tests" / "compat" / "vectors" / "rust_signed_ledger.ndjson"
    )
    if not fixture.exists():
        pytest.skip(f"rust-signed fixture not generated: {fixture}")
    report = provedex.verify_file(str(fixture))
    assert report.ok is True
    assert report.event_count == 3


@pytest.mark.integration
def test_cli_and_python_agree_a_tampered_ledger_is_broken(tmp_path, provedex_cli):
    # Both implementations must independently judge the same tampered chain
    # broken, proving they recompute self_hash over identical bytes.
    ledger = str(tmp_path / "ledger.ndjson")
    kp = provedex.SigningKeypair.generate()
    s = provedex.Session.open(keypair=kp, ledger_path=ledger, session_id="s1")
    s.record(provedex.events.session_started(agent_id="a", model_id="m", session_id="s1"))
    s.record(provedex.events.session_ended(reason="done", summary_sha256="x"))

    # Tamper: flip a value inside the second line's payload.
    with open(ledger) as f:
        lines = f.readlines()
    lines[1] = lines[1].replace('"done"', '"tampered"')
    with open(ledger, "w") as f:
        f.writelines(lines)

    py_report = provedex.verify_file(ledger)
    assert py_report.ok is False

    result = subprocess.run(
        [provedex_cli, "verify", "--ledger", ledger],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 1
    assert "status: BROKEN" in result.stdout
