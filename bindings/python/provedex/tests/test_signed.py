import json

import provedex


def test_sign_event_at_genesis_has_seq_zero_and_signer():
    kp = provedex.SigningKeypair.generate()
    e = provedex.events.session_started(agent_id="a", model_id="m", session_id="s")
    signed = provedex.sign_event(
        event=e, seq=0, parent_hash=provedex.GENESIS_PARENT_HASH, keypair=kp
    )
    assert signed.seq == 0
    assert signed.parent_hash == provedex.GENESIS_PARENT_HASH
    assert signed.signer_pubkey == kp.pubkey_hex
    assert len(signed.self_hash) == 64
    assert len(signed.signature) == 128


def test_signed_event_to_json_parses_and_has_fields():
    kp = provedex.SigningKeypair.generate()
    e = provedex.events.session_ended(reason="done", summary_sha256="x")
    signed = provedex.sign_event(
        event=e, seq=0, parent_hash=provedex.GENESIS_PARENT_HASH, keypair=kp
    )
    parsed = json.loads(signed.to_json())
    assert parsed["seq"] == 0
    assert parsed["event"]["type"] == "SessionEnded"
    assert signed.event["type"] == "SessionEnded"
    assert signed.event["payload"]["reason"] == "done"


def test_compute_self_hash_is_deterministic_for_fixed_inputs():
    e = provedex.events.session_started(agent_id="a", model_id="m", session_id="s")
    h1 = provedex.compute_self_hash(
        seq=0, timestamp_nanos=1234, event=e, parent_hash=provedex.GENESIS_PARENT_HASH
    )
    h2 = provedex.compute_self_hash(
        seq=0, timestamp_nanos=1234, event=e, parent_hash=provedex.GENESIS_PARENT_HASH
    )
    assert h1 == h2
    assert len(h1) == 64
