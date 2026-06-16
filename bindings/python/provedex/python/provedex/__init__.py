"""Native Python SDK for Provedex.

Signs Ed25519, hash-chained agent evidence in-process, byte-identical to the
Rust reference. See https://github.com/provedex/provedex.
"""

from ._provedex import (
    GENESIS_PARENT_HASH,
    ChainError,
    KeyLoadError,
    LedgerError,
    ProvedexError,
    SignedEvent,
    SigningError,
    SigningKeypair,
    __version__,
    compute_self_hash,
    events,
    sign_event,
)

__all__ = [
    "GENESIS_PARENT_HASH",
    "ChainError",
    "KeyLoadError",
    "LedgerError",
    "ProvedexError",
    "SignedEvent",
    "SigningError",
    "SigningKeypair",
    "__version__",
    "compute_self_hash",
    "events",
    "sign_event",
]
