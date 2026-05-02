# Provedex

[![ci](https://github.com/provedex/provedex/actions/workflows/ci.yml/badge.svg)](https://github.com/provedex/provedex/actions/workflows/ci.yml)

**Cryptographic evidence layer for regulated AI agents.**

When an AI agent makes a decision in healthcare, finance, legal, or any regulated environment, Provedex produces a tamper-evident, cryptographically signed record of exactly what happened: every tool call, every model output, every voice utterance. Regulators, auditors, and courts get a verifiable answer.

## Why

The EU AI Act Article 12 mandate goes into enforcement on August 2, 2026. Every high-risk AI deployment in the EU must produce tamper-evident logs of agent reasoning. Penalties run up to 15M EUR or 3% of global revenue. No funded pure-play exists for cryptographic agent audit ledgers today.

## What

- Open-source Rust SDK with Ed25519 signatures + SHA-256 hash chains
- Local NDJSON ledger, append-only
- `verify`, `replay`, `export` CLI commands
- Voice agent reference deployment as the v1 vertical (healthcare scribes, financial voice agents)

## Repository layout

```
crates/
  provedex-core/    signing primitives, hash chain, NDJSON ledger, export bundle
  provedex-cli/     `provedex` command-line tool
  provedex-server/  Axum demo server with whisper-rs + Ollama + Piper
frontend/           single-page demo UI (vanilla HTML, JS, Tailwind via CDN)
```

## Quickstart - voice demo

Prerequisites:

- Rust 1.89 (pinned in `rust-toolchain.toml`)
- `ffmpeg` on PATH (audio decoding)
- `ollama serve` running locally with `llama3.2:3b` pulled
- Whisper model at `~/.provedex/models/ggml-base.en.bin`
- (Optional) Piper binary on PATH plus a voice at `~/.provedex/voices/en_US-amy-medium.onnx` for spoken responses

Run the demo server:

```
cargo run -p provedex-server --features demo
```

Open `http://localhost:3000`. Hold the mic button, speak a clinical note, release. Signed events stream into the right panel as the pipeline runs. Click `Verify chain` for the root hash, `Tamper test` to corrupt one event, `Verify chain` again to see the chain break, and `Export for regulator` to download the signed bundle.

## CLI

```
cargo run -p provedex-cli -- verify
cargo run -p provedex-cli -- replay
cargo run -p provedex-cli -- export --output ./bundle.json
cargo run -p provedex-cli --features demo -- tamper-test
```

The default ledger lives at `~/.provedex/ledger.ndjson`; signing keys at `~/.provedex/keys/ed25519.key`.

## Tests

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

CI runs the same three checks on every push (`.github/workflows/ci.yml`).

## License

Apache-2.0. See [LICENSE](./LICENSE).
