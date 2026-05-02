use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use anyhow::{Context, Result};
use provedex_core::{
    default_key_path, default_ledger_path, AgentEvent, Ledger, SignedEvent, SigningKeypair,
    GENESIS_PARENT_HASH,
};
use tokio::sync::broadcast;

const EVENT_CHANNEL_CAPACITY: usize = 128;

/// Server-wide state. Sequence number, parent hash, and the broadcast channel
/// must move in lockstep; `seal_and_append` is the only sanctioned mutator.
pub struct AppState {
    pub session_id: String,
    pub keypair: SigningKeypair,
    pub ledger: Ledger,
    pub broadcast: broadcast::Sender<SignedEvent>,
    pub seq: AtomicU64,
    parent_hash: Mutex<String>,
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

        let existing = ledger.read_all()?;
        let (seq, parent_hash) = match existing.last() {
            Some(last) => (last.seq + 1, last.self_hash.clone()),
            None => (0, GENESIS_PARENT_HASH.to_string()),
        };

        let session_id = uuid::Uuid::new_v4().to_string();
        let (tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);

        Ok(Self {
            session_id,
            keypair,
            ledger,
            broadcast: tx,
            seq: AtomicU64::new(seq),
            parent_hash: Mutex::new(parent_hash),
        })
    }

    pub fn seal_and_append(&self, event: AgentEvent) -> Result<SignedEvent> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        let mut parent = self.parent_hash.lock().expect("parent_hash mutex poisoned");
        let signed = SignedEvent::seal(seq, event, &parent, &self.keypair)?;
        self.ledger.append(&signed)?;
        *parent = signed.self_hash.clone();
        drop(parent);
        let _ = self.broadcast.send(signed.clone());
        Ok(signed)
    }
}
