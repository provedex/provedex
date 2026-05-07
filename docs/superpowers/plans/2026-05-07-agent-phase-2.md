# Provedex Agent Phase 2 (Hardening) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Harden `provedex-agent` for unattended operation: bound body size on /v1/sign, per-IP rate limit, per-request structured logs, graceful shutdown on SIGTERM, and ledger-writable status reported via /v1/healthz.

**Architecture:** Compose tower middleware on the router (RequestBodyLimitLayer, GovernorLayer for rate limit, TraceLayer for logs). Wire `axum::serve(...).with_graceful_shutdown(...)` with a tokio signal future. Augment AgentState with a cheap writability probe used by healthz.

**Tech Stack:** Rust 1.89, axum 0.7, tower 0.5, tower-http 0.6, tower_governor 0.4 (new dep), tokio signal handling.

---

## Pre-flight

- Branch: `feat/agent-phase-2` (created off main).
- Issues: #14 (body size), #15 (rate limit), #16 (TraceLayer), #17 (graceful shutdown), #18 (fsync probe).
- ADR: not required; phase 2 is enforcement on top of the v1 API surface, no architectural decision changes.

## File Structure

**Create:**
- `crates/provedex-agent/CLAUDE.md` (gitignored, but we add the file locally for future sessions).
- `docs/superpowers/plans/2026-05-07-agent-phase-2.md` (this file).

**Modify:**
- `Cargo.toml` (root) - add `tower_governor` to workspace deps.
- `crates/provedex-agent/Cargo.toml` - add tower_governor.
- `crates/provedex-agent/src/router.rs` - mount middleware (size, rate, trace).
- `crates/provedex-agent/src/state.rs` - add `ledger_writable()` probe.
- `crates/provedex-agent/src/routes/healthz.rs` - report ledger_writable + ledger_path; return 503 when not writable.
- `crates/provedex-agent/src/main.rs` - parse new flags, wire graceful shutdown.
- `crates/provedex-agent/tests/api.rs` - tests for: 413 on oversize body, 429 on rate-limit, 503 on read-only ledger.

## Tasks

### Task 1: branch + plan + push

- [ ] Stage and commit the plan, push branch.

### Task 2: add tower_governor to workspace

Edit root `Cargo.toml` workspace.dependencies:

```toml
tower_governor = "0.4"
```

Then in `crates/provedex-agent/Cargo.toml` `[dependencies]`:

```toml
tower_governor = { workspace = true }
```

Run `cargo build -p provedex-agent` to fetch.

Commit: `chore(agent): add tower_governor for rate limiting`.

### Task 3: failing tests (RED)

Add three new tests to `crates/provedex-agent/tests/api.rs`:

```rust
#[tokio::test]
async fn sign_returns_413_on_oversize_body() {
    let (state, _dir) = fixture().await;
    let app = build_router_with_limits(state, 1024, None);  // 1 KiB cap
    let big = "A".repeat(2048);
    let body = serde_json::to_vec(&serde_json::json!({
        "event": {
            "type": "SessionStarted",
            "payload": { "agent_id": big, "model_id": "m", "session_id": "s" }
        }
    })).unwrap();
    let req = Request::builder()
        .method("POST")
        .uri("/v1/sign")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn healthz_reports_ledger_writable() {
    let (state, _dir) = fixture().await;
    let app = build_router(state);
    let req = Request::builder().method("GET").uri("/v1/healthz").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp.into_body()).await;
    assert_eq!(v["ledger_writable"], true);
    assert!(v["ledger_path"].as_str().unwrap().contains("ledger.ndjson"));
}

#[tokio::test]
async fn healthz_returns_503_when_ledger_not_writable() {
    let dir = tempdir().unwrap();
    let kp = SigningKeypair::generate();
    let ledger = Ledger::open(dir.path().join("ledger.ndjson")).unwrap();
    let session = LedgerSession::open(kp, ledger, "test".into()).unwrap();
    let state = Arc::new(AgentState::new(session));
    // Make the ledger directory read-only so probe fails.
    let mut perms = std::fs::metadata(dir.path()).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(dir.path(), perms).unwrap();
    let app = build_router(state);
    let req = Request::builder().method("GET").uri("/v1/healthz").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    // Restore writable so tempdir cleanup works.
    let mut perms = std::fs::metadata(dir.path()).unwrap().permissions();
    perms.set_readonly(false);
    std::fs::set_permissions(dir.path(), perms).unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
```

Run, verify all fail. Commit: `test(agent): add failing tests for body size, healthz writable probe`.

### Task 4: body size limit + healthz writable probe + status code (GREEN partial)

Add `pub fn build_router_with_limits(state, max_body_bytes, rate_limit) -> Router` so tests can dial limits.

In `state.rs`:

```rust
impl AgentState {
    pub fn ledger_writable(&self) -> bool {
        let path = self.session.ledger().path();
        std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .is_ok()
    }
}
```

In `routes/healthz.rs`:

```rust
#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
    pub session_id: String,
    pub pubkey: String,
    pub ledger_writable: bool,
    pub ledger_path: String,
}

pub async fn healthz(State(state): State<Arc<AgentState>>) -> (StatusCode, Json<Health>) {
    let writable = state.ledger_writable();
    let body = Health {
        status: if writable { "ok" } else { "degraded" },
        session_id: state.session.session_id().to_string(),
        pubkey: state.session.pubkey_hex(),
        ledger_writable: writable,
        ledger_path: state.session.ledger().path().display().to_string(),
    };
    let code = if writable { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(body))
}
```

