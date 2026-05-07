# tests/ - cross-crate and cross-language tests

Per-crate unit tests stay inside the crate. This folder is for tests that span multiple crates or multiple language runtimes.

## Sections

| Folder | Purpose |
|--------|---------|
| `compat/` | Byte-compat tests across language bindings. Each binding signs a fixed input; outputs must be byte-identical. |
| `e2e/` | End-to-end voice pipeline tests (audio in, signed events out, verify green). |

## When to add a test here vs in a crate

- Touches one crate only -> stay in that crate's `#[cfg(test)] mod tests`.
- Touches `provedex-core` AND a binding -> goes in `tests/compat/`.
- Spawns the demo server and drives the browser flow -> goes in `tests/e2e/`.
- Touches `provedex-cli` against a synthesized ledger -> stay in `provedex-cli` (it already depends on core).

## compat/ test shape

- Golden input: a hardcoded `AgentEvent` payload as JSON.
- Golden output: the canonical-JSON bytes and the SHA-256 of those bytes.
- Each binding has a small test harness that produces its own canonical-JSON for the same input and asserts byte equality.

## e2e/ test shape

- Boot `provedex-server` on a random port via a test harness.
- Drive a browser with a headless framework (Playwright when added, not yet decided).
- Speak a synthesized WAV into the mic API.
- Assert SSE backlog contains the expected event types.
- Click verify, assert valid.
- Click tamper, click verify, assert broken.

## CI cost

E2e tests are heavy. Run them in a separate CI job, not on every push. The `cargo fmt + clippy + test` gate stays cheap and fast.

## Conventions

- Plain ASCII. No em dashes.
- Test names describe the property being asserted, not the implementation step.
- One assertion focus per test. Use multiple tests rather than a single test that asserts six unrelated things.
