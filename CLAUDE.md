# Provedex - Claude project conventions

This file is auto-loaded into every Claude Code session in this repo. It tells Claude where things live, what the rules are, and what is in scope.

## Repository layout

```
crates/                 Rust workspace
  provedex-core/        signing primitives, hash chain, NDJSON ledger (lib)
  provedex-cli/         `provedex` command-line tool (bin)
  provedex-server/      demo voice agent server (bin)
bindings/               FFI wrappers around provedex-core
  python/               pyo3 -> PyPI `provedex` (not built yet)
  node/                 napi-rs -> npm `@provedex/core` (not built yet)
apps/                   deployable end-user apps
  demo-web/             single-page UI for the live voice scribe demo
docs/
  spec/                 byte-level normative specs (event schema, canonical JSON, signatures)
  adr/                  architecture decision records, NNNN-kebab-title.md
  integration/          framework-specific integration guides
  compliance/           regulator clause mappings (EU AI Act, HIPAA, FINRA)
examples/               runnable integration examples (one subdir per example)
tests/                  cross-crate / cross-language tests
.github/                CI workflows + issue + PR templates
```

## Where new files go

| New thing | Goes in |
|-----------|---------|
| Cryptographic primitive, ledger code | `crates/provedex-core/` |
| New CLI subcommand | `crates/provedex-cli/src/commands/` |
| New HTTP route on demo server | `crates/provedex-server/src/routes/` |
| New voice pipeline stage | `crates/provedex-server/src/voice/` |
| New deployable service (e.g. aggregator) | `crates/provedex-<name>/` |
| Python binding code | `bindings/python/` |
| TypeScript binding code | `bindings/node/` |
| Web UI (demo, dashboard, portal) | `apps/<app-name>-web/` |
| Frontend assets (HTML, CSS, JS) for demo | `apps/demo-web/` |
| Specification document | `docs/spec/<topic>-vN.md` |
| Architecture decision record | `docs/adr/NNNN-kebab-title.md` |
| Integration guide | `docs/integration/<framework>.md` |
| Compliance mapping | `docs/compliance/<regulation>.md` |
| Runnable integration example | `examples/<name>/` |
| Cross-crate test | `tests/<category>/` |
| Per-crate unit test | inside the crate, in a `#[cfg(test)] mod tests` block |

If a new file does not fit any of the above, ask before creating.

## Code standards (non-negotiable)

- Plain ASCII. No em dashes (use a hyphen, colon, parentheses, or rephrase). No emojis. No curly quotes, no en dashes, no special unicode.
- No AI slop adjectives: "robust", "comprehensive", "powerful", "elegant", "leveraging", "cutting-edge", "next-gen", "seamless".
- Comment the WHY when not obvious. Never narrate WHAT (the code already says that). No comment unless removing it would confuse a future reader.
- Function and variable names carry meaning. Comments are a backup, not a primary explanation.
- Small focused functions. Single responsibility. No dead code.

## Git workflow

- Conventional commits. Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`, `perf`, `build`. Imperative mood. Subject under 72 chars. Body explains why if not obvious.
- No co-author trailer. The repo-level setting `includeCoAuthoredBy: false` enforces this. If a commit ever shows a co-author trailer, that is a bug.
- Auto-commit and auto-push are enabled for this repo per the founder's instruction. Commit after meaningful changes. Push after commit. Do not batch large diffs.
- Stage specific files by name when possible. Avoid `git add -A` if there is a chance of staging junk.
- Never run destructive git commands (`reset --hard`, `push --force`, `branch -D`, `clean -f`) without explicit instruction in the same turn.

## CI requirements

Every push must pass:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

`cargo test` runs unit, integration, and doc tests. All three must be green before claiming work is done.

## Public API doc requirement

Any new public item in `crates/provedex-core/` (struct, enum, function, method) must:

1. Have a `///` doc comment that explains what it is and any non-obvious invariants.
2. Have at least one runnable doctest if it is a method on a public struct.

Doctests count toward the green-tests acceptance criterion.

## Runtime conventions

- Default ledger path: `~/.provedex/ledger.ndjson`.
- Default keypair path: `~/.provedex/keys/ed25519.key`.
- Default whisper model path: `~/.provedex/models/ggml-base.en.bin`.
- Default piper voice path: `~/.provedex/voices/en_US-amy-medium.onnx`.
- Demo server port: 3000 (frontend + API on same port).
- Override paths via env vars: `PROVEDEX_WHISPER_MODEL`, `PROVEDEX_PIPER_BIN`, `PROVEDEX_PIPER_VOICE`, `PROVEDEX_PIPER_LENGTH_SCALE`.

## What is in scope

- Open-source Rust SDK with signing primitives.
- Local NDJSON ledger.
- CLI for verify, replay, export, demo-only tamper-test.
- Voice agent demo server (whisper-rs, Ollama, Piper).
- Single-page demo UI.
- Python and TypeScript bindings (planned, in `bindings/`).
- Hosted aggregator service (planned, future `crates/provedex-aggregator/`).
- SIEM forwarders (planned, future `crates/provedex-siem/`).
- Transparency-log anchoring via Rekor (planned, future `crates/provedex-rekor/`).

## What is out of scope (do not build without asking)

- AI model training, fine-tuning, or RLHF.
- PII/PHI redaction (customer decides what goes in events; we provide the signing layer).
- A general-purpose observability platform (we are infrastructure, not a Datadog clone).
- A general compliance workflow tool (we sit underneath Vanta/Drata; we do not replace them).
- A blockchain. We use hash chains and signatures, not consensus.

## Ignored locally (gitignored)

- `STARTUP_CONTEXT.md` - founder-only strategic doc.
- `TECHNICAL_PLAN.md` - founder-only sprint plan.
- `EXPLAINER.md` - founder-only product explainer.
- `.claude/` - per-machine Claude Code config.
- `target/`, `.DS_Store`, `.env`, `.vscode/`, `.idea/`, `~/.provedex/`.
