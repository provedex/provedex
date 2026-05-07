# crates/ - Rust workspace conventions

Rust crates that compile into the workspace.

## Members today

- `provedex-core` - lib. Signing primitives, hash chain, NDJSON ledger, export bundle. Public crate, will publish to crates.io.
- `provedex-cli` - bin. The `provedex` command-line tool.
- `provedex-server` - bin. Demo voice agent server.

## Members planned (not yet scaffolded)

- `provedex-aggregator` - lib + bin. Hosted aggregator service (post-funding).
- `provedex-rekor` - lib. Transparency-log anchoring via Sigstore Rekor.
- `provedex-siem` - lib. Splunk / Datadog / Elastic forwarders.

When you scaffold a new crate, add it to `Cargo.toml` workspace members and use workspace-managed deps (`{ workspace = true }`).

## Naming

- Lib + bin crates: `provedex-<role>` (kebab-case).
- First noun for libraries (`provedex-core`, `provedex-rekor`).
- Verb-y for binaries (`provedex-cli`, `provedex-server`).

## Per-crate rules

- All public items have a `///` doc comment. New public methods on a public struct must have at least one runnable doctest (counts toward CI green-tests).
- Errors use `thiserror` for library crates and `anyhow` for binaries.
- No `unwrap` / `expect` in non-test code unless invariant is provable from the type system, and even then add a `// SAFETY:` style comment if not obvious.
- Tests live next to the code as `#[cfg(test)] mod tests`. Cross-crate tests live in `tests/` at the repo root, not here.
- Format: `cargo fmt`. Lint: `cargo clippy -D warnings`. No exceptions.

## Workspace dependency policy

- Add new dependencies to the root `[workspace.dependencies]` and reference them with `{ workspace = true }` in member crates.
- Pin major version (`tokio = "1.40"` not `tokio = "*"`).
- Prefer crates with active maintenance and a published security advisory record.
- Audit-relevant crypto deps (`ed25519-dalek`, `sha2`, `ring`, etc.) get a one-line ADR explaining why we picked them, in `docs/adr/`.

## Dev-deps

- `tempfile`, `proptest`, `criterion` are appropriate.
- Test fixtures and helpers belong in a `pub(crate)` module gated `#[cfg(test)]`, never in regular source.

## Forbidden

- No `unsafe` blocks in `provedex-core` without a written justification in the source comment AND an ADR.
- No I/O in `provedex-core::signed`, `::chain`, or `::keys`. Those modules are pure crypto. I/O lives in `::ledger` and `::keys::*_path` helpers only.
- No FFI from a Rust crate. Bindings live in `bindings/`, not in a `crates/` member.
