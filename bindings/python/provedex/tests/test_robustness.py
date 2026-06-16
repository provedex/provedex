import provedex
import pytest


def test_canonical_json_rejects_nan():
    with pytest.raises(provedex.SigningError):
        provedex.canonical_json({"x": float("nan")})


def test_canonical_json_rejects_infinity():
    with pytest.raises(provedex.SigningError):
        provedex.canonical_json({"x": float("inf")})


def test_tool_called_rejects_non_finite_in_args():
    with pytest.raises(provedex.SigningError):
        provedex.events.tool_called(
            tool_name="t", args_sha256="0" * 64, args_redacted={"v": float("nan")}
        )


def test_finite_float_is_accepted():
    # A normal float must still encode (and as a float, not coerced).
    assert provedex.canonical_json({"x": 1.5}) == b'{"x":1.5}'


def test_verify_chain_empty_list_is_valid():
    report = provedex.verify_chain([])
    assert report.ok is True
    assert report.event_count == 0


def test_verify_file_raises_chain_error_on_malformed_line(tmp_path):
    ledger = tmp_path / "bad.ndjson"
    ledger.write_text("this is not json\n")
    with pytest.raises(provedex.ChainError):
        provedex.verify_file(str(ledger))


def test_sign_event_rejects_wrong_event_type():
    kp = provedex.SigningKeypair.generate()
    with pytest.raises(TypeError):
        provedex.sign_event(
            event={"not": "an event"}, seq=0,
            parent_hash=provedex.GENESIS_PARENT_HASH, keypair=kp,
        )
