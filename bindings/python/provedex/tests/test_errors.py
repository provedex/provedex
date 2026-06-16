import provedex


def test_exception_hierarchy():
    assert issubclass(provedex.KeyLoadError, provedex.ProvedexError)
    assert issubclass(provedex.SigningError, provedex.ProvedexError)
    assert issubclass(provedex.LedgerError, provedex.ProvedexError)
    assert issubclass(provedex.ChainError, provedex.ProvedexError)
    assert issubclass(provedex.ProvedexError, Exception)
