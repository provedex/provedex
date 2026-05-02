# Provedex

**Cryptographic evidence layer for regulated AI agents.**

When an AI agent makes a decision in healthcare, finance, legal, or any regulated environment, Provedex produces a tamper-evident, cryptographically signed record of exactly what happened — every tool call, every model output, every voice utterance. Regulators, auditors, and courts get a verifiable answer.

## Why

The EU AI Act Article 12 mandate goes into enforcement on August 2, 2026. Every high-risk AI deployment in the EU must produce tamper-evident logs of agent reasoning. Penalties run up to €15M or 3% of global revenue. No funded pure-play exists for cryptographic agent audit ledgers today.

## What

- Open-source Rust SDK with Ed25519 signatures + SHA-256 hash chains + DSSE envelopes
- Local NDJSON ledger, append-only, append-from-anywhere
- Verify / replay / export commands
- Voice agent reference deployment as the v1 vertical (healthcare scribes, financial voice agents)

## Status

Early development. Solo founder building toward YC application demo. Public repo coming online May 2026.

## Documents in this repo

- `STARTUP_CONTEXT.md` — company context, market wedge, competitive landscape, visa/legal notes
- `TECHNICAL_PLAN.md` — 5-day sprint plan to YC-ready demo, architecture, day-by-day deliverables

## License

To be determined (Apache 2.0 or MIT, leaning Apache 2.0 for Sigstore-ecosystem compatibility).
