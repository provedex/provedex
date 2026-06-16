def test_import_and_version():
    import provedex

    assert isinstance(provedex.__version__, str)
    assert provedex.__version__ == "0.1.0"
