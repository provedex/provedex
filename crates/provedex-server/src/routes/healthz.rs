use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
    pub session_id: String,
    pub pubkey: String,
}

pub async fn healthz(State(state): State<Arc<AppState>>) -> Json<Health> {
    Json(Health {
        status: "ok",
        session_id: state.session_id.clone(),
        pubkey: state.keypair.pubkey_hex(),
    })
}
