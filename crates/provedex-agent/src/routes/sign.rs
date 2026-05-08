use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use provedex_core::{AgentEvent, SignedEvent};
use serde::Deserialize;

use crate::state::AgentState;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SignRequest {
    pub event: AgentEvent,
}

#[utoipa::path(
    post,
    path = "/v1/sign",
    tag = "agent",
    request_body = SignRequest,
    responses(
        (status = 200, description = "Event sealed and appended to the ledger", body = SignedEvent),
        (status = 400, description = "Malformed JSON or unknown event variant"),
        (status = 413, description = "Request body exceeded the configured size cap"),
        (status = 429, description = "Per-IP rate limit exceeded"),
        (status = 500, description = "Ledger write failed"),
    ),
)]
pub async fn sign(
    State(state): State<Arc<AgentState>>,
    body: Result<Json<SignRequest>, JsonRejection>,
) -> Result<Json<SignedEvent>, (StatusCode, String)> {
    let Json(req) = body.map_err(|e| {
        // Preserve the rejection's intended status. Body-limit failures must
        // surface as 413 rather than be swallowed as 400.
        let status = match &e {
            JsonRejection::BytesRejection(_) => StatusCode::PAYLOAD_TOO_LARGE,
            _ => StatusCode::BAD_REQUEST,
        };
        (status, e.to_string())
    })?;
    // seal_and_append calls fsync_data plus a SHA-256 + Ed25519 sign. All three
    // are blocking work; running them on a tokio runtime thread starves other
    // in-flight requests. spawn_blocking moves the work to the blocking pool.
    let state_for_blocking = state.clone();
    let signed = tokio::task::spawn_blocking(move || {
        state_for_blocking.session.seal_and_append(req.event)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join error: {e}")))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(signed))
}
