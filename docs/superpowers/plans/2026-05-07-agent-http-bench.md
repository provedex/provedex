# Sidecar HTTP Latency Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans.

**Goal:** Measure end-to-end latency for the sidecar HTTP API. Spawn a local agent, hit `/v1/sign`, `/v1/verify`, and `/v1/healthz` via `oha`, capture p50/p95/p99 + throughput, paste into README. Closes #29.

**Architecture:** Shell script that builds the agent in release mode, starts it on a random port, hands the port to oha for each scenario, parses the output, prints a summary table. No CI integration; local-only.

**Tech Stack:** `oha` (Rust HTTP benchmark tool), `bash`, `jq` for parsing oha JSON output.

---

## Pre-flight

- Branch: `perf/agent-http-bench` (created off main).
- Issue: #29.
- `oha` installed via `cargo install oha`.

## File Structure

**Create:**
- `benchmarks/agent-http/run.sh` - the benchmark script.
- `benchmarks/agent-http/README.md` - what it measures, how to interpret output.
- `docs/superpowers/plans/2026-05-07-agent-http-bench.md` (this file).

**Modify:**
- `README.md` - add "Sidecar HTTP" subsection under Performance.
- `CONTRIBUTING.md` - mention oha install + when to run agent benches.
- `CLAUDE.md` (root) - add `benchmarks/` row to Repo at a glance + Where new files go.
- `.gitignore` - exclude bench output dirs (`benchmarks/agent-http/out/`).

## Tasks

### Task 1: branch + plan + push

- [ ] Stage and commit the plan, push branch.

### Task 2: write the bench script

`benchmarks/agent-http/run.sh`. Responsibilities:

1. Build provedex-agent in release mode if not already built.
2. Pick a free port (or hardcode 8800 to avoid clash with the default 8765).
3. Start agent in background with the chosen port + a temp ledger + temp key.
4. Wait for healthz to return 200.
5. For each scenario, run oha and capture median + p95 + p99 + req/sec.
6. Stop the agent. Clean up temp files.
7. Print a summary table.

Scenarios:

- `/v1/healthz` baseline: GET, 50 concurrent, 5000 requests.
- `/v1/sign` concurrency=1: POST, 1 worker, 5000 requests.
- `/v1/sign` concurrency=10: POST, 10 workers, 10000 requests.
- `/v1/sign` concurrency=100: POST, 100 workers, 10000 requests.
- `/v1/verify` at chain size N: pre-populate ledger by running `/v1/sign` N times, then run verify 200 times.

Pre-population uses oha or a simple curl loop; the script generates a fresh keypair per scenario via temp dir.

Confidence target: script runs from clean state, prints numbers, exits 0.

Commit: `perf(agent): HTTP latency bench script`.

### Task 3: capture numbers

Run the script. Capture output. Sanity-check against expected ranges:

- /v1/healthz p50 around 0.3-1 ms (no I/O, just JSON encode + ledger probe).
- /v1/sign p50 around 4-6 ms (HTTP + seal_and_append at 3.8 ms).
- /v1/verify @ 1k events around 5-15 ms.
- /v1/verify @ 10k events around 50-150 ms.

If any number is wildly off, debug before pasting into README.

Confidence target: numbers stable across two runs (within 10%).

No new commit; data flows into README.

### Task 4: README "Sidecar HTTP" subsection

Insert under existing "Performance" section, after the in-process numbers table.

```markdown
### Sidecar HTTP roundtrip

Numbers from `bash benchmarks/agent-http/run.sh` on the same hardware (Apple M4 Pro, rustc 1.89.0). Loopback HTTP via `oha`.

| Endpoint | Concurrency | p50 | p95 | p99 | req/sec |
|----------|-------------|-----|-----|-----|---------|
| GET /v1/healthz | 50 | X.X ms | X.X ms | X.X ms | NN |
| POST /v1/sign | 1 | X.X ms | X.X ms | X.X ms | NN |
| POST /v1/sign | 10 | X.X ms | X.X ms | X.X ms | NN |
| POST /v1/sign | 100 | X.X ms | X.X ms | X.X ms | NN |
| POST /v1/verify (1k events) | 1 | X.X ms | X.X ms | X.X ms | NN |
| POST /v1/verify (10k events) | 1 | X.X ms | X.X ms | X.X ms | NN |

Reproduce: `bash benchmarks/agent-http/run.sh`.

The /v1/sign cost is roughly 1-2 ms HTTP overhead on top of the in-process seal_and_append (3.8 ms with fsync). Default rate limit (100 rps per IP) is high enough that single-app pinning to one agent is unaffected; concurrency above 100 starts hitting 429 unless `--rate-limit-off` or a higher per-second is set.
```

(Real numbers replace `X.X ms` from Task 3.)

Commit: `docs(readme): add sidecar HTTP latency numbers`.

### Task 5: CONTRIBUTING + CLAUDE updates

- CONTRIBUTING.md: under Benchmarking, mention `cargo install oha` + `bash benchmarks/agent-http/run.sh` for sidecar perf.
- CLAUDE.md (root): add `benchmarks/` to "Repo at a glance" + "Where new files go".
- .gitignore: exclude `benchmarks/agent-http/out/` if the script writes intermediate files.

Commit: `docs: add agent HTTP bench workflow to CONTRIBUTING + CLAUDE`.

### Task 6: self-review using code-review-provedex skill

- ASCII grep across all new files.
- AI-slop adjective check.
- Conventional commit subjects.
- bench script: no hardcoded paths that break on other dev's machines, no leaked /tmp files, agent reliably stops after run.
- README numbers reproducible (run script twice, assert numbers within 10%).
- CLAUDE.md update is a one-line `benchmarks/` addition; not a navigation entry.

Run full local CI gate: cargo fmt, clippy -D warnings, test, audit, deny.

### Task 7: PR + merge

- gh pr create in voice register.
- Wait for CI green.
- Confidence check (95% bar).
- Auto-merge.
- Close #29.

## Self-review (writer's pass)

Spec coverage: oha install (Task 5), bench script (Task 2), captured numbers (Task 3), README subsection (Task 4), workflow doc (Task 5). All issue acceptance criteria mapped.

Placeholder scan: README numbers are intentional placeholders filled in Task 3.

Type consistency: oha JSON output keys are `summary.average`, `summary.slowest`, `latency_percentiles` — script reads these via jq.

No gaps. Ready.
