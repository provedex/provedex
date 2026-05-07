//! Emits canonical-JSON bytes + SHA-256 for documented test vectors. Run via
//! `cargo run -p provedex-core --example print_test_vectors`. The output is
//! pasted verbatim into `docs/spec/canonical-json.md` and
//! `docs/spec/event-schema-v1.md`.

use provedex_core::{canonical_json, AgentEvent};
use serde_json::json;
use sha2::{Digest, Sha256};

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
}
