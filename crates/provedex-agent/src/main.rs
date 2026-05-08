use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use provedex_agent::openapi::ApiDoc;
use provedex_agent::router::{build_router_with_limits, RateLimitConfig, DEFAULT_MAX_BODY_BYTES};
use provedex_agent::state::AgentState;
use provedex_core::{default_key_path, default_ledger_path, Ledger, LedgerSession, SigningKeypair};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;

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

    /// Max request body size on /v1/sign in bytes. Default 32 KiB.
    #[arg(
        long,
        default_value_t = DEFAULT_MAX_BODY_BYTES,
        env = "PROVEDEX_AGENT_MAX_BODY_BYTES"
    )]
    max_body_bytes: usize,

    /// Per-IP token-bucket rate limit on /v1/sign (requests per second).
    /// Operator endpoints (/v1/healthz, /v1/verify) are not rate-limited.
    #[arg(long, default_value_t = 100)]
    rate_limit_per_sec: u64,

    /// Per-IP rate-limit burst on /v1/sign.
    #[arg(long, default_value_t = 200)]
    rate_limit_burst: u32,

    /// Disable rate limiting entirely. Useful for trusted single-tenant hosts.
    #[arg(long, default_value_t = false)]
    rate_limit_off: bool,

    /// Grace period in seconds for in-flight requests to drain on SIGTERM.
    #[arg(long, default_value_t = 30)]
    shutdown_grace_secs: u64,

    /// Print the OpenAPI 3 spec for the HTTP API as YAML to stdout and exit.
    /// Used to regenerate the committed docs/spec/openapi.yaml.
    #[arg(long, default_value_t = false)]
    print_openapi: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.print_openapi {
        let yaml = ApiDoc::openapi()
            .to_yaml()
            .context("serializing OpenAPI spec to YAML")?;
        print!("{yaml}");
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,axum=info")),
        )
        .compact()
        .init();

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

    let rate_limit = if args.rate_limit_off {
        None
    } else {
        Some(RateLimitConfig {
            per_sec: args.rate_limit_per_sec,
            burst: args.rate_limit_burst,
        })
    };

    let app = build_router_with_limits(state, args.max_body_bytes, rate_limit).layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(
                DefaultOnResponse::new()
                    .level(Level::INFO)
                    .latency_unit(LatencyUnit::Millis),
            ),
    );

    tracing::info!(
        addr = %args.listen,
        max_body_bytes = args.max_body_bytes,
        rate_limit_off = args.rate_limit_off,
        rate_per_sec = args.rate_limit_per_sec,
        rate_burst = args.rate_limit_burst,
        shutdown_grace_secs = args.shutdown_grace_secs,
        "provedex-agent listening"
    );

    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    // ConnectInfo wiring required by tower_governor's PeerIpKeyExtractor so
    // per-IP rate limiting can identify the caller.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(args.shutdown_grace_secs))
    .await?;
    Ok(())
}

async fn shutdown_signal(grace_secs: u64) {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c().await.expect("ctrl_c handler");
    };

    #[cfg(unix)]
    let term = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("sigterm handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
    tracing::info!(grace_secs, "shutdown signal received, draining");
}