In `router.rs`:

```rust
use tower_http::limit::RequestBodyLimitLayer;

pub fn build_router(state: Arc<AgentState>) -> Router {
    build_router_with_limits(state, 32 * 1024, None)
}

pub fn build_router_with_limits(
    state: Arc<AgentState>,
    max_body_bytes: usize,
    _rate_limit: Option<RateLimitConfig>,  // wired in Task 5
) -> Router {
    let api = Router::new()
        .route("/v1/healthz", get(routes::healthz::healthz))
        .route("/v1/sign", post(routes::sign::sign).layer(RequestBodyLimitLayer::new(max_body_bytes)))
        .route("/v1/verify", post(routes::verify::verify))
        .with_state(state);
    api
}

pub struct RateLimitConfig {
    pub per_sec: u64,
    pub burst: u32,
}
```

Run tests, verify size-limit + healthz-writable pass. Run fmt + clippy.

Commit: `feat(agent): RequestBodyLimitLayer + ledger writability probe in healthz`.

### Task 5: rate limit /v1/sign

Add tower_governor layer onto /v1/sign only (operator endpoints stay open).

```rust
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

pub fn build_router_with_limits(
    state: Arc<AgentState>,
    max_body_bytes: usize,
    rate_limit: Option<RateLimitConfig>,
) -> Router {
    let mut sign_route = post(routes::sign::sign).layer(RequestBodyLimitLayer::new(max_body_bytes));
    if let Some(rl) = rate_limit {
        let governor_conf = Box::leak(Box::new(
            GovernorConfigBuilder::default()
                .per_second(rl.per_sec)
                .burst_size(rl.burst)
                .finish()
                .expect("invalid governor config"),
        ));
        sign_route = sign_route.layer(GovernorLayer { config: governor_conf });
    }
    Router::new()
        .route("/v1/healthz", get(routes::healthz::healthz))
        .route("/v1/sign", sign_route)
        .route("/v1/verify", post(routes::verify::verify))
        .with_state(state)
}
```

Add a rate-limit integration test: rapid-fire 250 requests at burst=10/per_sec=10, assert at least one 429.

Commit: `feat(agent): per-IP rate limit on /v1/sign`.

### Task 6: TraceLayer for structured logs

```rust
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

let app = build_router_with_limits(state, max, rl)
    .layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO).latency_unit(tower_http::LatencyUnit::Millis)),
    );
```

Done in main.rs after `build_router_with_limits(...)`. No new test (visual inspection during smoke).

Commit: `feat(agent): per-request structured logs via TraceLayer`.

### Task 7: graceful shutdown on SIGTERM/SIGINT

In main.rs:

```rust
use tokio::signal;

async fn shutdown_signal(grace_secs: u64) {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("ctrl_c handler");
    };
    #[cfg(unix)]
    let term = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("sigterm handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
    tracing::info!(grace_secs, "shutdown signal received, draining");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;  // tiny grace before next stage
}

let listener = tokio::net::TcpListener::bind(args.listen).await?;
axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal(args.shutdown_grace_secs))
    .await?;
```

Add CLI flag `--shutdown-grace-secs` default 30.

Smoke: spawn agent, send SIGTERM, expect exit 0 + "draining" log line.

Commit: `feat(agent): graceful shutdown on SIGTERM/SIGINT`.

### Task 8: CLI flags for new knobs

Add to Args:

```rust
#[arg(long, default_value_t = 32 * 1024, env = "PROVEDEX_AGENT_MAX_BODY_BYTES")]
max_body_bytes: usize,

#[arg(long, default_value_t = 100)]
rate_limit_per_sec: u64,

#[arg(long, default_value_t = 200)]
rate_limit_burst: u32,

#[arg(long, default_value_t = 30)]
shutdown_grace_secs: u64,
```

Wire into build_router_with_limits + shutdown_signal.

Commit: `feat(agent): CLI flags for body cap, rate limit, shutdown grace`.

### Task 9: self-review (apply code-review-provedex skill)

- ASCII grep across new files.
- AI-slop adjective check.
- Conventional commit subjects.
- No unwrap outside tests in production code.
- No unsafe.
- Public API rustdoc.
- Run full CI gate: fmt + clippy + test + audit + deny.

Add `crates/provedex-agent/CLAUDE.md` with phase 2 conventions if not present.

Commit any review fixes.

### Task 10: PR + merge

- gh pr create with body in voice.
- Wait for CI green.
- Confidence check (95% bar).
- Auto-merge.
- Close issues #14 #15 #16 #17 #18 (PR body lists them).

## Self-review (writer's pass)

Spec coverage: body cap (Task 4), rate limit (Task 5), TraceLayer (Task 6), graceful shutdown (Task 7), healthz writability (Task 4 + integration test). All acceptance criteria mapped.

Placeholder scan: no TODOs / TBDs.

Type consistency: `RateLimitConfig { per_sec: u64, burst: u32 }`, `build_router_with_limits(Arc<AgentState>, usize, Option<RateLimitConfig>) -> Router`, `Health` struct gets two new fields.

No gaps. Ready.
