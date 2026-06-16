"""Minimal end-to-end: open a session, sign three events, verify the ledger."""

import tempfile
from pathlib import Path

import provedex


def main() -> None:
    workdir = Path(tempfile.mkdtemp())
    keypair = provedex.SigningKeypair.load_or_create(str(workdir / "ed25519.key"))
    session = provedex.Session.open(
        keypair=keypair,
        ledger_path=str(workdir / "ledger.ndjson"),
        session_id="demo-session",
    )

    session.record(
        provedex.events.session_started(
            agent_id="demo-agent", model_id="gpt-4o", session_id="demo-session"
        )
    )
    session.record(
        provedex.events.model_invoked(
            model_id="gpt-4o",
            prompt_sha256="a" * 64,
            response_sha256="b" * 64,
            prompt_tokens=12,
            response_tokens=34,
        )
    )
    session.record(
        provedex.events.session_ended(reason="completed", summary_sha256="c" * 64)
    )

    report = session_verify(workdir)
    print(f"signer: {keypair.pubkey_hex}")
    print(f"ledger: {workdir / 'ledger.ndjson'}")
    print(f"verified: ok={report.ok} events={report.event_count}")


def session_verify(workdir: Path) -> provedex.ChainReport:
    return provedex.verify_file(str(workdir / "ledger.ndjson"))


if __name__ == "__main__":
    main()
