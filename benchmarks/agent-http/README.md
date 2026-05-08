# agent-http benchmarks

Sidecar HTTP latency measurement. Spawns a fresh `provedex-agent` per scenario, runs `oha` against `/v1/healthz`, `/v1/sign`, and `/v1/verify`, prints a summary table.

## Run

```
cargo install --locked oha
bash benchmarks/agent-http/run.sh
```

Numbers land in `benchmarks/agent-http/out/*.json` (gitignored). The summary table is printed to stdout.

## What it measures

- **healthz baseline.** GET at concurrency 50, 5000 requests. Confirms the agent's serving floor: how fast a no-op JSON response goes through the router + TraceLayer + ConnectInfo wiring.
- **sign across concurrency.** POST a fixed `ModelInvoked` event at concurrency 1, 10, and 100. Reveals the fsync-bound throughput plateau.
- **verify at varying chain sizes.** Pre-populates a fresh ledger with N events, then measures POST /v1/verify at concurrency 1. Shows verify cost as a function of ledger length.

Each scenario uses a fresh agent + fresh sandboxed ledger so prior runs do not pollute the chain.

## What it does NOT measure

- Cross-machine network latency (deploys vary).
- Production-load simulation under realistic event mixes.
- Effects of `--rate-limit-off` removed (the script disables rate limiting to isolate the agent's true throughput).
- Container vs binary deployment overhead (negligible).

## When to run

- Before tagging a release that touches the agent or any of its hot paths.
- After bumping `axum`, `tower`, `tower-http`, `tower_governor`, `tokio`, or any sign/append code.
- If the README "Performance" table is more than 90 days stale.

## Output stability

The script runs each oha pass with 5000 sample requests. Median values stay within ~10 percent across runs on a quiet machine. Run on AC power, no concurrent workload, no Time Machine backup or Spotlight indexing.

If you see >25 percent variance, your machine is busy. Stop background work and re-run.
