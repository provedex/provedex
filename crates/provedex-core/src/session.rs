pub struct LedgerSession;

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
