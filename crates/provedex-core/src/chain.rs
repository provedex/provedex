use serde::{Deserialize, Serialize};

use crate::signed::{SignedEvent, GENESIS_PARENT_HASH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChainStatus {
    Valid,
    Broken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ChainReport {
    pub status: ChainStatus,
    pub event_count: u64,
    pub broken_at_seq: Option<u64>,
    pub broken_reason: Option<String>,
    pub root_hash: String,
}

/// Walk the ledger in order. For every event:
/// 1. recompute self_hash and verify the signature,
/// 2. confirm parent_hash links to the previous self_hash (or to the
///    genesis sentinel for seq 0),
/// 3. confirm seq monotonically increments by one.
///
/// Returns at the first failure with `broken_at_seq` set.
pub fn verify_chain(events: &[SignedEvent]) -> ChainReport {
    if events.is_empty() {
        return ChainReport {
            status: ChainStatus::Valid,
            event_count: 0,
            broken_at_seq: None,
            broken_reason: None,
            root_hash: GENESIS_PARENT_HASH.to_string(),
        };
    }

    let mut prev_hash = GENESIS_PARENT_HASH.to_string();

    for (expected_seq, evt) in (0_u64..).zip(events.iter()) {
        if evt.seq != expected_seq {
            return ChainReport {
                status: ChainStatus::Broken,
                event_count: events.len() as u64,
                broken_at_seq: Some(evt.seq),
                broken_reason: Some(format!("expected seq {expected_seq}, got {}", evt.seq)),
                root_hash: prev_hash,
            };
        }
        if evt.parent_hash != prev_hash {
            return ChainReport {
                status: ChainStatus::Broken,
                event_count: events.len() as u64,
                broken_at_seq: Some(evt.seq),
                broken_reason: Some("parent_hash does not match previous self_hash".into()),
                root_hash: prev_hash,
            };
        }
        if let Err(e) = evt.verify_self() {
            return ChainReport {
                status: ChainStatus::Broken,
                event_count: events.len() as u64,
                broken_at_seq: Some(evt.seq),
                broken_reason: Some(format!("event verification failed: {e}")),
                root_hash: prev_hash,
            };
        }
        prev_hash = evt.self_hash.clone();
    }

    ChainReport {
        status: ChainStatus::Valid,
        event_count: events.len() as u64,
        broken_at_seq: None,
        broken_reason: None,
        root_hash: prev_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::AgentEvent;
    use crate::keys::SigningKeypair;

    fn make_event(i: u64) -> AgentEvent {
        AgentEvent::ModelInvoked {
            model_id: "llama3.2:3b".into(),
            prompt_sha256: format!("p{i:064}"),
            response_sha256: format!("r{i:064}"),
            prompt_tokens: 10 + i as u32,
            response_tokens: 20 + i as u32,
        }
    }

    fn build_chain(kp: &SigningKeypair, n: u64) -> Vec<SignedEvent> {
        let mut events = Vec::new();
        let mut parent = GENESIS_PARENT_HASH.to_string();
        for i in 0..n {
            let s = SignedEvent::seal(i, make_event(i), &parent, kp).unwrap();
            parent = s.self_hash.clone();
            events.push(s);
        }
        events
    }

    #[test]
    fn empty_chain_is_valid() {
        let r = verify_chain(&[]);
        assert_eq!(r.status, ChainStatus::Valid);
        assert_eq!(r.event_count, 0);
    }

    #[test]
    fn hundred_event_chain_verifies() {
        let kp = SigningKeypair::generate();
        let events = build_chain(&kp, 100);
        let r = verify_chain(&events);
        assert_eq!(r.status, ChainStatus::Valid);
        assert_eq!(r.event_count, 100);
        assert_eq!(r.root_hash, events[99].self_hash);
    }

    #[test]
    fn tampered_middle_event_breaks_chain() {
        let kp = SigningKeypair::generate();
        let mut events = build_chain(&kp, 10);
        if let AgentEvent::ModelInvoked {
            ref mut prompt_tokens,
            ..
        } = events[5].event
        {
            *prompt_tokens = 9999;
        }
        let r = verify_chain(&events);
        assert_eq!(r.status, ChainStatus::Broken);
        assert_eq!(r.broken_at_seq, Some(5));
    }

    #[test]
    fn tampered_signature_breaks_chain() {
        let kp = SigningKeypair::generate();
        let mut events = build_chain(&kp, 5);
        let mut sig_bytes = hex::decode(&events[2].signature).unwrap();
        sig_bytes[0] ^= 0xff;
        events[2].signature = hex::encode(sig_bytes);
        let r = verify_chain(&events);
        assert_eq!(r.status, ChainStatus::Broken);
        assert_eq!(r.broken_at_seq, Some(2));
    }

    #[test]
    fn deleted_event_breaks_chain_via_seq_gap() {
        let kp = SigningKeypair::generate();
        let mut events = build_chain(&kp, 5);
        events.remove(2);
        let r = verify_chain(&events);
        assert_eq!(r.status, ChainStatus::Broken);
        assert_eq!(r.broken_at_seq, Some(3));
    }
}
