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

| Crate / package | Role | Status |
|------------------|------|--------|
| `provedex-core` | signing primitives, hash chain, NDJSON ledger, export bundle | shipped |
| `provedex-cli` | `provedex` command-line tool: verify, replay, export | shipped |
| `provedex-agent` | localhost HTTP signing daemon for non-Rust customers (default integration) | shipped |
| `provedex` (Python) | Native PyO3 binding: in-process Ed25519 signing, byte-identical to the Rust core, no sidecar. PyPI. See [`bindings/python/provedex/`](bindings/python/provedex/README.md). | shipped |
| `provedex-pipecat` (Python) | Pipecat `FrameProcessor` that signs every frame through the sidecar. PyPI. See [`bindings/python/provedex-pipecat/`](bindings/python/provedex-pipecat/README.md). | shipped |
| `provedex-langchain` (Python) | LangChain `CallbackHandler` that signs every LLM and tool call via the sidecar. Covers LangGraph by inheritance. PyPI. See [`bindings/python/provedex-langchain/`](bindings/python/provedex-langchain/README.md). | shipped |

Framework adapters wrap the sidecar; that is the default integration layer. The native FFI binding (`provedex`, PyO3) is the opt-in in-process fast-path for Python (a napi-rs binding for Node is planned). The sidecar covers every other language via localhost HTTP. See ADR 0004.

The reference voice-agent integration (whisper.cpp + Ollama + Piper) lives in its own repo at [provedex/demo-voice](https://github.com/provedex/demo-voice). It dogfoods this SDK against the published `v0.1.0` tag.

## Install

### Pre-built binary (recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/provedex/provedex/releases) and extract:

```bash
tar -xzf provedex-agent-vN.N.N-aarch64-apple-darwin.tar.gz
sudo install -m 755 provedex-agent /usr/local/bin/
```

### Container (Kubernetes / Docker)

```bash
docker pull ghcr.io/provedex/provedex-agent:latest
docker run --rm -p 8765:8765 ghcr.io/provedex/provedex-agent:latest
```

Multi-arch: `linux/amd64`, `linux/arm64`. Customer apps in the same pod or Docker network POST to the agent's `/v1/sign` endpoint.

### From source

```bash
cargo install --locked --git https://github.com/provedex/provedex --bin provedex-agent
```

### systemd / launchd / Kubernetes manifests

See [`deploy/`](deploy/) for ready-to-adapt manifests.

### Python bindings (PyPI)

```bash
pip install provedex              # native PyO3 binding: in-process signing, no sidecar
pip install provedex-pipecat      # Pipecat voice agents (via sidecar)
pip install provedex-langchain    # LangChain + LangGraph (via sidecar)
```

`provedex` links the Rust core directly and signs in-process; it needs no agent. The wheels are pre-built (cpython 3.11+ on Linux x86_64/aarch64 and macOS arm64), so the install needs no Rust toolchain. `provedex-pipecat` and `provedex-langchain` instead POST to the local `provedex-agent` sidecar: run the agent first, then wire the adapter into your pipeline. See the per-package READMEs for a five-line quickstart.

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

## Voice agent reference (separate repo)

A working voice-agent integration on top of this SDK lives at [`provedex/demo-voice`](https://github.com/provedex/demo-voice). It records audio in the browser, transcribes via whisper.cpp, calls a local Ollama model, signs every step with `provedex-core` v0.1.0 (consumed via git tag), and optionally speaks the reply via Piper.

That repo is the reference dogfood: how a customer integrates `provedex-core` into a real voice pipeline.

## Repository layout

```
crates/
  provedex-core/    signing primitives, hash chain, NDJSON ledger, export bundle
  provedex-cli/     `provedex` command-line tool
  provedex-agent/   localhost HTTP signing daemon (default integration)
bindings/
  python/           PyO3 wrapper (planned)
  node/             napi-rs wrapper (planned)
docs/
  spec/             byte-level normative specs (canonical-json, event-schema-v1, signature-scheme, ledger-format, openapi.yaml)
  adr/              architecture decision records
  integration/      framework-specific integration guides
  compliance/       regulator clause mappings (planned)
examples/           runnable integration examples
deploy/             systemd, launchd, Kubernetes sidecar manifests
benchmarks/         load-test scripts for the sidecar HTTP API
```

The voice-agent reference demo lives in a sibling repo: [`provedex/demo-voice`](https://github.com/provedex/demo-voice).

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
- [docs/spec/signature-scheme.md](docs/spec/signature-scheme.md) - Ed25519 over SHA-256 of canonical-JSON, keypair file format, deterministic signature test vectors.
- [docs/spec/ledger-format.md](docs/spec/ledger-format.md) - NDJSON line format, append/read/crash-recovery semantics for the local ledger.
- [docs/spec/openapi.yaml](docs/spec/openapi.yaml) - OpenAPI 3 description of the sidecar HTTP API. Generated from the agent source via `provedex-agent --print-openapi`.
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

### Sidecar HTTP roundtrip

Numbers from `bash benchmarks/agent-http/run.sh` on the same hardware. Loopback HTTP via `oha`. Each scenario gets a fresh agent with an empty ledger.

| Endpoint | Concurrency | p50 | p95 | p99 | req/sec |
|----------|-------------|-----|-----|-----|---------|
| GET /v1/healthz | 50 | 0.91 ms | 2.78 ms | 7.65 ms | 41,456 |
| POST /v1/sign | 1 | 3.98 ms | 4.69 ms | 5.43 ms | 253 |
| POST /v1/sign | 10 | 35.2 ms | 39.9 ms | 42.8 ms | 280 |
| POST /v1/sign | 100 | 353 ms | 388 ms | 420 ms | 283 |
| POST /v1/verify (100 events) | 1 | 2.09 ms | 2.18 ms | 2.38 ms | 476 |
| POST /v1/verify (1k events) | 1 | 3.97 ms | 4.12 ms | 4.28 ms | 251 |
| POST /v1/verify (10k events) | 1 | 10.9 ms | 11.4 ms | 11.9 ms | 90 |

Reproduce: `bash benchmarks/agent-http/run.sh`.

The /v1/sign cost is roughly 0.2 ms HTTP overhead on top of in-process `seal_and_append` (3.8 ms with fsync). Sign throughput plateaus near 280 req/sec regardless of concurrency because `fsync_data` on every event serializes the writer. The handler offloads the sync work to tokio's blocking pool via `spawn_blocking`, so other in-flight requests on the runtime are not starved during a sign. Customers that need higher per-key throughput either run multiple agents (one per signing identity) or batch flushes (planned). Verify scales linearly with chain size; verify on a 100k-event ledger takes roughly 100 ms.

`/v1/healthz` is fast enough for Kubernetes liveness probes at any reasonable cadence (43k req/sec single-agent throughput).

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
