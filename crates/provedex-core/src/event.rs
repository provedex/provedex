use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum AgentEvent {
    SessionStarted {
        agent_id: String,
        model_id: String,
        session_id: String,
    },
    UtteranceCaptured {
        audio_sha256: String,
        transcript: String,
        lang: String,
        duration_ms: u64,
    },
    ToolCalled {
        tool_name: String,
        args_sha256: String,
        args_redacted: serde_json::Value,
    },
    ToolReturned {
        tool_name: String,
        result_sha256: String,
        latency_ms: u64,
        success: bool,
    },
    ModelInvoked {
        model_id: String,
        prompt_sha256: String,
        response_sha256: String,
        prompt_tokens: u32,
        response_tokens: u32,
    },
    UtteranceSpoken {
        text_sha256: String,
        text: String,
        audio_sha256: String,
    },
    SessionEnded {
        reason: String,
        summary_sha256: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_started_roundtrip() {
        let e = AgentEvent::SessionStarted {
            agent_id: "agent-1".into(),
            model_id: "llama3.2:3b".into(),
            session_id: "sess-1".into(),
        };
        let json = serde_json::to_string(&e).unwrap();
        let back: AgentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn tagged_representation_uses_type_and_payload() {
        let e = AgentEvent::SessionEnded {
            reason: "user_hangup".into(),
            summary_sha256: "abc".into(),
        };
        let v: serde_json::Value = serde_json::to_value(&e).unwrap();
        assert_eq!(v["type"], "SessionEnded");
        assert_eq!(v["payload"]["reason"], "user_hangup");
    }
}
