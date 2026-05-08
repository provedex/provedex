use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::state::AgentState;

#[derive(Serialize, utoipa::ToSchema)]
pub struct Health {
    /// "ok" when the ledger is writable, "degraded" otherwise.
    #[schema(example = "ok")]
    pub status: &'static str,
    /// UUID assigned to this agent process at startup.
    pub session_id: String,
    /// Hex-encoded Ed25519 public key of the signing keypair.
    pub pubkey: String,
    /// Whether the ledger directory passed a non-destructive write probe.
    pub ledger_writable: bool,
    /// Absolute path of the ledger file on the agent host.
    pub ledger_path: String,
}

#[utoipa::path(
    get,
    path = "/v1/healthz",
    tag = "agent",
    responses(
        (status = 200, description = "Agent is healthy and ledger is writable", body = Health),
        (status = 503, description = "Agent is degraded; ledger is not writable", body = Health),
    ),
)]
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
