import pytest

import provedex


def test_generate_has_64_hex_pubkey():
    kp = provedex.SigningKeypair.generate()
    assert len(kp.pubkey_hex) == 64
    int(kp.pubkey_hex, 16)  # is hex


def test_save_load_roundtrip_same_pubkey(tmp_path):
    path = str(tmp_path / "k.key")
    kp = provedex.SigningKeypair.generate()
    kp.save(path)
    loaded = provedex.SigningKeypair.load(path)
    assert loaded.pubkey_hex == kp.pubkey_hex


def test_load_or_create_is_stable(tmp_path):
    path = str(tmp_path / "nested" / "k.key")
    first = provedex.SigningKeypair.load_or_create(path)
    second = provedex.SigningKeypair.load_or_create(path)
    assert first.pubkey_hex == second.pubkey_hex


def test_load_missing_raises_key_load_error(tmp_path):
    with pytest.raises(provedex.KeyLoadError):
        provedex.SigningKeypair.load(str(tmp_path / "does-not-exist.key"))
