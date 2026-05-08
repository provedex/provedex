use provedex_core::{AgentEvent, ChainReport, ChainStatus, SignedEvent};
use utoipa::OpenApi;

use crate::routes::healthz::Health;
use crate::routes::sign::SignRequest;

/// Aggregates the agent's HTTP routes and component schemas into a single
/// OpenAPI 3 document. The CLI flag `--print-openapi` serializes this struct.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "provedex-agent",
        version = env!("CARGO_PKG_VERSION"),
        description = "Provedex sidecar HTTP signing daemon. Customer applications POST events to /v1/sign; the agent seals them with the local Ed25519 keypair and appends them to the NDJSON ledger. /v1/verify walks the ledger and reports chain integrity. /v1/healthz reports liveness plus a non-destructive ledger-writable probe.",
        license(name = "Apache-2.0", identifier = "Apache-2.0"),
    ),
    servers(
        (url = "http://127.0.0.1:8765", description = "Default loopback bind"),
    ),
    paths(
        crate::routes::healthz::healthz,
        crate::routes::sign::sign,
        crate::routes::verify::verify,
    ),
    components(schemas(
        Health,
        SignRequest,
        AgentEvent,
        SignedEvent,
        ChainReport,
        ChainStatus,
    )),
    tags(
        (name = "agent", description = "Sidecar HTTP signing daemon endpoints"),
    ),
)]
pub struct ApiDoc;
