use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use provedex_core::{verify_chain, ChainReport};

use crate::state::AgentState;

#[utoipa::path(
    post,
    path = "/v1/verify",
    tag = "agent",
    responses(
        (status = 200, description = "Ledger walked end to end; ChainReport carries the verdict", body = ChainReport),
        (status = 500, description = "Ledger read failed"),
    ),
)]
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
