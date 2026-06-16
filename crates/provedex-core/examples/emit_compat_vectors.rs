//! Emit byte-compat golden vectors as language-neutral JSON. Inputs use fixed
//! seq/timestamp values so output is deterministic across runs. Run with:
//!   cargo run -p provedex-core --example emit_compat_vectors
//! Writes tests/compat/vectors/{canonical_json,self_hash}.json relative to the
//! repo root.

use std::fs;
use std::path::Path;

use provedex_core::{
    canonical_json, compute_self_hash, AgentEvent, Ledger, LedgerSession, SigningKeypair,
};
use serde_json::{json, Value};

fn canonical_cases() -> Vec<Value> {
    let inputs = vec![
        ("sorted_keys", json!({"b": 1, "a": 2, "c": 3})),
        ("nested_array", json!({"c": [3, 2, 1], "a": 2})),
        ("control_chars", json!({"k": "line1\nline2\t\"end\""})),
        (
            "non_ascii_raw_utf8",
            json!({"k": "caf\u{e9} \u{2705} \u{1f512}"}),
        ),
        (
            "ints_and_bools",
            json!({"n": 42, "z": 0, "flag": true, "nil": null}),
        ),
        ("empty_object_and_array", json!({"o": {}, "a": []})),
    ];
    inputs
        .into_iter()
        .map(|(name, input)| {
            let bytes = canonical_json(&input);
            json!({
                "name": name,
                "input": input,
                "expected": String::from_utf8(bytes).expect("canonical json is valid utf-8"),
            })
        })
        .collect()
}

fn self_hash_cases() -> Vec<Value> {
    let events = vec![
        (
            "session_started",
            AgentEvent::SessionStarted {
                agent_id: "agent-1".into(),
                model_id: "llama3.2:3b".into(),
                session_id: "sess-1".into(),
            },
        ),
        (
            "model_invoked",
            AgentEvent::ModelInvoked {
                model_id: "gpt-4o".into(),
                prompt_sha256: "a".repeat(64),
                response_sha256: "b".repeat(64),
                prompt_tokens: 12,
                response_tokens: 34,
            },
        ),
        (
            "tool_called_non_ascii",
            AgentEvent::ToolCalled {
                tool_name: "search".into(),
                args_sha256: "c".repeat(64),
                args_redacted: json!({"q": "caf\u{e9} \u{2705}"}),
            },
        ),
    ];
    let parent = "0".repeat(64);
    events
        .into_iter()
        .enumerate()
        .map(|(i, (name, event))| {
            let seq = i as u64;
            let timestamp_nanos = 1_700_000_000_000_000_000u64 + seq;
            let hash = compute_self_hash(seq, timestamp_nanos, &event, &parent)
                .expect("hash a known-good event");
            json!({
                "name": name,
                "seq": seq,
                "timestamp_nanos": timestamp_nanos,
                "event": serde_json::to_value(&event).unwrap(),
                "parent_hash": parent,
                "self_hash": hex::encode(hash),
            })
        })
        .collect()
}

/// Emit a real Rust-signed ledger that the Python binding must verify as VALID.
/// This is the "Rust signs, Python verifies" direction of cross-verification.
/// A test-only fixed secret key keeps the signer identity stable across
/// regenerations (timestamps and signatures still vary run to run, which is
/// fine: the consumer only asserts the chain verifies, not specific bytes).
fn emit_signed_ledger(dir: &Path) {
    let path = dir.join("rust_signed_ledger.ndjson");
    // Fresh file each run; appending would duplicate a prior chain.
    let _ = fs::remove_file(&path);

    // SigningKeypair has no public from_bytes; round-trip a fixed secret
    // through a temp key file. [7u8; 32] is a fixture key, not a real identity.
    let key_path = dir.join(".fixture.key");
    fs::write(&key_path, [7u8; 32]).expect("write fixture key");
    let kp = SigningKeypair::load(&key_path).expect("load fixture key");
    fs::remove_file(&key_path).ok();

    let ledger = Ledger::open(&path).expect("open fixture ledger");
    let session =
        LedgerSession::open(kp, ledger, "rust-fixture".into()).expect("open fixture session");
    session
        .seal_and_append(AgentEvent::SessionStarted {
            agent_id: "rust-agent".into(),
            model_id: "gpt-4o".into(),
            session_id: "rust-fixture".into(),
        })
        .expect("seal 0");
    session
        .seal_and_append(AgentEvent::ModelInvoked {
            model_id: "gpt-4o".into(),
            prompt_sha256: "a".repeat(64),
            response_sha256: "b".repeat(64),
            prompt_tokens: 12,
            response_tokens: 34,
        })
        .expect("seal 1");
    session
        .seal_and_append(AgentEvent::SessionEnded {
            reason: "completed".into(),
            summary_sha256: "c".repeat(64),
        })
        .expect("seal 2");
}

fn main() {
    let dir = Path::new("tests/compat/vectors");
    fs::create_dir_all(dir).expect("create vectors dir");
    let canonical = Value::Array(canonical_cases());
    let self_hash = Value::Array(self_hash_cases());
    fs::write(
        dir.join("canonical_json.json"),
        serde_json::to_string_pretty(&canonical).unwrap() + "\n",
    )
    .expect("write canonical_json.json");
    fs::write(
        dir.join("self_hash.json"),
        serde_json::to_string_pretty(&self_hash).unwrap() + "\n",
    )
    .expect("write self_hash.json");
    emit_signed_ledger(dir);
    println!(
        "wrote {} canonical + {} self_hash vectors + rust_signed_ledger.ndjson",
        canonical.as_array().unwrap().len(),
        self_hash.as_array().unwrap().len(),
    );
}
