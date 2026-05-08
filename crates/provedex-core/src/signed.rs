use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::event::AgentEvent;
use crate::keys::{verify_signature, KeyError, SigningKeypair};

pub const GENESIS_PARENT_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Error)]
pub enum SignedError {
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("key: {0}")]
    Key(#[from] KeyError),
    #[error("self_hash mismatch")]
    HashMismatch,
    #[error("invalid hex: {0}")]
    Hex(#[from] hex::FromHexError),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SignedEvent {
    pub seq: u64,
    pub timestamp_nanos: u64,
    pub event: AgentEvent,
    pub parent_hash: String,
    pub self_hash: String,
    pub signature: String,
    pub signer_pubkey: String,
}

impl SignedEvent {
    /// Build, hash, and sign a new event in one shot. The caller supplies the
    /// previous event's `self_hash` (or `GENESIS_PARENT_HASH` for seq 0).
    pub fn seal(
        seq: u64,
        event: AgentEvent,
        parent_hash: &str,
        keypair: &SigningKeypair,
    ) -> Result<Self, SignedError> {
        let timestamp_nanos = now_nanos();
        let hash_bytes = compute_self_hash(seq, timestamp_nanos, &event, parent_hash)?;
        let signature = keypair.sign(&hash_bytes);
        Ok(Self {
            seq,
            timestamp_nanos,
            event,
            parent_hash: parent_hash.to_string(),
            self_hash: hex::encode(hash_bytes),
            signature: hex::encode(signature.to_bytes()),
            signer_pubkey: keypair.pubkey_hex(),
        })
    }

    /// Recompute the hash and verify both the hash and the signature against
    /// the embedded public key. Does not check parent linkage; that lives in
    /// `chain::verify_chain`.
    pub fn verify_self(&self) -> Result<(), SignedError> {
        let recomputed = compute_self_hash(
            self.seq,
            self.timestamp_nanos,
            &self.event,
            &self.parent_hash,
        )?;
        if hex::encode(recomputed) != self.self_hash {
            return Err(SignedError::HashMismatch);
        }
        verify_signature(&self.signer_pubkey, &recomputed, &self.signature)?;
        Ok(())
    }
}

pub fn compute_self_hash(
    seq: u64,
    timestamp_nanos: u64,
    event: &AgentEvent,
    parent_hash: &str,
) -> Result<[u8; 32], SignedError> {
    let mut map = serde_json::Map::new();
    map.insert("event".into(), serde_json::to_value(event)?);
    map.insert("parent_hash".into(), Value::String(parent_hash.to_string()));
    map.insert("seq".into(), Value::Number(seq.into()));
    map.insert(
        "timestamp_nanos".into(),
        Value::Number(timestamp_nanos.into()),
    );
    let bytes = canonical_json(&Value::Object(map));
    let digest = Sha256::digest(&bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    Ok(out)
}

fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Serialize a JSON value with sorted object keys, no whitespace, and a fixed
/// number/string representation. Required because the hash chain and signature
/// must be reproducible from the stored event regardless of producer.
pub fn canonical_json(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write_canonical(&mut out, value);
    out
}

fn write_canonical(out: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
        Value::Number(n) => out.extend_from_slice(n.to_string().as_bytes()),
        Value::String(s) => write_json_string(out, s),
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_canonical(out, item);
            }
            out.push(b']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push(b'{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_json_string(out, key);
                out.push(b':');
                write_canonical(out, &map[*key]);
            }
            out.push(b'}');
        }
    }
}

fn write_json_string(out: &mut Vec<u8>, s: &str) {
    out.push(b'"');
    for c in s.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\x08' => out.extend_from_slice(b"\\b"),
            '\x0c' => out.extend_from_slice(b"\\f"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_orders_object_keys() {
        let a = json!({"b": 1, "a": 2, "c": [3, 2, 1]});
        let b = json!({"c": [3, 2, 1], "a": 2, "b": 1});
        assert_eq!(canonical_json(&a), canonical_json(&b));
        assert_eq!(canonical_json(&a), br#"{"a":2,"b":1,"c":[3,2,1]}"#.to_vec());
    }

    #[test]
    fn canonical_escapes_control_chars() {
        let v = json!({"k": "line1\nline2\t\"end\""});
        assert_eq!(
            canonical_json(&v),
            br#"{"k":"line1\nline2\t\"end\""}"#.to_vec()
        );
    }

    #[test]
    fn canonical_is_stable_across_roundtrip() {
        let v = json!({
            "session_id": "abc",
            "events": [{"type": "x", "n": 1}, {"type": "y", "n": 2}]
        });
        let bytes = canonical_json(&v);
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(canonical_json(&parsed), bytes);
    }

    #[test]
    fn seal_and_verify_self() {
        let kp = SigningKeypair::generate();
        let evt = AgentEvent::SessionStarted {
            agent_id: "a".into(),
            model_id: "m".into(),
            session_id: "s".into(),
        };
        let s = SignedEvent::seal(0, evt, GENESIS_PARENT_HASH, &kp).unwrap();
        s.verify_self().unwrap();
    }

    #[test]
    fn tampering_with_event_breaks_self_hash() {
        let kp = SigningKeypair::generate();
        let evt = AgentEvent::SessionStarted {
            agent_id: "a".into(),
            model_id: "m".into(),
            session_id: "s".into(),
        };
        let mut s = SignedEvent::seal(0, evt, GENESIS_PARENT_HASH, &kp).unwrap();
        s.event = AgentEvent::SessionEnded {
            reason: "tampered".into(),
            summary_sha256: "x".into(),
        };
        assert!(matches!(s.verify_self(), Err(SignedError::HashMismatch)));
    }
}
