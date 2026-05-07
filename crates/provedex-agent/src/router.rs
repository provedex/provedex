use std::sync::Arc;

use axum::Router;

use crate::state::AgentState;

pub fn build_router(_state: Arc<AgentState>) -> Router {
    Router::new()
}
