use serde::{Deserialize, Serialize};

use crate::chain::{verify_chain, ChainReport};
use crate::signed::SignedEvent;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportBundle {
    pub schema_version: u32,
    pub generated_at: String,
    pub chain_report: ChainReport,
    pub events: Vec<SignedEvent>,
}

impl ExportBundle {
    pub fn from_events(events: Vec<SignedEvent>) -> Self {
        let chain_report = verify_chain(&events);
        Self {
            schema_version: 1,
            generated_at: chrono::Utc::now().to_rfc3339(),
            chain_report,
            events,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::ChainStatus;
    use crate::event::AgentEvent;
    use crate::keys::SigningKeypair;
    use crate::signed::GENESIS_PARENT_HASH;

    #[test]
    fn bundle_has_chain_report() {
        let kp = SigningKeypair::generate();
        let evt = AgentEvent::SessionStarted {
            agent_id: "a".into(),
            model_id: "m".into(),
            session_id: "s".into(),
        };
        let s = SignedEvent::seal(0, evt, GENESIS_PARENT_HASH, &kp).unwrap();
        let b = ExportBundle::from_events(vec![s]);
        assert_eq!(b.chain_report.status, ChainStatus::Valid);
        assert_eq!(b.events.len(), 1);
    }
}
