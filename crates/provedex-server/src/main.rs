use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::routing::{get, post};
use axum::Router;
use clap::Parser;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod state;
mod voice;

mod routes {
    pub mod chat;
    pub mod events;
    pub mod export;
    pub mod healthz;
    pub mod verify;

    #[cfg(feature = "demo")]
    pub mod tamper;
}

use state::AppState;

#[derive(Parser, Debug)]
#[command(name = "provedex-server", version)]
struct Args {
    /// Port to bind on. Frontend and API share the same port.
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Override the static frontend directory.
    #[arg(long)]
    frontend_dir: Option<PathBuf>,

    /// Override the NDJSON ledger path.
    #[arg(long)]
    ledger: Option<PathBuf>,

    /// Override the Ed25519 key path.
    #[arg(long)]
    key: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,axum=info")),
        )
        .compact()
        .init();

    let args = Args::parse();
    let state = Arc::new(AppState::initialize(args.ledger.clone(), args.key.clone())?);

    state.seal_and_append(provedex_core::AgentEvent::SessionStarted {
        agent_id: "provedex-voice-scribe".into(),
        model_id: "llama3.2:3b".into(),
        session_id: state.session_id.clone(),
    })?;

    let frontend_dir = match args.frontend_dir {
        Some(p) => p,
        None => find_frontend_dir().context("locating frontend directory")?,
    };
    tracing::info!(path = %frontend_dir.display(), "serving frontend");

    let api = Router::new()
        .route("/healthz", get(routes::healthz::healthz))
        .route("/chat", post(routes::chat::chat))
        .route("/events", get(routes::events::events))
        .route("/verify", post(routes::verify::verify))
        .route("/export", post(routes::export::export));

    #[cfg(feature = "demo")]
    let api = api.route("/tamper-test", post(routes::tamper::tamper_test));

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new(frontend_dir))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    tracing::info!(%addr, "provedex-server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn find_frontend_dir() -> Result<PathBuf> {
    let cargo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        cargo_root.join("../../frontend"),
        PathBuf::from("frontend"),
        PathBuf::from("./frontend"),
    ];
    for p in candidates.iter() {
        if p.is_dir() {
            return Ok(std::fs::canonicalize(p)?);
        }
    }
    anyhow::bail!("frontend directory not found")
}
