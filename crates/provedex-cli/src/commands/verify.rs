use std::path::PathBuf;

use anyhow::Result;
use provedex_core::{read_file, verify_chain, ChainStatus};

pub fn run(ledger: PathBuf) -> Result<()> {
    let events = read_file(&ledger)?;
    let report = verify_chain(&events);
    let status = match report.status {
        ChainStatus::Valid => "VALID",
        ChainStatus::Broken => "BROKEN",
    };
    println!("ledger: {}", ledger.display());
    println!("events: {}", report.event_count);
    println!("status: {status}");
    println!("root_hash: {}", report.root_hash);
    if let Some(seq) = report.broken_at_seq {
        println!("broken_at_seq: {seq}");
        if let Some(reason) = &report.broken_reason {
            println!("broken_reason: {reason}");
        }
    }
    if matches!(report.status, ChainStatus::Broken) {
        std::process::exit(1);
    }
    Ok(())
}
