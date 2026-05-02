use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const OLLAMA_URL: &str = "http://127.0.0.1:11434/api/chat";
const SYSTEM_PROMPT: &str = "You are a clinical voice scribe assistant. Acknowledge the user's medical note succinctly and ask one clarifying question if needed. Keep responses under three sentences.";

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatResponseMessage,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

pub struct LlmReply {
    pub content: String,
    pub prompt_text: String,
    pub model_id: String,
    pub prompt_tokens: u32,
    pub response_tokens: u32,
}

pub async fn chat(user_text: &str, model_id: &str) -> Result<LlmReply> {
    let prompt_text = format!("system:{SYSTEM_PROMPT}\nuser:{user_text}");
    let req = ChatRequest {
        model: model_id,
        messages: vec![
            ChatMessage {
                role: "system",
                content: SYSTEM_PROMPT,
            },
            ChatMessage {
                role: "user",
                content: user_text,
            },
        ],
        stream: false,
    };
    let client = reqwest::Client::new();
    let resp = client
        .post(OLLAMA_URL)
        .json(&req)
        .send()
        .await
        .context("calling ollama; ensure `ollama serve` is running")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("ollama returned {status}: {body}");
    }
    let parsed: ChatResponse = resp.json().await.context("decoding ollama response")?;
    Ok(LlmReply {
        content: parsed.message.content,
        prompt_text,
        model_id: model_id.to_string(),
        prompt_tokens: parsed.prompt_eval_count.unwrap_or(0),
        response_tokens: parsed.eval_count.unwrap_or(0),
    })
}
