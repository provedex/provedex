use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use provedex_core::{read_file, AgentEvent};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct TamperResponse {
    pub tampered_seq: u64,
    pub event_count: u64,
}

/// Demo-only. Mutates one event in the on-disk ledger so the chain breaks.
/// Picks the middle event so the visible event-stream column shows the failure.
pub async fn tamper_test(
    State(state): State<Arc<AppState>>,
) -> Result<Json<TamperResponse>, (StatusCode, String)> {
    let path = state.ledger.path().to_path_buf();
    let result = tokio::task::spawn_blocking(move || tamper(&path)).await;
    match result {
        Ok(Ok(resp)) => Ok(Json(resp)),
        Ok(Err(e)) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

fn tamper(path: &std::path::Path) -> anyhow::Result<TamperResponse> {
    let mut events = read_file(path)?;
    if events.is_empty() {
        anyhow::bail!("ledger is empty; capture some events before tampering");
    }
    let target_idx = events.len() / 2;
    let target_seq = events[target_idx].seq;
    mutate(&mut events[target_idx].event);
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    for evt in &events {
        let line = serde_json::to_vec(evt)?;
        w.write_all(&line)?;
        w.write_all(b"\n")?;
    }
    w.flush()?;
    Ok(TamperResponse {
        tampered_seq: target_seq,
        event_count: events.len() as u64,
    })
}

fn mutate(event: &mut AgentEvent) {
    match event {
        AgentEvent::UtteranceCaptured { transcript, .. } => transcript.push_str(" [TAMPERED]"),
        AgentEvent::UtteranceSpoken { text, .. } => text.push_str(" [TAMPERED]"),
        AgentEvent::ToolCalled { tool_name, .. } => tool_name.push_str("_tampered"),
        AgentEvent::ToolReturned { success, .. } => *success = !*success,
        AgentEvent::ModelInvoked { prompt_tokens, .. } => {
            *prompt_tokens = prompt_tokens.wrapping_add(1)
        }
        AgentEvent::SessionStarted { agent_id, .. } => agent_id.push_str("_tampered"),
        AgentEvent::SessionEnded { reason, .. } => reason.push_str("_tampered"),
    }
}
