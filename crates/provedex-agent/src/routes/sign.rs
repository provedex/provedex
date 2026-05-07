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
