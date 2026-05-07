use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use provedex_agent::router::{build_router, build_router_with_limits};
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
    let app1 = build_router(state.clone());
    let app2 = build_router(state);

    let post = |app: axum::Router, agent_id: &str| {
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

    let resp1 = post(app1, "first").await;
    let v1 = body_json(resp1.into_body()).await;
    let resp2 = post(app2, "second").await;
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
async fn sign_returns_413_on_oversize_body() {
    let (state, _dir) = fixture().await;
    let app = build_router_with_limits(state, 1024, None);
    let big = "A".repeat(2048);
    let body = serde_json::to_vec(&json!({
        "event": {
            "type": "SessionStarted",
            "payload": { "agent_id": big, "model_id": "m", "session_id": "s" }
        }
    }))
    .unwrap();
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
    let req = Request::builder()
        .method("GET")
        .uri("/v1/healthz")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp.into_body()).await;
    assert_eq!(v["ledger_writable"], true);
    assert!(v["ledger_path"].as_str().unwrap().contains("ledger.ndjson"));
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
