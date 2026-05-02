use std::path::PathBuf;

use anyhow::Result;
use chrono::DateTime;
use provedex_core::{read_file, AgentEvent};

pub fn run(ledger: PathBuf) -> Result<()> {
    let events = read_file(&ledger)?;
    if events.is_empty() {
        println!("ledger is empty");
        return Ok(());
    }
    for evt in events {
        let ts = DateTime::from_timestamp(
            (evt.timestamp_nanos / 1_000_000_000) as i64,
            (evt.timestamp_nanos % 1_000_000_000) as u32,
        );
        let ts_str = ts
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| evt.timestamp_nanos.to_string());
        let summary = describe(&evt.event);
        println!("[{:>4}] {ts_str}  {summary}", evt.seq);
    }
    Ok(())
}

fn describe(e: &AgentEvent) -> String {
    match e {
        AgentEvent::SessionStarted {
            agent_id,
            model_id,
            session_id,
        } => format!("session_started agent={agent_id} model={model_id} session={session_id}"),
        AgentEvent::UtteranceCaptured {
            transcript,
            duration_ms,
            ..
        } => format!("utterance_captured ({duration_ms}ms): {transcript}"),
        AgentEvent::ToolCalled {
            tool_name,
            args_sha256,
            ..
        } => format!("tool_called {tool_name} args_sha256={}", short(args_sha256)),
        AgentEvent::ToolReturned {
            tool_name,
            latency_ms,
            success,
            ..
        } => format!("tool_returned {tool_name} {latency_ms}ms success={success}"),
        AgentEvent::ModelInvoked {
            model_id,
            prompt_tokens,
            response_tokens,
            ..
        } => format!("model_invoked {model_id} prompt={prompt_tokens} response={response_tokens}"),
        AgentEvent::UtteranceSpoken { text, .. } => format!("utterance_spoken: {text}"),
        AgentEvent::SessionEnded { reason, .. } => format!("session_ended reason={reason}"),
    }
}

fn short(hash: &str) -> String {
    if hash.len() > 12 {
        format!("{}...", &hash[..12])
    } else {
        hash.to_string()
    }
}
