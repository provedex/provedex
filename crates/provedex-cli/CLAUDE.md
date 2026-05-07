# crates/provedex-cli - the `provedex` command-line tool

Single binary. Wraps `provedex-core` for operators and auditors.

## Layout

```
src/
  main.rs              clap subcommand parser, exit codes
  commands/
    verify.rs          verify chain, exit non-zero if broken
    replay.rs          human-readable transcript
    export.rs          write signed bundle JSON
    tamper_test.rs     demo-only, gated #[cfg(feature = "demo")]
    mod.rs             module declarations
```

## Adding a new subcommand

1. Add a variant to `Command` enum in `main.rs`.
2. Create `src/commands/<name>.rs` with a `pub fn run(...)` that returns `anyhow::Result<()>`.
3. Wire it in `mod.rs` and the `match` in `main.rs`.
4. If demo-only, gate the variant AND the module with `#[cfg(feature = "demo")]` AND list it after the non-demo variants.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Chain broken (verify) or expected business-logic failure |
| 2 | Unexpected error (I/O, bad path, parse failure) |

`run()` returns `Err(_)` for code 2. For code 1, `std::process::exit(1)` directly inside the command.

## Default paths

`provedex_core::default_ledger_path()` and `default_key_path()`. Honor `--ledger` and `--key` overrides on every command.

## Output format

- Human-readable for terminals. Plain ASCII. No tables with unicode characters.
- For machine consumption, add a future `--json` flag rather than parsing the human output.

## Forbidden

- No HTTP calls. CLI is offline.
- No long-running daemons. Each invocation is one shot.
- No demo-only logic outside `#[cfg(feature = "demo")]`.
