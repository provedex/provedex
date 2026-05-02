//! Minimal sign-then-verify flow for the Provedex audit ledger.
//!
//! Run with: `cargo run -p provedex-core --example basic_signing`.

use provedex_core::{
    verify_chain, AgentEvent, ChainStatus, SignedEvent, SigningKeypair, GENESIS_PARENT_HASH,
};

fn main() {
    let kp = SigningKeypair::generate();
    println!("pubkey: {}", kp.pubkey_hex());

    let mut parent = GENESIS_PARENT_HASH.to_string();
    let mut events = Vec::new();
    for i in 0..3 {
        let evt = AgentEvent::SessionStarted {
            agent_id: format!("agent-{i}"),
            model_id: "llama3.2:3b".into(),
            session_id: "session-demo".into(),
        };
        let signed = SignedEvent::seal(i, evt, &parent, &kp).expect("seal");
        parent = signed.self_hash.clone();
        events.push(signed);
    }

    let report = verify_chain(&events);
    assert_eq!(report.status, ChainStatus::Valid);
    println!("status: {:?}", report.status);
    println!("events: {}", report.event_count);
    println!("root:   {}", report.root_hash);
}
