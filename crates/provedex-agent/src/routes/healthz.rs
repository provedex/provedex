use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::state::AgentState;

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
    let code = if writable {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(body))
}
