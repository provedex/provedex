# Provedex

[![ci](https://github.com/provedex/provedex/actions/workflows/ci.yml/badge.svg)](https://github.com/provedex/provedex/actions/workflows/ci.yml)

A black box flight recorder for AI agents. Every utterance, tool call, and model output is signed at the moment it happens and chained to the one before it, so a regulator or auditor can later check that nothing was edited.

The first target is voice agents in healthcare and finance: scribes, intake bots, claims agents. Same primitive works for any AI agent whose decisions land in front of a regulator or a court.

## Why now

The EU AI Act Article 12 obligation begins on August 2, 2026. High-risk AI deployments in the EU have to keep tamper-evident logs of agent reasoning. Fines go up to 15M EUR or 3% of global revenue. There is no funded pure-play in cryptographic agent audit ledgers today.

## What is in the box

- A Rust crate (`provedex-core`) with the signing primitives: Ed25519 signatures, a SHA-256 hash chain, canonical JSON, and an append-only NDJSON ledger.
- A CLI (`provedex`) that verifies, replays, and exports a ledger, plus a demo-only tamper-test.
- A small Axum server (`provedex-server`) that runs a local voice scribe pipeline (whisper.cpp for STT, Ollama for the LLM, Piper for TTS) and emits signed events for every step.
- A single-page UI for the demo so you can speak into a microphone and watch the signed event stream fill in live.

Everything runs on one machine. There is no hosted component.

## Layout

```
crates/
  provedex-core/    signing primitives, hash chain, NDJSON ledger, export bundle
  provedex-cli/     `provedex` command-line tool
  provedex-server/  axum demo server + voice pipeline
frontend/           single-page demo UI (vanilla HTML, JS, CSS)
.github/workflows/  CI: cargo fmt, clippy, test
```

## Quickstart

You need Rust 1.89 (pinned in `rust-toolchain.toml`), ffmpeg, Ollama, a whisper model, and optionally Piper for spoken replies.

1. Install the toolchain and runtime deps.

   ```
   rustup toolchain install 1.89.0
   brew install ffmpeg ollama
   ollama serve &
   ollama pull llama3.2:3b
   ```

2. Drop a whisper model into `~/.provedex/models/`. The base English model is enough.

   ```
   mkdir -p ~/.provedex/models
   curl -L -o ~/.provedex/models/ggml-base.en.bin \
     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
   ```

3. (Optional) Install Piper and a voice if you want the agent to speak back.

   ```
   pipx install piper-tts
   mkdir -p ~/.provedex/voices
   curl -L -o ~/.provedex/voices/en_US-amy-medium.onnx \
     https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/amy/medium/en_US-amy-medium.onnx
   curl -L -o ~/.provedex/voices/en_US-amy-medium.onnx.json \
     https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/amy/medium/en_US-amy-medium.onnx.json
   ```

4. Run the server.

   ```
   cargo run -p provedex-server --features demo
   ```

5. Open `http://localhost:3000`. Hold the mic button, say something, let go. Signed events show up in the right panel. The buttons at the bottom run the three demos:

   - `verify chain` walks the ledger, recomputes every hash, and checks every signature.
   - `tamper test` mutates one event in the local ledger so the chain breaks.
   - `export bundle` downloads a JSON file with the full signed ledger.

The ledger lives at `~/.provedex/ledger.ndjson`. The signing key lives at `~/.provedex/keys/ed25519.key`. Delete the ledger to start fresh.

## CLI

```
cargo run -p provedex-cli -- verify
cargo run -p provedex-cli -- replay
cargo run -p provedex-cli -- export --output ./bundle.json
cargo run -p provedex-cli --features demo -- tamper-test
```

`verify` exits non-zero if the chain is broken.

## Tests

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

CI runs the same three checks on every push and pull request.

## Status

Pre-incorporation. Solo founder building toward a YC application demo. The signing primitives, ledger, CLI, and voice demo work end to end. The hosted aggregator, transparency-log anchoring, and SIEM forwarders are not built yet and are out of scope until after funding.

## License

Apache-2.0. See [LICENSE](./LICENSE).
