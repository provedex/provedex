use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use provedex_core::{read_file, ExportBundle};

pub fn run(ledger: PathBuf, output: PathBuf) -> Result<()> {
    let events = read_file(&ledger)?;
    let bundle = ExportBundle::from_events(events);
    let json =
        serde_json::to_string_pretty(&bundle).context("serializing export bundle to json")?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, json)?;
    println!(
        "exported {} events to {}",
        bundle.events.len(),
        output.display()
    );
    Ok(())
}
