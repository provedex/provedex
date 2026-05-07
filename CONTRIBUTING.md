# Contributing

## Development setup

1. Install Rust 1.89.0 (pinned in `rust-toolchain.toml`).

   ```
   rustup toolchain install 1.89.0
   ```

2. Install runtime dependencies for the voice demo.

   ```
   brew install ffmpeg ollama
   ollama serve &
   ollama pull llama3.2:3b
   ```

3. Drop the whisper model into `~/.provedex/models/`.

   ```
   mkdir -p ~/.provedex/models
   curl -L -o ~/.provedex/models/ggml-base.en.bin \
     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
   ```

4. (Optional, for spoken replies) install Piper and a voice.

   ```
   pipx install piper-tts
   mkdir -p ~/.provedex/voices
   curl -L -o ~/.provedex/voices/en_US-amy-medium.onnx \
     https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/amy/medium/en_US-amy-medium.onnx
   curl -L -o ~/.provedex/voices/en_US-amy-medium.onnx.json \
     https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/amy/medium/en_US-amy-medium.onnx.json
   ```

5. Run the test suite.

   ```
   cargo test --workspace --all-features
   ```

6. Install dev tooling for commit-msg enforcement and mutation testing.

   ```
   cargo install --locked cocogitto cargo-mutants
   cog install-hook commit-msg
   ```

   `cog` rejects non-conventional commit subjects locally before they leave your machine. `cargo mutants` is documented in the "Mutation testing" section below.

To run the demo server locally:

```
cargo run -p provedex-server --features demo
```

Open `http://localhost:3000`.

## Branches

Work on a topic branch off `main`. Push the branch and open a pull request against `main`.

## Commit messages

Conventional commits, imperative mood, subject line under 72 characters. Body explains the why if it is not obvious.

```
feat(core): add export bundle schema version
fix(server): fall back to ~/.local/bin when locating piper
chore(ci): cache target dir keyed on Cargo.lock
docs: rewrite quickstart for plain ascii
test(core): add tamper-detection coverage on signature mutation
refactor(server): split chat handler into voice pipeline stages
```

Allowed types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`, `perf`, `build`. Do not include co-author trailers from AI tools.

## Pull request process

- One feature or fix per pull request. If you find an unrelated cleanup, open a separate pull request.
- The pull request must pass CI before review: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features`.
- If you change a public API in `provedex-core`, add or update the rustdoc comment and include at least one runnable doctest.
- Keep the diff small. Drop dead code in the same change set that introduces it.
- Do not commit generated artifacts, IDE files, or anything from `~/.provedex/`.

## Style

- No em dashes anywhere in code, comments, commit messages, or docs. Use a hyphen, a colon, parentheses, or rephrase.
- No emojis.
- Comment the why when it is not obvious. Do not narrate what the code does.
- Names over comments. Small functions over big ones.
- No new files outside the structure documented in the README and `TECHNICAL_PLAN.md`.

## Mutation testing

Crypto + ledger code in `provedex-core` is covered by mutation testing via `cargo-mutants`. The point: a passing unit test means the code does what we wrote; a passing mutation test means our tests would catch the code being subtly wrong.

Run before any release that touches `crates/provedex-core/src/{chain,ledger,signed,session,keys}.rs`:

```
cargo mutants -p provedex-core --in-place --file crates/provedex-core/src/chain.rs --file crates/provedex-core/src/ledger.rs --file crates/provedex-core/src/session.rs
```

Any surviving mutant means a test gap. Fix the test, not the mutant. CI does not run mutants on every push (slow); local discipline before tagging a release is enough.

## Benchmarking

Latency numbers for `provedex-core` hot paths live in `crates/provedex-core/benches/sign_bench.rs`. Run before tagging a release:

```
cargo bench -p provedex-core
```

To diff against a saved baseline:

```
cargo bench -p provedex-core -- --save-baseline v0.1.0
# ... make changes ...
cargo bench -p provedex-core -- --baseline v0.1.0
```

CI does not run benches; criterion needs warmup time and stable hardware. Run them locally on a quiet machine. Update the README "Performance" table any time a benchmark moves more than 10 percent.

## Reporting bugs and security issues

Open an issue using the bug-report template for non-security bugs. For security reports, follow `SECURITY.md`.
