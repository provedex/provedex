import provedex


def test_sorts_object_keys_and_strips_whitespace():
    assert provedex.canonical_json({"b": 1, "a": 2}) == b'{"a":2,"b":1}'


def test_nested_arrays_preserved_in_order():
    assert provedex.canonical_json({"c": [3, 2, 1], "a": 2}) == b'{"a":2,"c":[3,2,1]}'


def test_non_ascii_passes_through_as_raw_utf8():
    # The Rust encoder does NOT \u-escape non-ASCII; it emits raw UTF-8 bytes.
    assert provedex.canonical_json({"k": "café"}) == '{"k":"café"}'.encode()


def test_control_chars_escaped():
    assert provedex.canonical_json({"k": "a\nb"}) == b'{"k":"a\\nb"}'
