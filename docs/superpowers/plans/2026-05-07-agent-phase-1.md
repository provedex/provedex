# Provedex Agent Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `crates/provedex-agent/` v1: a single Rust binary that exposes a localhost HTTP signing API so any-language customer apps (Python, Node, Java, Go, Ruby, PHP) can `POST /v1/sign` and land a SignedEvent in the local ledger without per-language FFI bindings.

**Architecture:** Reuse the `LedgerSession` primitive from `provedex-core` (extracted in PR #12). The agent is a thin Axum HTTP wrapper plus a CLI: parse flags, open the LedgerSession, expose three routes (`/v1/sign`, `/v1/verify`, `/v1/healthz`), bind `127.0.0.1:8765` by default. Refuse non-loopback bind unless `--insecure-allow-public` is passed.

**Tech Stack:** Rust 1.89, Axum 0.7, tokio, tower / tower-http, clap, tracing, reqwest (test-only), uuid, dirs.

---

## Pre-flight

- Branch: `feat/agent-phase-1` (already created off main).
- Issue: #11 (sidecar scaffold).
- ADR: 0004 (sidecar as default integration). Existing.
- LedgerSession landed in PR #12.

## File Structure

**Create:**
- `crates/provedex-agent/Cargo.toml`
- `crates/provedex-agent/src/main.rs` - clap parsing + tokio runtime + bind.
- `crates/provedex-agent/src/lib.rs` - re-exports for testability.
- `crates/provedex-agent/src/state.rs` - `AgentState` wrapping `LedgerSession`.
- `crates/provedex-agent/src/router.rs` - `build_router(state)` for tests + main.
- `crates/provedex-agent/src/routes/mod.rs`
- `crates/provedex-agent/src/routes/healthz.rs` - GET `/v1/healthz`.
- `crates/provedex-agent/src/routes/sign.rs` - POST `/v1/sign`.
- `crates/provedex-agent/src/routes/verify.rs` - POST `/v1/verify`.
- `crates/provedex-agent/tests/api.rs` - integration tests via `tower::ServiceExt::oneshot`.
- `docs/integration/sidecar.md` - integration guide with curl + per-language clients.

**Modify:**
- `Cargo.toml` (workspace root) - add `crates/provedex-agent` to members.

## Task Decomposition

---

### Task 1: Branch + plan + push

- [ ] **Step 1: Stage and commit the plan**

```bash
git add docs/superpowers/plans/2026-05-07-agent-phase-1.md
git commit -m "docs(plan): provedex-agent phase 1"
git push -u origin feat/agent-phase-1
```

Expected: branch published.

---

### Task 2: Scaffold provedex-agent crate

**Files:**
- Create: `crates/provedex-agent/Cargo.toml`
- Create: `crates/provedex-agent/src/main.rs` (placeholder)
- Create: `crates/provedex-agent/src/lib.rs` (placeholder)
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Add to workspace members**

Edit root `Cargo.toml`. Add `"crates/provedex-agent"` to the `members` array.

- [ ] **Step 2: Write `crates/provedex-agent/Cargo.toml`**

```toml
[package]
name = "provedex-agent"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Provedex sidecar daemon: localhost HTTP signing API for any-language customer apps"

[[bin]]
name = "provedex-agent"
path = "src/main.rs"

[lib]
path = "src/lib.rs"

[dependencies]
provedex-core = { path = "../provedex-core", version = "0.1.0" }
anyhow = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
clap = { workspace = true }
tokio = { workspace = true }
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
uuid = { workspace = true }
dirs = { workspace = true }

[dev-dependencies]
http-body-util = "0.1"
tower = { workspace = true, features = ["util"] }
tempfile = "3.12"
```

- [ ] **Step 3: Write placeholder `src/lib.rs`**

```rust
pub mod router;
pub mod routes;
pub mod state;
```

- [ ] **Step 4: Write placeholder `src/main.rs`**

```rust
fn main() {
    eprintln!("provedex-agent: unimplemented");
}
```

- [ ] **Step 5: Verify workspace compiles**

```bash
cargo check --workspace --all-features 2>&1 | tail -3
```

Expected: clean build (compile errors expected for missing modules `router`, `routes`, `state`).

- [ ] **Step 6: Add stub modules so it compiles**

Create empty stubs that the tests will replace:

```rust
// src/state.rs
use provedex_core::LedgerSession;
pub struct AgentState { pub session: LedgerSession }
```

```rust
// src/router.rs
use std::sync::Arc;
use axum::Router;
use crate::state::AgentState;
pub fn build_router(_state: Arc<AgentState>) -> Router { Router::new() }
```

```rust
// src/routes/mod.rs
pub mod healthz;
pub mod sign;
pub mod verify;
```

```rust
// src/routes/healthz.rs (placeholder)
// src/routes/sign.rs (placeholder)
// src/routes/verify.rs (placeholder)
```

Each placeholder is `// stub` for now; production code lands in tasks 4-6.

- [ ] **Step 7: cargo check**

```bash
cargo check --workspace --all-features 2>&1 | tail -3
```

Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/provedex-agent
git commit -m "chore(agent): scaffold provedex-agent crate"
```

---

### Task 3: Write failing integration tests (RED)

**Files:**
- Create: `crates/provedex-agent/tests/api.rs`

- [ ] **Step 1: Write integration tests for all three routes**

```rust
// crates/provedex-agent/tests/api.rs

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use provedex_agent::router::build_router;
use provedex_agent::state::AgentState;
use provedex_core::{Ledger, LedgerSession, SigningKeypair};
use serde_json::{json, Value};
use tempfile::tempdir;
use tower::ServiceExt;

async fn fixture() -> (Arc<AgentState>, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let kp = SigningKeypair::generate();
    let ledger = Ledger::open(dir.path().join("ledger.ndjson")).unwrap();
    let session = LedgerSession::open(kp, ledger, "test-agent".into()).unwrap();
    (Arc::new(AgentState { session }), dir)
}

async fn body_json(body: Body) -> Value {
    let bytes = to_bytes(body, 1 << 20).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn healthz_reports_pubkey_and_session_id() {
    let (state, _dir) = fixture().await;
    let pubkey_expected = state.session.pubkey_hex();
    let app = build_router(state);
    let req = Request::builder()
        .method("GET")
        .uri("/v1/healthz")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp.into_body()).await;
    assert_eq!(v["status"], "ok");
    assert_eq!(v["pubkey"], pubkey_expected);
    assert_eq!(v["session_id"], "test-agent");
}

#[tokio::test]
async fn sign_accepts_event_and_returns_signed() {
    let (state, _dir) = fixture().await;
    let app = build_router(state);
    let body = json!({
        "event": {
            "type": "SessionStarted",
            "payload": {
                "agent_id": "demo",
                "model_id": "m",
                "session_id": "s"
            }
        }
    });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/sign")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp.into_body()).await;
    assert_eq!(v["seq"], 0);
    assert_eq!(
        v["parent_hash"],
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert!(v["self_hash"].as_str().unwrap().len() == 64);
    assert!(v["signature"].as_str().unwrap().len() == 128);
}

#[tokio::test]
async fn sign_chains_subsequent_events() {
    let (state, _dir) = fixture().await;
    let app = build_router(state);
    let post_event = |app: axum::Router, agent_id: &str| {
        let body = json!({
            "event": {
                "type": "SessionStarted",
                "payload": { "agent_id": agent_id, "model_id": "m", "session_id": "s" }
            }
        });
        let req = Request::builder()
            .method("POST")
            .uri("/v1/sign")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        async move { app.oneshot(req).await.unwrap() }
    };

    let resp1 = post_event(app.clone(), "first").await;
    let v1 = body_json(resp1.into_body()).await;

    let resp2 = post_event(app, "second").await;
    let v2 = body_json(resp2.into_body()).await;

    assert_eq!(v1["seq"], 0);
    assert_eq!(v2["seq"], 1);
    assert_eq!(v2["parent_hash"], v1["self_hash"]);
}

#[tokio::test]
async fn verify_returns_chain_report() {
    let (state, _dir) = fixture().await;
    state
        .session
        .seal_and_append(provedex_core::AgentEvent::SessionStarted {
            agent_id: "a".into(),
            model_id: "m".into(),
            session_id: "s".into(),
        })
        .unwrap();
    let app = build_router(state);
    let req = Request::builder()
        .method("POST")
        .uri("/v1/verify")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp.into_body()).await;
    assert_eq!(v["status"], "valid");
    assert_eq!(v["event_count"], 1);
}

#[tokio::test]
async fn sign_rejects_invalid_event_payload() {
    let (state, _dir) = fixture().await;
    let app = build_router(state);
    let body = json!({ "event": { "type": "NotARealVariant" } });
    let req = Request::builder()
        .method("POST")
        .uri("/v1/sign")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 2: Run tests, verify they fail**

```bash
cargo test -p provedex-agent --tests 2>&1 | grep -E "FAILED|test result|error\[" | head -10
```

Expected: build fails (router stub returns empty Router so routes 404; healthz/sign/verify modules don't exist as expected).

- [ ] **Step 3: Commit**

```bash
git add crates/provedex-agent/tests/api.rs
git commit -m "test(agent): add failing integration tests for v1 routes"
```

---

### Task 4: Implement state + healthz route (GREEN partial)

**Files:**
- Modify: `crates/provedex-agent/src/state.rs`
- Modify: `crates/provedex-agent/src/routes/healthz.rs`
- Modify: `crates/provedex-agent/src/router.rs`

- [ ] **Step 1: Replace state stub with real implementation**

```rust
// crates/provedex-agent/src/state.rs
use provedex_core::LedgerSession;

/// Owns the LedgerSession the agent serves over HTTP. One LedgerSession per
/// agent process. Multi-tenant isolation is a follow-up (see ADR 0004).
pub struct AgentState {
    pub session: LedgerSession,
}

impl AgentState {
    pub fn new(session: LedgerSession) -> Self {
        Self { session }
    }
}
```

- [ ] **Step 2: Implement /v1/healthz**

```rust
// crates/provedex-agent/src/routes/healthz.rs
use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::AgentState;

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
    pub session_id: String,
    pub pubkey: String,
}

pub async fn healthz(State(state): State<Arc<AgentState>>) -> Json<Health> {
    Json(Health {
        status: "ok",
        session_id: state.session.session_id().to_string(),
        pubkey: state.session.pubkey_hex(),
    })
}
```

- [ ] **Step 3: Update router builder**

```rust
// crates/provedex-agent/src/router.rs
use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::routes;
use crate::state::AgentState;

pub fn build_router(state: Arc<AgentState>) -> Router {
    Router::new()
        .route("/v1/healthz", get(routes::healthz::healthz))
        .with_state(state)
}
```

- [ ] **Step 4: Run healthz test, verify passes**

```bash
cargo test -p provedex-agent --tests healthz_reports 2>&1 | tail -5
```

Expected: 1 test passes; sign + verify still fail.

- [ ] **Step 5: Commit**

```bash
git add crates/provedex-agent/src
git commit -m "feat(agent): healthz route + AgentState"
```

---

### Task 5: Implement /v1/sign + /v1/verify (GREEN)

**Files:**
- Modify: `crates/provedex-agent/src/routes/sign.rs`
- Modify: `crates/provedex-agent/src/routes/verify.rs`
- Modify: `crates/provedex-agent/src/router.rs`

- [ ] **Step 1: Implement /v1/sign**

```rust
// crates/provedex-agent/src/routes/sign.rs
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use provedex_core::{AgentEvent, SignedEvent};
use serde::Deserialize;

use crate::state::AgentState;

#[derive(Deserialize)]
pub struct SignRequest {
    pub event: AgentEvent,
}

pub async fn sign(
    State(state): State<Arc<AgentState>>,
    body: Result<Json<SignRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<SignedEvent>, (StatusCode, String)> {
    let Json(req) = body.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let signed = state
        .session
        .seal_and_append(req.event)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(signed))
}
```

- [ ] **Step 2: Implement /v1/verify**

```rust
// crates/provedex-agent/src/routes/verify.rs
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use provedex_core::{verify_chain, ChainReport};

use crate::state::AgentState;

pub async fn verify(
    State(state): State<Arc<AgentState>>,
) -> Result<Json<ChainReport>, (StatusCode, String)> {
    let events = state
        .session
        .ledger()
        .read_all()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(verify_chain(&events)))
}
```

- [ ] **Step 3: Update router**

```rust
// crates/provedex-agent/src/router.rs
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

use crate::routes;
use crate::state::AgentState;

pub fn build_router(state: Arc<AgentState>) -> Router {
    Router::new()
        .route("/v1/healthz", get(routes::healthz::healthz))
        .route("/v1/sign", post(routes::sign::sign))
        .route("/v1/verify", post(routes::verify::verify))
        .with_state(state)
}
```

- [ ] **Step 4: Run all tests, verify they pass**

```bash
cargo test -p provedex-agent --tests 2>&1 | grep "test result"
```

Expected: 5 tests pass.

- [ ] **Step 5: Run fmt + clippy**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -3
```

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/provedex-agent/src
git commit -m "feat(agent): /v1/sign and /v1/verify routes"
```

---

### Task 6: CLI flags + main.rs + loopback enforcement

**Files:**
- Modify: `crates/provedex-agent/src/main.rs`

- [ ] **Step 1: Implement main.rs with clap**

```rust
// crates/provedex-agent/src/main.rs
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use provedex_agent::router::build_router;
use provedex_agent::state::AgentState;
use provedex_core::{default_key_path, default_ledger_path, Ledger, LedgerSession, SigningKeypair};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "provedex-agent", version, about = "Provedex sidecar HTTP signing daemon")]
struct Args {
    /// Listen address. Default 127.0.0.1:8765. Non-loopback addresses require
    /// --insecure-allow-public.
    #[arg(long, default_value = "127.0.0.1:8765", env = "PROVEDEX_AGENT_LISTEN")]
    listen: SocketAddr,

    /// Override the NDJSON ledger path. Default ~/.provedex/ledger.ndjson.
    #[arg(long, env = "PROVEDEX_LEDGER")]
    ledger: Option<PathBuf>,

    /// Override the Ed25519 key path. Default ~/.provedex/keys/ed25519.key.
    #[arg(long, env = "PROVEDEX_KEY")]
    key: Option<PathBuf>,

    /// Allow binding to a non-loopback address. Off by default; the agent has
    /// no auth and must not face the public internet without TLS + auth in
    /// front of it.
    #[arg(long, default_value_t = false)]
    insecure_allow_public: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,axum=info")),
        )
        .compact()
        .init();

    let args = Args::parse();

    if !args.listen.ip().is_loopback() && !args.insecure_allow_public {
        anyhow::bail!(
            "refusing to bind {} (non-loopback) without --insecure-allow-public; \
             this daemon has no auth and must not face the public internet",
            args.listen
        );
    }

    let ledger_path = args.ledger.unwrap_or(default_ledger_path()?);
    let key_path = args.key.unwrap_or(default_key_path()?);

    let keypair = SigningKeypair::load_or_create(&key_path)
        .with_context(|| format!("loading or creating keypair at {}", key_path.display()))?;
    let ledger = Ledger::open(&ledger_path)
        .with_context(|| format!("opening ledger at {}", ledger_path.display()))?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let session = LedgerSession::open(keypair, ledger, session_id)
        .context("opening ledger session")?;
    let state = Arc::new(AgentState::new(session));

    let app = build_router(state);

    tracing::info!(addr = %args.listen, "provedex-agent listening");
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 2: Build + run smoke**

```bash
cargo build -p provedex-agent 2>&1 | tail -3
```

Expected: clean.

- [ ] **Step 3: cargo fmt + clippy + test**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -3
cargo test --workspace --all-features 2>&1 | grep "test result"
```

Expected: green across the board.

- [ ] **Step 4: Manual smoke**

```bash
rm -f ~/.provedex/ledger.ndjson
cargo run -p provedex-agent &
sleep 3
python3 -c "
import json, urllib.request
def post(url, body=b''):
    req = urllib.request.Request(url, data=body, method='POST',
        headers={'content-type':'application/json'})
    return urllib.request.urlopen(req).read().decode()
print(urllib.request.urlopen('http://127.0.0.1:8765/v1/healthz').read().decode())
event = json.dumps({'event':{'type':'SessionStarted','payload':{'agent_id':'curl-test','model_id':'m','session_id':'s'}}}).encode()
print(post('http://127.0.0.1:8765/v1/sign', event))
print(post('http://127.0.0.1:8765/v1/verify'))
"
pkill -f provedex-agent
rm -f ~/.provedex/ledger.ndjson
```

Expected: healthz returns 200 with status ok. sign returns a SignedEvent JSON. verify returns valid ChainReport with event_count >= 1.

- [ ] **Step 5: Commit**

```bash
git add crates/provedex-agent/src/main.rs
git commit -m "feat(agent): CLI with loopback-only default + key/ledger overrides"
```

---

### Task 7: Integration guide

**Files:**
- Create: `docs/integration/sidecar.md`

- [ ] **Step 1: Write the integration guide**

```markdown
# Sidecar integration guide

`provedex-agent` is a single Rust binary that exposes a localhost HTTP signing
API. Customer applications in any language `POST` event payloads as JSON; the
agent signs locally with the customer's Ed25519 key and appends to the on-disk
NDJSON ledger.

This is the default integration for any language other than Rust (decision in
ADR 0004). Native bindings (`bindings/python`, `bindings/node`) are optional
fast-paths for sub-millisecond signing.

## Install

After v1 release, prebuilt binaries land on GitHub Releases. Until then:

\`\`\`
git clone https://github.com/provedex/provedex
cd provedex
cargo build --release -p provedex-agent
sudo install -m 755 target/release/provedex-agent /usr/local/bin/
\`\`\`

## Run

\`\`\`
provedex-agent
\`\`\`

Defaults:

- Listen: \`127.0.0.1:8765\` (loopback only).
- Ledger: \`~/.provedex/ledger.ndjson\`.
- Key: \`~/.provedex/keys/ed25519.key\` (auto-generated on first run).

Override via flags or env:

\`\`\`
provedex-agent --listen 127.0.0.1:9001
PROVEDEX_LEDGER=/var/log/provedex/ledger.ndjson provedex-agent
\`\`\`

The agent refuses to bind to a non-loopback address without
\`--insecure-allow-public\`. There is no auth on the API; production deploys
must front it with TLS + auth (Envoy, nginx, sidecar in a service mesh).

## API

### \`GET /v1/healthz\`

\`\`\`
curl http://127.0.0.1:8765/v1/healthz
{"status":"ok","session_id":"...","pubkey":"..."}
\`\`\`

### \`POST /v1/sign\`

Body: \`{ "event": <AgentEvent JSON> }\`. Returns the full SignedEvent.

\`\`\`
curl -X POST http://127.0.0.1:8765/v1/sign \\
  -H 'content-type: application/json' \\
  -d '{"event":{"type":"SessionStarted","payload":{"agent_id":"demo","model_id":"m","session_id":"s"}}}'
\`\`\`

### \`POST /v1/verify\`

\`\`\`
curl -X POST http://127.0.0.1:8765/v1/verify
{"status":"valid","event_count":1,...}
\`\`\`

## Per-language clients

### Python

\`\`\`python
import json, urllib.request
def sign(event):
    req = urllib.request.Request(
        "http://127.0.0.1:8765/v1/sign",
        data=json.dumps({"event": event}).encode(),
        method="POST",
        headers={"content-type": "application/json"},
    )
    return json.loads(urllib.request.urlopen(req).read())

signed = sign({
    "type": "ModelInvoked",
    "payload": {
        "model_id": "gpt-4o",
        "prompt_sha256": "9f3b...",
        "response_sha256": "a1c2...",
        "prompt_tokens": 482,
        "response_tokens": 71,
    },
})
print(signed["seq"], signed["self_hash"])
\`\`\`

### Node / TypeScript

\`\`\`ts
async function sign(event: object) {
  const res = await fetch("http://127.0.0.1:8765/v1/sign", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ event }),
  });
  if (!res.ok) throw new Error(await res.text());
  return await res.json();
}
\`\`\`

### Java

\`\`\`java
HttpRequest req = HttpRequest.newBuilder()
    .uri(URI.create("http://127.0.0.1:8765/v1/sign"))
    .header("content-type", "application/json")
    .POST(BodyPublishers.ofString(eventJson))
    .build();
HttpResponse<String> resp = httpClient.send(req, BodyHandlers.ofString());
\`\`\`

### Go

\`\`\`go
body, _ := json.Marshal(map[string]any{"event": event})
resp, err := http.Post("http://127.0.0.1:8765/v1/sign", "application/json",
    bytes.NewReader(body))
\`\`\`

### Ruby

\`\`\`ruby
require "net/http"; require "json"
res = Net::HTTP.post(
  URI("http://127.0.0.1:8765/v1/sign"),
  { event: event }.to_json,
  "content-type" => "application/json",
)
\`\`\`

## Verify your integration

After signing some events from your app:

\`\`\`
provedex verify
\`\`\`

A green report (\`status: valid\`) means every event your app emitted is
cryptographically chained and verifiable by anyone with the public key.

If your CI ever produces a red report, the chain is broken; check whether
something is writing to the ledger outside the sidecar.

## Out of scope for v1

- TLS support for non-loopback binds.
- Bearer-token auth on the API.
- Multi-tenant isolation (multiple key namespaces).
- Aggregator forwarding (push to hosted aggregator).
- SIEM exporters (Splunk, Datadog, Elastic).
- Streaming sign API.

Each lands in a follow-up issue.
```

- [ ] **Step 2: Verify ASCII**

```bash
grep -nP '[^\x00-\x7F]' docs/integration/sidecar.md && echo "FOUND non-ascii" || echo "ascii only"
```

Expected: ascii only.

- [ ] **Step 3: Commit**

```bash
git add docs/integration/sidecar.md
git commit -m "docs(integration): sidecar guide with curl + per-language clients"
```

---

### Task 8: Self-review using code-review-provedex skill

**Files:** none modified.

- [ ] **Step 1: Walk diff**

```bash
git diff main...HEAD --stat
```

- [ ] **Step 2: Apply auto-block invariants**

- canonical_json + compute_self_hash + GENESIS_PARENT_HASH unchanged. (Agent only orchestrates; reuses LedgerSession.) ✓
- Public API in core: no change in this PR. ✓
- New `pub` items in agent: AgentState, build_router, SignRequest, Health. They are part of an internal lib used by tests + main; rustdoc on AgentState already added. Add doc comments to the rest if missing.
- Conventional commit subjects across the branch.
- ASCII only: `grep -rnP '[^\x00-\x7F]' crates/provedex-agent/ docs/integration/sidecar.md`.
- AI slop adjective audit.
- No new top-level dir.
- No `unsafe` in core (and none in agent either).
- No `unwrap` outside tests in production code.
- New crate added to workspace members; check root Cargo.toml.

- [ ] **Step 3: Run full CI gate**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
cargo deny check
```

Expected: all green.

- [ ] **Step 4: Commit any review fixes**

If any rustdoc or polish needed:

```bash
git add -A
git commit -m "docs(agent): rustdoc on public API"
```

---

### Task 9: Open PR

**Files:** none modified.

- [ ] **Step 1: Push final state**

```bash
git push
```

- [ ] **Step 2: Open PR**

PR title: `feat(agent): provedex-agent sidecar phase 1 (HTTP signing daemon)`. Body in voice-aditya semi-formal register, summary + what changed + test plan + ADR ref + closes #11 (or "refs #11" if more phases pending).

- [ ] **Step 3: Wait for CI green**

```bash
gh run watch --exit-status
```

---

### Task 10: Confidence check + merge

- [ ] **Step 1: Read PR diff one more time as if you wrote nothing**

```bash
gh pr diff
```

- [ ] **Step 2: Confidence check (95% bar)**

Self-questions:
- Does the agent reject a non-loopback bind without the explicit flag? (Verify the bail in main.rs runs before binding.)
- Does the agent fail loud if the ledger cannot be opened or the key cannot be loaded? (LedgerSession::open returns Result; main propagates with context.)
- Are all routes covered by tests? (healthz, sign, sign-chain, verify, sign-rejects-bad-event = 5 tests.)
- Are the public API items (AgentState, build_router, SignRequest, Health) all rustdoc-documented?
- Does the integration doc match the actual API surface? (curl examples, language clients use the same path + body shape.)

If any "no" or "uncertain", do not merge; fix first.

- [ ] **Step 3: Merge**

```bash
gh pr merge --squash --delete-branch
git checkout main
git pull
```

- [ ] **Step 4: Mark issue closure**

```bash
gh issue close 11 --comment "Sidecar phase 1 shipped in PR #N. Phase 2 (hardening: rate limits, structured logs, body size cap, graceful shutdown) and phase 3 (release artifacts: GitHub Releases, container image, systemd unit) are tracked as new issues."
```

Open follow-up issues for phase 2 + 3 after merge.

---

## Self-review (writer's pass on this plan)

Spec coverage:
- Sidecar HTTP signing daemon: covered (Tasks 4-6).
- /v1/sign + /v1/verify + /v1/healthz: covered (Tasks 4-5).
- Loopback-only default + insecure flag: covered (Task 6).
- Env var overrides: covered (Task 6).
- Reuses LedgerSession: covered (Task 4 + 5).
- Integration doc with per-language clients: covered (Task 7).
- Self-review with 95% confidence bar: covered (Task 8 + 10).
- PR with code review before merge: covered (Tasks 9 + 10).
- CI gate + supply chain: covered (Task 8 step 3).
- Closes / refs #11: covered (Task 10 step 4).

Placeholder scan: none of the patterns from the skill's "No Placeholders" list appear.

Type consistency: `AgentState { session: LedgerSession }`, `build_router(Arc<AgentState>) -> Router`, `SignRequest { event: AgentEvent }`, sign returns `Json<SignedEvent>`, verify returns `Json<ChainReport>`, healthz returns `Json<Health>`. Tests use the same shapes.

No gaps found. Plan ready for execution.
