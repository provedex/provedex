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

## Reporting bugs and security issues

Open an issue using the bug-report template for non-security bugs. For security reports, follow `SECURITY.md`.
