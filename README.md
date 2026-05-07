# Provedex

[![ci](https://github.com/provedex/provedex/actions/workflows/ci.yml/badge.svg)](https://github.com/provedex/provedex/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Cryptographic evidence layer for AI agents. Every utterance, tool call, and model output is signed at emission time and chained to the one before it. Anyone with the public key can verify the log, offline, with no involvement from the operator who produced it.

Built for regulated AI: healthcare scribes, financial voice agents, legal intake bots, claims handlers. Same primitive works for any agent whose decisions land in front of a regulator or a court.

## Why this exists

The EU AI Act Article 12 obligation begins on August 2, 2026. High-risk AI deployments in the EU must produce tamper-evident logs of agent reasoning. Penalties go up to 15M EUR or 3% of global revenue.

Existing logging stacks (Datadog, Splunk, OpenTelemetry) record what the operator's app says happened. They do not produce evidence that survives an adversarial review. Compliance SaaS vendors collect screenshots; the post-Delve market has learned what that is worth.

Provedex is the primitive underneath. Sign locally, chain locally, verify offline. The operator never has to trust a vendor for the integrity of the log.

## Components

| Crate | Role | Status |
|-------|------|--------|
| `provedex-core` | signing primitives, hash chain, NDJSON ledger, export bundle | shipped |
| `provedex-cli` | `provedex` command-line tool: verify, replay, export | shipped |
| `provedex-agent` | localhost HTTP signing daemon for non-Rust customers (default integration) | shipped |
| `provedex-server` | reference voice-agent demo (whisper.cpp + Ollama + Piper) | shipped, demo-only |

Native bindings (Python, Node) are planned as optional fast-paths; the sidecar covers every other language via localhost HTTP. See ADR 0004.

## Quickstart - sidecar

The sidecar is the default integration path for any non-Rust app.

```bash
git clone https://github.com/provedex/provedex
cd provedex
cargo build --release -p provedex-agent
./target/release/provedex-agent
```

The agent binds `127.0.0.1:8765` and auto-creates a keypair at `~/.provedex/keys/ed25519.key`. Sign an event from any language:

```bash
curl -X POST http://127.0.0.1:8765/v1/sign \
  -H 'content-type: application/json' \
  -d '{"event":{"type":"SessionStarted","payload":{"agent_id":"demo","model_id":"gpt-4o","session_id":"s1"}}}'
```

Verify the chain:

```bash
curl -X POST http://127.0.0.1:8765/v1/verify
```

Per-language clients (Python, Node, Java, Go, Ruby, PHP) live in [docs/integration/sidecar.md](docs/integration/sidecar.md).

## Quickstart - Rust crate

For Rust apps, link the crate directly:

```toml
[dependencies]
provedex-core = "0.1"
```

```rust
use provedex_core::{AgentEvent, Ledger, LedgerSession, SigningKeypair};

let keypair = SigningKeypair::load_or_create("./key")?;
let ledger = Ledger::open("./ledger.ndjson")?;
let session = LedgerSession::open(keypair, ledger, "session-1".into())?;

let signed = session.seal_and_append(AgentEvent::SessionStarted {
    agent_id: "agent-1".into(),
    model_id: "gpt-4o".into(),
    session_id: "session-1".into(),
})?;
```

Run the minimal end-to-end example:

```bash
cargo run -p provedex-core --example basic_signing
```

## Voice agent reference (optional)

The `provedex-server` crate runs a local voice scribe pipeline against the sidecar primitives. Useful as a working integration example. Requires ffmpeg, Ollama, and a whisper model.

```bash
# Install runtime deps
brew install ffmpeg ollama
ollama serve &
ollama pull llama3.2:3b

# Whisper model
mkdir -p ~/.provedex/models
curl -L -o ~/.provedex/models/ggml-base.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin

# (Optional) Piper TTS for spoken replies
pipx install piper-tts
mkdir -p ~/.provedex/voices
curl -L -o ~/.provedex/voices/en_US-amy-medium.onnx \
  https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/amy/medium/en_US-amy-medium.onnx

# Run the demo
cargo run -p provedex-server --features demo
```

Open `http://localhost:3000` and hold the mic button. Signed events stream into the right panel. The footer buttons run verify, tamper-test, and export.

## Repository layout

```
crates/
  provedex-core/    signing primitives, hash chain, NDJSON ledger, export bundle
  provedex-cli/     `provedex` command-line tool
  provedex-agent/   localhost HTTP signing daemon (default integration)
  provedex-server/  voice-agent reference demo
bindings/
  python/           PyO3 wrapper (planned)
  node/             napi-rs wrapper (planned)
apps/
  demo-web/         single-page UI for the voice-agent reference
docs/
  spec/             byte-level normative specs (event-schema-v1, canonical-json)
  adr/              architecture decision records
  integration/      framework-specific integration guides
  compliance/       regulator clause mappings (planned)
examples/           runnable integration examples
```

## CLI

```bash
provedex verify                              # verify the local ledger
provedex replay                              # human-readable transcript
provedex export --output ./bundle.json       # signed export bundle for an auditor
```

`provedex verify` exits non-zero if the chain is broken.

## Specs

Normative documents that bindings, auditors, and third-party verifiers implement against:

- [docs/spec/event-schema-v1.md](docs/spec/event-schema-v1.md) - the seven `AgentEvent` variants and their JSON shape, with test vectors.
- [docs/spec/canonical-json.md](docs/spec/canonical-json.md) - the deterministic JSON encoding used for hashing and signing, with test vectors.
- [docs/adr/](docs/adr/) - architecture decision records.

A binding implementation that follows these specs produces signed events byte-identical to the Rust reference.

## Build and test

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

CI runs the same three checks plus `cargo audit` and `cargo deny` on every push and pull request. Mutation testing on `provedex-core` is documented in [CONTRIBUTING.md](CONTRIBUTING.md).

## Performance

Numbers from `cargo bench -p provedex-core` on an Apple M4 Pro running rustc 1.89.0, criterion default sample size (100), 3-second warmup. Reproduce: `cargo bench -p provedex-core`.

| Operation | Median time / event | Throughput |
|-----------|--------------------|------------|
| `canonical_json` (one ModelInvoked event) | 940 ns | 1.06M events/sec |
| `compute_self_hash` (canonical-JSON + SHA-256) | 2.7 us | 366K events/sec |
| `SignedEvent::seal` (full sign, no I/O) | 11.2 us | 89K events/sec |
| `LedgerSession::seal_and_append` (sign + append + fsync_data) | 3.8 ms | 261 events/sec |

`seal_only` isolates the crypto cost. The full append cycle is dominated by `fsync_data` on every event; customers that batch flushes will see the full-cycle cost approach the seal-only number. Voice agents in the typical 50 RPS regime stay well under the per-event budget either way.

## Versioning

Pre-1.0. The public API of `provedex-core` may change between minor versions until the schema is settled. Any breaking change to the canonical-JSON format, the hashed-field set, or the AgentEvent variants requires:

- A new `docs/spec/` document with a bumped version suffix.
- A new ADR superseding the affected prior decision.
- A bump of `ExportBundle::schema_version`.

Bindings, the sidecar, and the CLI track `provedex-core` semver.

## License

Apache-2.0. See [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, commit-message format, and pull request expectations. Security reports go through [SECURITY.md](SECURITY.md).
