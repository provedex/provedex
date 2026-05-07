# 0004. Sidecar binary as the default integration for non-Rust customers

Date: 2026-05-07
Status: accepted

## Context

Customer applications run in many languages: Python (FastAPI, asyncio, Django), Node / TypeScript (Express, Next.js), Java (Spring), Go, Ruby, PHP. Each customer needs a way to call `sign_event(...)` from inside their app.

The original plan was per-language native bindings: PyO3 for Python, napi-rs for Node, JNI for Java, cgo for Go, and so on. Each binding is its own crate, its own build matrix (macOS arm64 / x86_64, linux x86_64 / arm64, windows x86_64), its own publish surface (PyPI, npm, Maven Central, ...), its own version-skew matrix against `provedex-core`. For a solo founder pre-funding, that is N parallel projects where N is the number of supported languages, each one a real distraction from the cryptographic primitive.

Three alternatives were considered:

1. Per-language native FFI binding for every supported language.
2. A single sidecar binary (`provedex-agent`) that exposes a localhost HTTP signing API; customer apps integrate via a thin HTTP client in their language.
3. A pure-WASM module that every runtime can call. Cross-language but immature tooling for some hosts (Java's WASM story is unfinished as of 2026).

## Decision

We ship `provedex-agent` as the default integration for any language other than Rust.

- `provedex-agent` is a single Rust binary that wraps `provedex-core`.
- It binds `127.0.0.1:8765` by default (localhost only; refuses `0.0.0.0` without an explicit insecure-mode flag).
- Customers POST event payloads as JSON; the agent signs and appends to the local NDJSON ledger, returns the full `SignedEvent`.
- A "thin HTTP client" in each language is ~50 lines and is part of the integration guide, not a separate published package.
- `~/.provedex/ledger.ndjson` and `~/.provedex/keys/ed25519.key` remain canonical paths, configurable via env.
- Native FFI bindings (`bindings/python/`, `bindings/node/`) stay on the roadmap as **optional fast-paths** for customers who have measured the localhost roundtrip (1-2 ms) and need sub-millisecond signing on a hot path.

We explicitly drop from the roadmap: `bindings/java/`, `bindings/go/`, `bindings/ruby/`, `bindings/php/`. Customers in those languages use the sidecar.

## Consequences

- One Rust binary covers every language we did not have a binding for. Day-one reach is N=any.
- Customer's signing key never leaves the customer's host. The agent reads it once at startup from a path or a secret-manager URI. The aggregator (separate, hosted, paid tier) never receives the key.
- Byte-compat across languages comes free: the sidecar IS the reference implementation. There is no second canonical-JSON encoder to keep in sync.
- Latency: 1-2 ms per signed event over localhost. Voice agent total budget is 200-500 ms. Negligible.
- Customer ops: one extra process to deploy + monitor. Standard sidecar pattern in K8s, systemd unit on a VM, launchd plist on macOS, or a service entry on Windows. Documented under `docs/integration/sidecar.md`.
- Customer trust: the binary is open-source Rust, signed releases on GitHub, reproducible builds (eventual goal). Audit-able before deploy.
- We delete a large amount of imagined work: 5+ language bindings worth of toolchains, build matrices, and release surfaces.
- If a future customer needs a binding for a language we do not have, the sidecar covers them. Native binding becomes an optional follow-up driven by measured need, not anticipated need.

## What this does not change

- `provedex-core` stays the canonical Rust library. The sidecar links to it. Future native bindings link to it. Specs (canonical JSON, hash chain, signature scheme) remain authoritative.
- Compatibility tests in `tests/compat/` still apply when a native binding ships; the binding must produce byte-identical output to the sidecar (and to `provedex-core` directly).
- The `provedex-cli` does not change. It remains the operator-facing tool for verify / replay / export.
- Hosted aggregator architecture does not change. Customers push from sidecar (or from a native binding) over HTTPS; the aggregator stores + provides verification API.

## Concrete API surface for v1

Routes:

- `POST /v1/sign` - body `{ "event": <AgentEvent JSON> }` - returns full `SignedEvent`.
- `POST /v1/verify` - returns `ChainReport`.
- `GET  /v1/healthz` - returns agent status, current public key, session ID.

Out of scope for v1, queued as follow-ups:

- Streaming sign API (gRPC or chunked HTTP) for high-RPS customers.
- TLS support for non-localhost binds.
- Multi-tenant isolation (multiple key namespaces in one agent).
- Aggregator forwarding (push to hosted aggregator over HTTPS).
- SIEM exporters (Splunk HEC, Datadog log ingest, Elastic).
- Metrics endpoint (`GET /metrics` Prometheus format).

Each is its own follow-up issue, none required for the sidecar to replace native bindings as the default integration.

## References

- Adjacent precedent: Datadog Agent (sidecar pattern, native bindings as fast path), OpenTelemetry Collector (sidecar pattern, language SDKs that target it), HashiCorp Vault Agent.
- Issue tracking the implementation: provedex/provedex#11.
- Deprioritized: provedex/provedex#6 (Python binding), #7 (Node binding).
