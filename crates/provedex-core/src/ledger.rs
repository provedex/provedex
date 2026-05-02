use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use thiserror::Error;

use crate::signed::SignedEvent;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Append-only NDJSON ledger. Each line is one canonically-serialized
/// `SignedEvent`. Writes are fsynced so the chain survives a crash mid-session.
#[derive(Debug)]
pub struct Ledger {
    path: PathBuf,
    writer: Mutex<File>,
}

impl Ledger {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, LedgerError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        Ok(Self {
            path,
            writer: Mutex::new(file),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&self, event: &SignedEvent) -> Result<(), LedgerError> {
        let line = serde_json::to_vec(event)?;
        let mut file = self.writer.lock().expect("ledger writer mutex poisoned");
        file.write_all(&line)?;
        file.write_all(b"\n")?;
        file.sync_data()?;
        Ok(())
    }

    pub fn read_all(&self) -> Result<Vec<SignedEvent>, LedgerError> {
        read_file(&self.path)
    }

    pub fn last(&self) -> Result<Option<SignedEvent>, LedgerError> {
        Ok(self.read_all()?.into_iter().last())
    }

    pub fn count(&self) -> Result<u64, LedgerError> {
        Ok(self.read_all()?.len() as u64)
    }
}

pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Vec<SignedEvent>, LedgerError> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let evt: SignedEvent = serde_json::from_str(&line)?;
        out.push(evt);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::AgentEvent;
    use tempfile::tempdir;

    fn synth_event(seq: u64) -> SignedEvent {
        SignedEvent {
            seq,
            timestamp_nanos: seq * 1_000,
            event: AgentEvent::SessionStarted {
                agent_id: "agent".into(),
                model_id: "m".into(),
                session_id: format!("s{seq}"),
            },
            parent_hash: format!("{:064x}", seq),
            self_hash: format!("{:064x}", seq + 1),
            signature: "00".repeat(64),
            signer_pubkey: "00".repeat(32),
        }
    }

    #[test]
    fn append_and_read_back() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ledger.ndjson");
        let ledger = Ledger::open(&path).unwrap();
        for i in 0..5 {
            ledger.append(&synth_event(i)).unwrap();
        }
        let all = ledger.read_all().unwrap();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0].seq, 0);
        assert_eq!(all[4].seq, 4);
    }

    #[test]
    fn append_persists_across_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ledger.ndjson");
        {
            let l = Ledger::open(&path).unwrap();
            l.append(&synth_event(0)).unwrap();
            l.append(&synth_event(1)).unwrap();
        }
        let l = Ledger::open(&path).unwrap();
        l.append(&synth_event(2)).unwrap();
        let all = l.read_all().unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[2].seq, 2);
    }

    #[test]
    fn last_returns_most_recent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ledger.ndjson");
        let l = Ledger::open(&path).unwrap();
        for i in 0..3 {
            l.append(&synth_event(i)).unwrap();
        }
        assert_eq!(l.last().unwrap().unwrap().seq, 2);
    }

    #[test]
    fn empty_ledger_reads_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ledger.ndjson");
        let l = Ledger::open(&path).unwrap();
        assert!(l.read_all().unwrap().is_empty());
        assert!(l.last().unwrap().is_none());
    }
}
