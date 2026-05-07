# Cross-crate tests

End-to-end and cross-language tests that do not belong to a single crate.

## Layout

- `compat/` - FUTURE. Byte-compat tests across language bindings. Each binding signs a fixed input; outputs must be byte-identical.
- `e2e/` - FUTURE. Full voice pipeline tests (audio in, signed events out, verify green).

Per-crate unit tests stay inside the relevant crate (`crates/<name>/src/.../tests`). This directory is for tests that span crates or runtimes.
