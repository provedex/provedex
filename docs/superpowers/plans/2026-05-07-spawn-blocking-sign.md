# spawn_blocking on /v1/sign

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans.

**Goal:** Wrap the sync `seal_and_append` call in `/v1/sign` with `tokio::task::spawn_blocking` so its fsync + hash + sign work does not block the tokio runtime thread.

**Architecture:** `AgentState` is already inside `Arc`. Clone the `Arc` into the closure passed to `spawn_blocking`. The closure returns the `Result<SignedEvent, SessionError>` and the outer async fn awaits the join handle.

**Tech stack:** No new deps. tokio is already wired.

## Pre-flight

- Branch: `perf/spawn-blocking-sign` (created off main).
- Issue: #33.
- Constraint: `LedgerSession` and therefore `AgentState` must be `Send + Sync` for `spawn_blocking` to compile. Already true (the existing tower router needs it).

## File Structure

**Modify:**
- `crates/provedex-agent/src/routes/sign.rs` - wrap call.
- `README.md` - if benchmark numbers improve materially, refresh the table.

**No changes:**
- `provedex-core` API surface (still sync).
- Other route handlers.

## Tasks

### Task 1: branch + plan + push

- [x] Branch + commit plan + push.

### Task 2: spawn_blocking wrap

In `routes/sign.rs`:

```rust
let state_for_blocking = state.clone();
let signed = tokio::task::spawn_blocking(move || {
    state_for_blocking.session.seal_and_append(req.event)
})
.await
.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join error: {e}")))?
.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
Ok(Json(signed))
```

The join error path covers tokio runtime panic / cancel. Convert to 500 with the panic message; in practice should never fire because `seal_and_append` does not panic.

### Task 3: build + test + clippy

- `cargo build -p provedex-agent`
- `cargo clippy -p provedex-agent --all-targets -- -D warnings`
- `cargo test -p provedex-agent --all-features`

### Task 4: rerun benchmark

- `bash benchmarks/agent-http/run.sh`
- Capture before / after p50/p95/p99 numbers at c=100 specifically.
- If p99 drops by more than 20%, update README "Sidecar HTTP roundtrip" table.

### Task 5: self-review with code-review-provedex

- Auto-block invariants pass (no canonical-JSON change, no schema change, no new pub item, conventional commits, ascii, no AI slop).
- Universal style.
- Hot-path note: this PR removes blocking I/O from a hot path; opposite of a regression.

### Task 6: PR + merge

- voice-aditya register PR body.
- Wait CI green.
- Auto-merge.
- Close #33.

## Self-review (writer's pass)

Coverage: issue acceptance criteria 1-3 mapped to tasks. (4) bench rerun in task 4. (5) operator endpoints out of scope explicitly.

Risk: `Arc<AgentState>` clone is cheap (one atomic increment). The mutex inside `LedgerSession` still serializes appends, so spawn_blocking does not increase fsync throughput. The win is freeing the runtime thread to service other requests during fsync. Aligns with axum + tokio best practice.

No type gaps. Ready.
