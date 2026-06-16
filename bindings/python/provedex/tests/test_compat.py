import json
from pathlib import Path

import provedex
import pytest

# tests/ -> provedex/ -> python/ -> bindings/ -> repo root
_VECTORS = Path(__file__).resolve().parents[4] / "tests" / "compat" / "vectors"


def _load(name):
    path = _VECTORS / name
    if not path.exists():
        pytest.skip(f"golden vectors not generated: {path}")
    return json.loads(path.read_text())


def test_canonical_json_matches_rust_goldens():
    for case in _load("canonical_json.json"):
        got = provedex.canonical_json(case["input"])
        assert got == case["expected"].encode("utf-8"), case["name"]


def test_self_hash_matches_rust_goldens():
    for case in _load("self_hash.json"):
        event = provedex.events.from_dict(case["event"])
        got = provedex.compute_self_hash(
            seq=case["seq"],
            timestamp_nanos=case["timestamp_nanos"],
            event=event,
            parent_hash=case["parent_hash"],
        )
        assert got == case["self_hash"], case["name"]
