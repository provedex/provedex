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
