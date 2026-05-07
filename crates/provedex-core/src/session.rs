use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use thiserror::Error;

use crate::event::AgentEvent;
use crate::keys::SigningKeypair;
use crate::ledger::{Ledger, LedgerError};
use crate::signed::{SignedError, SignedEvent, GENESIS_PARENT_HASH};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("ledger: {0}")]
    Ledger(#[from] LedgerError),
    #[error("signed: {0}")]
    Signed(#[from] SignedError),
}

/// Owns the seq counter, parent_hash mutex, signing key, and ledger handle for
/// a single signing session. Both `provedex-server` and the upcoming
/// `provedex-agent` sidecar build on this primitive so the chain invariants
/// live in one place.
///
/// ```
/// use provedex_core::{Ledger, LedgerSession, SigningKeypair};
/// let dir = tempfile::tempdir().unwrap();
/// let kp = SigningKeypair::generate();
/// let ledger = Ledger::open(dir.path().join("ledger.ndjson")).unwrap();
/// let s = LedgerSession::new(kp, ledger, "demo".into());
/// assert_eq!(s.session_id(), "demo");
/// ```
#[derive(Debug)]
pub struct LedgerSession {
    session_id: String,
    keypair: SigningKeypair,
    ledger: Ledger,
    seq: AtomicU64,
    parent_hash: Mutex<String>,
}

impl LedgerSession {
    /// Construct a session, resuming from any pre-existing events on disk.
    /// Reads the ledger once at construction to recover the next seq and the
    /// most recent self_hash for chaining.
    pub fn new(keypair: SigningKeypair, ledger: Ledger, session_id: String) -> Self {
        let existing = ledger.read_all().unwrap_or_default();
        let (seq, parent_hash) = match existing.last() {
            Some(last) => (last.seq + 1, last.self_hash.clone()),
            None => (0, GENESIS_PARENT_HASH.to_string()),
        };
        Self {
            session_id,
            keypair,
            ledger,
            seq: AtomicU64::new(seq),
            parent_hash: Mutex::new(parent_hash),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn pubkey_hex(&self) -> String {
        self.keypair.pubkey_hex()
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Atomically allocate the next seq, sign the event against the current
    /// parent hash, append to the ledger, and update the parent hash. This is
    /// the only sanctioned event emitter for live runs.
    ///
    /// ```
    /// use provedex_core::{AgentEvent, Ledger, LedgerSession, SigningKeypair};
    /// let dir = tempfile::tempdir().unwrap();
    /// let kp = SigningKeypair::generate();
    /// let ledger = Ledger::open(dir.path().join("ledger.ndjson")).unwrap();
    /// let s = LedgerSession::new(kp, ledger, "demo".into());
    /// let signed = s
    ///     .seal_and_append(AgentEvent::SessionStarted {
    ///         agent_id: "a".into(),
    ///         model_id: "m".into(),
    ///         session_id: "s".into(),
    ///     })
    ///     .unwrap();
    /// assert_eq!(signed.seq, 0);
    /// ```
    pub fn seal_and_append(&self, event: AgentEvent) -> Result<SignedEvent, SessionError> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        let mut parent = self.parent_hash.lock().expect("parent_hash mutex poisoned");
        let signed = SignedEvent::seal(seq, event, &parent, &self.keypair)?;
        self.ledger.append(&signed)?;
        *parent = signed.self_hash.clone();
        Ok(signed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::AgentEvent;
    use crate::keys::SigningKeypair;
    use tempfile::tempdir;

    fn fixture(dir: &std::path::Path) -> LedgerSession {
        let kp = SigningKeypair::generate();
        let ledger = crate::ledger::Ledger::open(dir.join("ledger.ndjson")).unwrap();
        LedgerSession::new(kp, ledger, "test-session".into())
    }

    fn evt(i: u64) -> AgentEvent {
        AgentEvent::SessionStarted {
            agent_id: format!("a{i}"),
            model_id: "m".into(),
            session_id: "s".into(),
        }
    }

    #[test]
    fn first_event_uses_genesis_parent() {
        let dir = tempdir().unwrap();
        let s = fixture(dir.path());
        let signed = s.seal_and_append(evt(0)).unwrap();
        assert_eq!(signed.seq, 0);
        assert_eq!(signed.parent_hash, crate::signed::GENESIS_PARENT_HASH);
    }

    #[test]
    fn subsequent_events_chain_to_previous() {
        let dir = tempdir().unwrap();
        let s = fixture(dir.path());
        let a = s.seal_and_append(evt(0)).unwrap();
        let b = s.seal_and_append(evt(1)).unwrap();
        assert_eq!(b.seq, 1);
        assert_eq!(b.parent_hash, a.self_hash);
    }

    #[test]
    fn ledger_picks_up_pre_existing_events_on_open() {
        let dir = tempdir().unwrap();
        {
            let s = fixture(dir.path());
            s.seal_and_append(evt(0)).unwrap();
            s.seal_and_append(evt(1)).unwrap();
        }
        let kp = SigningKeypair::generate();
        let ledger = crate::ledger::Ledger::open(dir.path().join("ledger.ndjson")).unwrap();
        let s = LedgerSession::new(kp, ledger, "resume".into());
        let c = s.seal_and_append(evt(2)).unwrap();
        assert_eq!(c.seq, 2);
        let report = crate::chain::verify_chain(&s.ledger().read_all().unwrap());
        assert_eq!(report.event_count, 3);
    }

    #[test]
    fn pubkey_hex_exposes_signer_identity() {
        let dir = tempdir().unwrap();
        let s = fixture(dir.path());
        let pk = s.pubkey_hex();
        assert_eq!(pk.len(), 64);
        let signed = s.seal_and_append(evt(0)).unwrap();
        assert_eq!(signed.signer_pubkey, pk);
    }
}
