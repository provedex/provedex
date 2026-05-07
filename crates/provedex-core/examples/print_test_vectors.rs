//! Emits canonical-JSON bytes + SHA-256 + signature test vectors for the
//! normative specs. Run via `cargo run -p provedex-core --example
//! print_test_vectors`. Output is pasted verbatim into
//! `docs/spec/canonical-json.md`, `docs/spec/event-schema-v1.md`, and
//! `docs/spec/signature-scheme.md`. Signature vectors use a deterministic
//! 32-byte seed so they are reproducible across runs.

use std::io::Write;

use provedex_core::{
    canonical_json, compute_self_hash, AgentEvent, SigningKeypair, GENESIS_PARENT_HASH,
};
use serde_json::json;
use sha2::{Digest, Sha256};

const FIXED_SEED: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

fn show(label: &str, value: &serde_json::Value) {
    let bytes = canonical_json(value);
    let hex_sha = hex::encode(Sha256::digest(&bytes));
    println!("--- {label} ---");
    println!("input:  {}", serde_json::to_string(value).unwrap());
    println!("bytes:  {}", String::from_utf8_lossy(&bytes));
    println!("len:    {}", bytes.len());
    println!("sha256: {hex_sha}");
    println!();
}

fn show_event(label: &str, event: &AgentEvent) {
    let v = serde_json::to_value(event).unwrap();
    let bytes = canonical_json(&v);
    let hex_sha = hex::encode(Sha256::digest(&bytes));
    println!("--- {label} ---");
    println!("input:  {}", serde_json::to_string(&v).unwrap());
    println!("bytes:  {}", String::from_utf8_lossy(&bytes));
    println!("len:    {}", bytes.len());
    println!("sha256: {hex_sha}");
    println!();
}

fn fixed_keypair() -> SigningKeypair {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ed25519.key");
    let mut f = std::fs::File::create(&path).expect("create key file");
    f.write_all(&FIXED_SEED).expect("write seed");
    f.sync_all().expect("sync");
    SigningKeypair::load(&path).expect("load fixed-seed key")
}

fn show_signed(label: &str, seq: u64, timestamp_nanos: u64, event: AgentEvent, parent_hash: &str) {
    // Manually compute self_hash + signature so the example does not depend
    // on now_nanos() and stays reproducible across runs.
    let kp = fixed_keypair();
    let hash_bytes = compute_self_hash(seq, timestamp_nanos, &event, parent_hash).unwrap();
    let signature = kp.sign(&hash_bytes);
    println!("--- {label} ---");
    println!("seq:         {seq}");
    println!("timestamp:   {timestamp_nanos}");
    println!("parent_hash: {parent_hash}");
    println!(
        "self_hash:   {} ({} bytes raw)",
        hex::encode(hash_bytes),
        hash_bytes.len()
    );
    println!("pubkey:      {}", kp.pubkey_hex());
    println!("signature:   {}", hex::encode(signature.to_bytes()));
    println!();
}

fn main() {
    println!("== canonical-json test vectors ==\n");

    show("object key sort", &json!({"b": 1, "a": 2, "c": [3, 2, 1]}));
    show(
        "control char escape",
        &json!({"k": "line1\nline2\t\"end\""}),
    );
    show(
        "nested",
        &json!({
            "session_id": "abc",
            "events": [{"type": "x", "n": 1}, {"type": "y", "n": 2}]
        }),
    );
    show(
        "empty containers",
        &json!({"empty_arr": [], "empty_obj": {}, "null_field": null}),
    );
    show("unicode in string", &json!({"name": "Aditya"}));
    show(
        "number ranges",
        &json!({"u": 18446744073709551615u64, "z": 0}),
    );

    println!("\n== event-schema test vectors ==\n");

    show_event(
        "SessionStarted",
        &AgentEvent::SessionStarted {
            agent_id: "agent-1".into(),
            model_id: "llama3.2:3b".into(),
            session_id: "session-demo".into(),
        },
    );
    show_event(
        "UtteranceCaptured",
        &AgentEvent::UtteranceCaptured {
            audio_sha256: "9f3b2a1c0d4e5f6789abcdef0123456789abcdef0123456789abcdef01234567".into(),
            transcript: "patient reports chest pain".into(),
            lang: "en".into(),
            duration_ms: 2400,
        },
    );
    show_event(
        "ModelInvoked",
        &AgentEvent::ModelInvoked {
            model_id: "gpt-4o".into(),
            prompt_sha256: "aaaa1111bbbb2222cccc3333dddd4444eeee5555ffff6666aaaa1111bbbb2222"
                .into(),
            response_sha256: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .into(),
            prompt_tokens: 482,
            response_tokens: 71,
        },
    );
    show_event(
        "SessionEnded",
        &AgentEvent::SessionEnded {
            reason: "user_hangup".into(),
            summary_sha256: "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
                .into(),
        },
    );

    println!("\n== signature-scheme test vectors (fixed seed) ==\n");
    println!("seed: {} (hex)", hex::encode(FIXED_SEED));
    println!("pubkey: {}\n", fixed_keypair().pubkey_hex());

    show_signed(
        "Signed SessionStarted at seq 0",
        0,
        1_700_000_000_000_000_000,
        AgentEvent::SessionStarted {
            agent_id: "agent-1".into(),
            model_id: "llama3.2:3b".into(),
            session_id: "session-demo".into(),
        },
        GENESIS_PARENT_HASH,
    );

    show_signed(
        "Signed ModelInvoked at seq 1 (parent = previous self_hash placeholder)",
        1,
        1_700_000_000_500_000_000,
        AgentEvent::ModelInvoked {
            model_id: "gpt-4o".into(),
            prompt_sha256: "aaaa1111bbbb2222cccc3333dddd4444eeee5555ffff6666aaaa1111bbbb2222"
                .into(),
            response_sha256: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .into(),
            prompt_tokens: 482,
            response_tokens: 71,
        },
        "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
    );
}
