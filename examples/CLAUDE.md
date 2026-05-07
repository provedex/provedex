# examples/ - runnable integration examples

Each subfolder is one self-contained, runnable example. New engineers and customer engineers should be able to copy a folder and adapt it.

## Naming

- Verb-y or use-case names: `voice-scribe`, `python-quickstart`, `langchain-callback`.
- No `provedex-` prefix on examples.

## Per-example layout

```
examples/<name>/
  README.md          prerequisites, run command, expected output, verify steps
  Cargo.toml         (if Rust) or package.json (if Node) or pyproject.toml (if Python)
  src/main.rs        (or main.py / index.ts)
  expected/          golden outputs an integrator can diff against
```

## README required sections

1. What this example shows.
2. Prerequisites (toolchain, models, env vars).
3. How to run (one command).
4. What to expect (sample output).
5. How to verify (the `provedex verify` invocation that proves the run was real).

## Conventions

- Examples must build and run on a fresh clone. CI may not exercise them, but a customer engineer should not hit a broken example.
- Examples may depend on `provedex-core` and bindings. They must NOT depend on private workspace internals.
- Plain ASCII. No em dashes. Same code-style rules as the rest of the repo.

## Rust examples

- A simple Rust example can also live as `crates/<crate>/examples/<name>.rs` for `cargo run -p <crate> --example <name>`. The mirror in `examples/<name>/` is for non-Rust audiences who would not look inside `crates/`.

## Forbidden

- No partial examples (skeleton without working code).
- No examples that require keys, models, or services the customer cannot obtain in 5 minutes.
- No examples that mutate `~/.provedex/` without saying so up front in the README.
