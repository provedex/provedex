use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use provedex_core::ExportBundle;

use crate::state::AppState;

pub async fn export(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let events = match state.ledger.read_all() {
        Ok(e) => e,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let bundle = ExportBundle::from_events(events);
    let body = match serde_json::to_vec_pretty(&bundle) {
        Ok(b) => b,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let filename = format!("provedex-export-{}.json", state.session_id);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        body,
    )
        .into_response()
}
