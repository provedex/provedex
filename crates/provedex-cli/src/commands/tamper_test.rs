use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use provedex_core::{read_file, AgentEvent};

/// Demo-only. Reads the ledger, mutates one event so the chain breaks, and
/// rewrites the file. Pick the middle event by default so a viewer can see
/// the tamper indicator land in the visible window.
pub fn run(ledger: PathBuf, seq: Option<u64>) -> Result<()> {
    let mut events = read_file(&ledger)?;
    if events.is_empty() {
        return Err(anyhow!("ledger is empty"));
    }
    let target_idx = match seq {
        Some(s) => events
            .iter()
            .position(|e| e.seq == s)
            .ok_or_else(|| anyhow!("seq {s} not found"))?,
        None => events.len() / 2,
    };
    let target_seq = events[target_idx].seq;
    mutate(&mut events[target_idx].event);
    let file = File::create(&ledger)?;
    let mut w = BufWriter::new(file);
    for evt in &events {
        let line = serde_json::to_vec(evt)?;
        w.write_all(&line)?;
        w.write_all(b"\n")?;
    }
    w.flush()?;
    println!("tampered with seq {target_seq} in {}", ledger.display());
    Ok(())
}

fn mutate(event: &mut AgentEvent) {
    match event {
        AgentEvent::UtteranceCaptured { transcript, .. } => {
            transcript.push_str(" [TAMPERED]");
        }
        AgentEvent::UtteranceSpoken { text, .. } => {
            text.push_str(" [TAMPERED]");
        }
        AgentEvent::ToolCalled { tool_name, .. } => {
            tool_name.push_str("_tampered");
        }
        AgentEvent::ToolReturned { success, .. } => {
            *success = !*success;
        }
        AgentEvent::ModelInvoked { prompt_tokens, .. } => {
            *prompt_tokens = prompt_tokens.wrapping_add(1);
        }
        AgentEvent::SessionStarted { agent_id, .. } => {
            agent_id.push_str("_tampered");
        }
        AgentEvent::SessionEnded { reason, .. } => {
            reason.push_str("_tampered");
        }
    }
}
