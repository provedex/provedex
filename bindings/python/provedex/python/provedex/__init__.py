"""Native Python SDK for Provedex.

Signs Ed25519, hash-chained agent evidence in-process, byte-identical to the
Rust reference. See https://github.com/provedex/provedex.
"""

from ._provedex import (
    ChainError,
    KeyLoadError,
    LedgerError,
    ProvedexError,
    SigningError,
    SigningKeypair,
    __version__,
)

__all__ = [
    "ChainError",
    "KeyLoadError",
    "LedgerError",
    "ProvedexError",
    "SigningError",
    "SigningKeypair",
    "__version__",
]
