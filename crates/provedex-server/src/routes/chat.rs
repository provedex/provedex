use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Multipart, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use provedex_core::AgentEvent;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::state::AppState;
use crate::voice;

const FAKE_TOOL: &str = "lookup_patient_history";

pub async fn chat(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Response, (StatusCode, String)> {
    let mut audio_bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        if field.name() == Some("audio") {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            audio_bytes = Some(bytes.to_vec());
        }
    }
    let audio_bytes = audio_bytes.ok_or((StatusCode::BAD_REQUEST, "missing audio field".into()))?;
    if audio_bytes.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "audio field is empty".into()));
    }

    let audio_sha = hex_sha256(&audio_bytes);

    let model_path = voice::stt::default_model_path();
    if !model_path.exists() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("whisper model missing at {}", model_path.display()),
        ));
    }
    let transcription = voice::stt::transcribe(audio_bytes, &model_path)
        .await
        .map_err(internal)?;
    state
        .seal_and_append(AgentEvent::UtteranceCaptured {
            audio_sha256: audio_sha.clone(),
            transcript: transcription.text.clone(),
            lang: transcription.lang.clone(),
            duration_ms: transcription.duration_ms,
        })
        .map_err(internal)?;

    // Mock tool call so the demo shows tool events alongside the model trace.
    let args = json!({"query": transcription.text});
    let args_sha = hex_sha256(args.to_string().as_bytes());
    state
        .seal_and_append(AgentEvent::ToolCalled {
            tool_name: FAKE_TOOL.into(),
            args_sha256: args_sha.clone(),
            args_redacted: json!({"query": "<redacted>"}),
        })
        .map_err(internal)?;
    let started = Instant::now();
    let tool_result = "no prior visits in last 12 months";
    let tool_latency_ms = started.elapsed().as_millis() as u64;
    state
        .seal_and_append(AgentEvent::ToolReturned {
            tool_name: FAKE_TOOL.into(),
            result_sha256: hex_sha256(tool_result.as_bytes()),
            latency_ms: tool_latency_ms,
            success: true,
        })
        .map_err(internal)?;

    let llm = voice::llm::chat(&transcription.text, "llama3.2:3b")
        .await
        .map_err(internal)?;
    state
        .seal_and_append(AgentEvent::ModelInvoked {
            model_id: llm.model_id.clone(),
            prompt_sha256: hex_sha256(llm.prompt_text.as_bytes()),
            response_sha256: hex_sha256(llm.content.as_bytes()),
            prompt_tokens: llm.prompt_tokens,
            response_tokens: llm.response_tokens,
        })
        .map_err(internal)?;

    let tts = voice::tts::synthesize(&llm.content)
        .await
        .map_err(internal)?;
    let response_audio_sha = if tts.audio_wav.is_empty() {
        String::new()
    } else {
        hex_sha256(&tts.audio_wav)
    };
    state
        .seal_and_append(AgentEvent::UtteranceSpoken {
            text_sha256: hex_sha256(llm.content.as_bytes()),
            text: llm.content.clone(),
            audio_sha256: response_audio_sha.clone(),
        })
        .map_err(internal)?;

    let body_json = json!({
        "transcript": transcription.text,
        "response_text": llm.content,
        "response_audio_b64": if tts.audio_wav.is_empty() {
            None
        } else {
            Some(base64_encode(&tts.audio_wav))
        },
        "tts_available": tts.used_synthesizer,
    });
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body_json.to_string()))
        .unwrap()
        .into_response())
}

fn hex_sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    STANDARD.encode(bytes)
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
