# Provedex Python binding

Status: not built yet.

Plan:

- PyO3 wrapper around `provedex-core`.
- Build with `maturin`. Publishes `provedex` to PyPI.
- API surface: `provedex.SigningKeypair.generate()`, `provedex.sign(event_dict, parent_hash, keypair) -> SignedEvent`, `provedex.verify_chain(events) -> ChainReport`.
- Type hints in a sibling `.pyi` stub file.
- Wheels for cpython 3.11+ on macOS arm64, macOS x86_64, linux x86_64, linux arm64.
