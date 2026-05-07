# Provedex Benchmarks v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Publish v0.1 latency numbers for the hot paths in `provedex-core` (canonical_json, compute_self_hash, seal_and_append). Captures the customer-facing perf story before phase 3 distribution.

**Architecture:** Add criterion as a dev-dep, write three benchmark groups in `crates/provedex-core/benches/sign_bench.rs`, run on the local machine, paste numbers into a new `## Performance` section in README. CI does not run benches.

**Tech Stack:** Rust 1.89, criterion 0.5, provedex-core existing primitives.

---

## Pre-flight

- Branch: `feat/benchmarks-v1` (created off main).
- Issue: #22.

## File Structure

**Create:**
- `crates/provedex-core/benches/sign_bench.rs`
- `docs/superpowers/plans/2026-05-07-benchmarks-v1.md` (this file)

**Modify:**
- `Cargo.toml` (root) - add `criterion` to workspace.dependencies.
- `crates/provedex-core/Cargo.toml` - dev-dep on criterion + bench harness declaration.
- `README.md` - new `## Performance` section.
- `CONTRIBUTING.md` - "Benchmarking" section.

## Tasks

### Task 1: branch + plan + push

- [ ] Stage and commit the plan, push branch.

### Task 2: add criterion dev-dep + bench harness

In root `Cargo.toml` workspace.dependencies:

```toml
criterion = { version = "0.5", features = ["html_reports"] }
```

In `crates/provedex-core/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3.12"
criterion = { workspace = true }

[[bench]]
name = "sign_bench"
harness = false
```

Verify `cargo bench -p provedex-core --no-run` builds. Commit: `chore(core): add criterion dev-dep + bench harness`.

### Task 3: write the benchmark file

Three benchmark groups: canonical_json, compute_self_hash, seal_and_append.

```rust
// crates/provedex-core/benches/sign_bench.rs

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use provedex_core::{
    canonical_json, compute_self_hash, AgentEvent, Ledger, LedgerSession, SignedEvent,
    SigningKeypair, GENESIS_PARENT_HASH,
};

fn fixture_event() -> AgentEvent {
    AgentEvent::ModelInvoked {
        model_id: "gpt-4o".into(),
        prompt_sha256: "9f3b2a1c0d4e5f6789abcdef0123456789abcdef0123456789abcdef01234567".into(),
        response_sha256: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".into(),
        prompt_tokens: 482,
        response_tokens: 71,
    }
}

fn bench_canonical_json(c: &mut Criterion) {
    let event = fixture_event();
    let value = serde_json::to_value(&event).unwrap();
    let mut group = c.benchmark_group("canonical_json");
    group.throughput(Throughput::Elements(1));
    group.bench_function("ModelInvoked", |b| {
        b.iter(|| {
            let _ = canonical_json(black_box(&value));
        });
    });
    group.finish();
}

fn bench_compute_self_hash(c: &mut Criterion) {
    let event = fixture_event();
    let mut group = c.benchmark_group("compute_self_hash");
    group.throughput(Throughput::Elements(1));
    group.bench_function("ModelInvoked", |b| {
        b.iter(|| {
            let _ = compute_self_hash(
                black_box(0),
                black_box(1_700_000_000_000_000_000),
                black_box(&event),
                black_box(GENESIS_PARENT_HASH),
            )
            .unwrap();
        });
    });
    group.finish();
}

fn bench_seal_and_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("seal_and_append");
    group.throughput(Throughput::Elements(1));
    group.sample_size(50);

    group.bench_function("with_fsync", |b| {
        let dir = tempfile::tempdir().unwrap();
        let kp = SigningKeypair::generate();
        let ledger = Ledger::open(dir.path().join("ledger.ndjson")).unwrap();
        let session = LedgerSession::open(kp, ledger, "bench".into()).unwrap();
        b.iter(|| {
            let _ = session.seal_and_append(black_box(fixture_event())).unwrap();
        });
    });

    // Standalone seal (no append, no fsync) to isolate the crypto cost from the I/O cost.
    group.bench_function("seal_only", |b| {
        let kp = SigningKeypair::generate();
        let mut parent = GENESIS_PARENT_HASH.to_string();
        let mut seq = 0u64;
        b.iter(|| {
            let signed = SignedEvent::seal(
                black_box(seq),
                black_box(fixture_event()),
                black_box(&parent),
                black_box(&kp),
            )
            .unwrap();
            parent = signed.self_hash;
            seq += 1;
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_canonical_json,
    bench_compute_self_hash,
    bench_seal_and_append
);
criterion_main!(benches);
```

- [ ] Build the bench: `cargo bench -p provedex-core --no-run`.
- [ ] Run: `cargo bench -p provedex-core`.
- [ ] Capture the printed numbers.

Commit: `perf(core): criterion benchmarks for sign hot paths`.

### Task 4: README Performance section

Insert before the `## License` section in README.md.

Use the actual numbers from Task 3 output. Format:

```markdown
## Performance

Numbers from `cargo bench -p provedex-core` on a [hardware label] running rustc 1.89.0, criterion default sample size.

| Operation | Time / event | Throughput |
|-----------|-------------|------------|
| `canonical_json` (one ModelInvoked event) | X.X us | NN events/sec |
| `compute_self_hash` (canonical-JSON + SHA-256) | X.X us | NN events/sec |
| `SignedEvent::seal` (sign without I/O) | X.X us | NN events/sec |
| `LedgerSession::seal_and_append` (full cycle, fsync after each event) | X.X us | NN events/sec |

Reproduce: `cargo bench -p provedex-core`.

The seal-only number isolates the crypto cost; the full-cycle number includes the fsync_data the ledger does after every append. Customers that move from per-event fsync to batched flushes will see the full-cycle cost approach the seal-only cost.
```

Commit: `docs(readme): add Performance section with v0.1 numbers`.

### Task 5: CONTRIBUTING.md "Benchmarking" section

Insert before the "Mutation testing" section.

```markdown
## Benchmarking

Latency numbers for `provedex-core` hot paths live in `crates/provedex-core/benches/sign_bench.rs`. Run before tagging a release:

\`\`\`
cargo bench -p provedex-core
\`\`\`

To diff against a saved baseline:

\`\`\`
cargo bench -p provedex-core -- --save-baseline v0.1.0
# ... make changes ...
cargo bench -p provedex-core -- --baseline v0.1.0
\`\`\`

CI does not run benches; criterion needs warmup time and stable hardware. Run them locally on a quiet machine. Update the README "Performance" table any time a benchmark moves more than 10 percent.
```

Commit: `docs(contributing): benchmarking workflow`.

### Task 6: self-review

- ASCII grep across all modified files.
- AI-slop adjective check.
- Conventional commit subjects.
- No regressions: `cargo test --workspace --all-features`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- `cargo audit` + `cargo deny check`.
- Apply code-review-provedex auto-block list.

Fix any findings. Commit.

### Task 7: PR + merge

- gh pr create in voice register.
- Wait for CI green.
- Confidence check (95% bar).
- Auto-merge.
- Close #22.

## Self-review (writer's pass)

Spec coverage: criterion + bench harness (Task 2), three groups (Task 3), README numbers (Task 4), CONTRIBUTING workflow (Task 5). All issue acceptance criteria mapped.

Placeholder scan: README numbers section says "X.X us" - those are intentional placeholders that get filled in Task 3 from real bench output. Marked as needing the run-output paste.

Type consistency: `fixture_event() -> AgentEvent` used in all three groups. `seal_only` benchmark mutates parent + seq across iterations to model the real chain (each iteration's parent becomes the previous self_hash).

No gaps. Ready.
