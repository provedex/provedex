# Provedex - Claude project conventions

Lean root. For deeper context on a folder, read its `CLAUDE.md` instead of asking.

## Repo at a glance

```
crates/                 Rust workspace (lib + bins)
bindings/               FFI wrappers around provedex-core (python, node)
apps/                   deployable end-user apps (demo-web)
docs/                   spec, adr, integration, compliance
examples/               runnable integration examples
tests/                  cross-crate / cross-language tests
.github/                CI + issue + PR templates
```

## Navigation - read the relevant sub-CLAUDE.md before working in that area

| Working in | Read |
|------------|------|
| Cryptographic core, ledger, hash chain | `crates/provedex-core/CLAUDE.md` |
| CLI subcommands | `crates/provedex-cli/CLAUDE.md` |
| Demo server, voice pipeline, routes | `crates/provedex-server/CLAUDE.md` |
| Workspace-level Rust conventions | `crates/CLAUDE.md` |
| Frontend UI rules + design tokens | `apps/demo-web/CLAUDE.md` |
| Adding a new app | `apps/CLAUDE.md` |
| FFI bindings, byte-compat rules | `bindings/CLAUDE.md` |
| Specs, ADRs, integration, compliance docs | `docs/CLAUDE.md` |
| Runnable examples | `examples/CLAUDE.md` |
| Cross-crate tests | `tests/CLAUDE.md` |

## Code rules (always on, every file in repo)

- Plain ASCII. No em dashes (use a hyphen, colon, parentheses, or rephrase). No emojis. No curly quotes. No special unicode (no en dash, arrow, middle dot).
- No AI slop adjectives: "robust", "comprehensive", "powerful", "elegant", "leveraging", "cutting-edge", "next-gen", "seamless".
- Comment the WHY when not obvious. Never narrate WHAT. If removing the comment would not confuse a future reader, do not write it.
- Function and variable names carry meaning. Comments are a backup, not a primary explanation.
- Small focused functions. Single responsibility. No dead code. No half-finished implementations.
- Trust internal code and framework guarantees. Validate only at system boundaries.
- No new files outside this repo's documented layout. Ask before adding a new top-level directory.

## Git workflow

- Conventional commits. Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`, `perf`, `build`. Imperative mood. Subject under 72 chars. Body explains why if not obvious.
- Auto commit + push enabled per founder instruction. Push after each meaningful change. Do not batch large diffs.
- Stage files by name when possible. Avoid `git add -A` if junk could land.
- No co-author trailer. The setting `includeCoAuthoredBy: false` enforces this.
- Never destructive (`reset --hard`, `push --force`, `branch -D`, `clean -f`) without explicit instruction in the same turn.

## CI gate (must be green before claiming done)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

`cargo test` runs unit, integration, and doc tests. All three must pass.

## Where new files go (quick map)

| New thing | Folder |
|-----------|--------|
| Cryptographic primitive, ledger code | `crates/provedex-core/` |
| New CLI subcommand | `crates/provedex-cli/src/commands/` |
| New HTTP route | `crates/provedex-server/src/routes/` |
| New voice pipeline stage | `crates/provedex-server/src/voice/` |
| New deployable Rust service | `crates/provedex-<name>/` |
| Python binding code | `bindings/python/` |
| TypeScript binding code | `bindings/node/` |
| Web UI (demo, dashboard) | `apps/<name>-web/` |
| Spec document | `docs/spec/<topic>-vN.md` |
| ADR | `docs/adr/NNNN-kebab-title.md` |
| Integration guide | `docs/integration/<framework>.md` |
| Compliance mapping | `docs/compliance/<regulation>.md` |
| Runnable example | `examples/<name>/` |
| Cross-crate test | `tests/<category>/` |

If a new file does not fit any row, ask before creating.

## Out of scope (do not build without asking)

- AI model training, fine-tuning, or RLHF.
- PII / PHI redaction. Customer decides what goes in events.
- A general observability platform.
- A general compliance workflow tool.
- A blockchain. We use hash chains and signatures, not consensus.

## Ignored locally (gitignored)

`STARTUP_CONTEXT.md`, `TECHNICAL_PLAN.md`, `EXPLAINER.md`, `.claude/`, `target/`, `.DS_Store`, `.env`, `.vscode/`, `.idea/`, `~/.provedex/`.
