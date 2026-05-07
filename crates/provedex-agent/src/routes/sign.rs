use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
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
    let signed = state
        .session
        .seal_and_append(req.event)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(signed))
}
