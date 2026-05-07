use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::routes;
use crate::state::AgentState;

/// Build the Axum router for the agent. Exposed so integration tests can
/// drive the router directly via `tower::ServiceExt::oneshot` without
/// spawning a TCP listener.
pub fn build_router(state: Arc<AgentState>) -> Router {
    Router::new()
        .route("/v1/healthz", get(routes::healthz::healthz))
        .with_state(state)
}
