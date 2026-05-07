use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::limit::RequestBodyLimitLayer;

use crate::routes;
use crate::state::AgentState;

/// Tunables for token-bucket rate limiting on `/v1/sign`. None = unlimited.
pub struct RateLimitConfig {
    pub per_sec: u64,
    pub burst: u32,
}

/// Default body cap for `/v1/sign`. Signed event payloads are typically a few
/// hundred bytes; 32 KiB allows for unusual variants without inviting abuse.
pub const DEFAULT_MAX_BODY_BYTES: usize = 32 * 1024;

/// Build the Axum router with default limits. Convenience wrapper for tests
/// and the binary.
pub fn build_router(state: Arc<AgentState>) -> Router {
    build_router_with_limits(state, DEFAULT_MAX_BODY_BYTES, None)
}

/// Build the router with explicit limits. Tests dial these to exercise edge
/// cases; main.rs reads the values from CLI flags.
pub fn build_router_with_limits(
    state: Arc<AgentState>,
    max_body_bytes: usize,
    rate_limit: Option<RateLimitConfig>,
) -> Router {
    let mut sign_route = post(routes::sign::sign).layer(RequestBodyLimitLayer::new(max_body_bytes));

    if let Some(rl) = rate_limit {
        use std::sync::Arc;
        use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
        let conf = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(rl.per_sec)
                .burst_size(rl.burst)
                .finish()
                .expect("invalid governor config"),
        );
        sign_route = sign_route.layer(GovernorLayer { config: conf });
    }

    Router::new()
        .route("/v1/healthz", get(routes::healthz::healthz))
        .route("/v1/sign", sign_route)
        .route("/v1/verify", post(routes::verify::verify))
        .with_state(state)
}
