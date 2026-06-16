"""Native Python SDK for Provedex.

Signs Ed25519, hash-chained agent evidence in-process, byte-identical to the
Rust reference. See https://github.com/provedex/provedex.
"""

from ._provedex import __version__

__all__ = ["__version__"]
