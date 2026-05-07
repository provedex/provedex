use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use provedex_agent::router::build_router;
use provedex_agent::state::AgentState;
use provedex_core::{default_key_path, default_ledger_path, Ledger, LedgerSession, SigningKeypair};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "provedex-agent",
    version,
    about = "Provedex sidecar HTTP signing daemon"
)]
struct Args {
    /// Listen address. Default 127.0.0.1:8765. Non-loopback addresses require
    /// --insecure-allow-public.
    #[arg(long, default_value = "127.0.0.1:8765", env = "PROVEDEX_AGENT_LISTEN")]
    listen: SocketAddr,

    /// Override the NDJSON ledger path. Default ~/.provedex/ledger.ndjson.
    #[arg(long, env = "PROVEDEX_LEDGER")]
    ledger: Option<PathBuf>,

    /// Override the Ed25519 key path. Default ~/.provedex/keys/ed25519.key.
    #[arg(long, env = "PROVEDEX_KEY")]
    key: Option<PathBuf>,

    /// Allow binding to a non-loopback address. Off by default; the agent has
    /// no auth and must not face the public internet without TLS + auth in
    /// front of it.
    #[arg(long, default_value_t = false)]
    insecure_allow_public: bool,
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

    if !args.listen.ip().is_loopback() && !args.insecure_allow_public {
        anyhow::bail!(
            "refusing to bind {} (non-loopback) without --insecure-allow-public; \
             this daemon has no auth and must not face the public internet",
            args.listen
        );
    }

    let ledger_path = args.ledger.unwrap_or(default_ledger_path()?);
    let key_path = args.key.unwrap_or(default_key_path()?);

    let keypair = SigningKeypair::load_or_create(&key_path)
        .with_context(|| format!("loading or creating keypair at {}", key_path.display()))?;
    let ledger = Ledger::open(&ledger_path)
        .with_context(|| format!("opening ledger at {}", ledger_path.display()))?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let session =
        LedgerSession::open(keypair, ledger, session_id).context("opening ledger session")?;
    let state = Arc::new(AgentState::new(session));

    let app = build_router(state);

    tracing::info!(addr = %args.listen, "provedex-agent listening");
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
