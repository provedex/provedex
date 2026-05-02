use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event::AgentEvent;

pub const GENESIS_PARENT_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SignedEvent {
    pub seq: u64,
    pub timestamp_nanos: u64,
    pub event: AgentEvent,
    pub parent_hash: String,
    pub self_hash: String,
    pub signature: String,
    pub signer_pubkey: String,
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
}
