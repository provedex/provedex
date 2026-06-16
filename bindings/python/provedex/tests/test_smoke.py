import re


def test_import_and_version():
    import provedex

    # Assert the version is wired and well-formed, not a hardcoded value, so a
    # release bump does not require editing this test.
    assert isinstance(provedex.__version__, str)
    assert re.fullmatch(r"\d+\.\d+\.\d+", provedex.__version__)
