use std::path::PathBuf;

use anyhow::{Context, Result};
use provedex_core::{
    default_key_path, default_ledger_path, AgentEvent, Ledger, LedgerSession, SignedEvent,
    SigningKeypair,
};
use tokio::sync::broadcast;

const EVENT_CHANNEL_CAPACITY: usize = 128;

/// Server-wide state. Wraps a `LedgerSession` (the shared signing primitive)
/// and adds the SSE broadcast channel that the demo UI subscribes to.
pub struct AppState {
    pub session: LedgerSession,
    pub broadcast: broadcast::Sender<SignedEvent>,
}

impl AppState {
    pub fn initialize(
        ledger_override: Option<PathBuf>,
        key_override: Option<PathBuf>,
    ) -> Result<Self> {
        let ledger_path = ledger_override.unwrap_or(default_ledger_path()?);
        let key_path = key_override.unwrap_or(default_key_path()?);

        let keypair = SigningKeypair::load_or_create(&key_path)
            .with_context(|| format!("loading or creating keypair at {}", key_path.display()))?;
        let ledger = Ledger::open(&ledger_path)
            .with_context(|| format!("opening ledger at {}", ledger_path.display()))?;
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = LedgerSession::new(keypair, ledger, session_id);

        let (tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);

        Ok(Self {
            session,
            broadcast: tx,
        })
    }

    /// Server-side wrapper that seals + broadcasts the event so SSE
    /// subscribers see it without the routes needing to know about both
    /// concerns.
    pub fn seal_and_append(&self, event: AgentEvent) -> Result<SignedEvent> {
        let signed = self.session.seal_and_append(event)?;
        let _ = self.broadcast.send(signed.clone());
        Ok(signed)
    }

    pub fn session_id(&self) -> &str {
        self.session.session_id()
    }

    pub fn pubkey_hex(&self) -> String {
        self.session.pubkey_hex()
    }

    pub fn ledger(&self) -> &Ledger {
        self.session.ledger()
    }
}
