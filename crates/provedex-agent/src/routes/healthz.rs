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
