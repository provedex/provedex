use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "provedex", version, about = "Provedex audit ledger CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verify the cryptographic chain of an NDJSON ledger.
    Verify {
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Replay a session transcript from the ledger to stdout.
    Replay {
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Export the full signed bundle as JSON.
    Export {
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long, short)]
        output: PathBuf,
    },
    /// Demo-only. Corrupt one event in the ledger to show the chain breaks.
    #[cfg(feature = "demo")]
    TamperTest {
        #[arg(long)]
        ledger: Option<PathBuf>,
        /// Sequence number of the event to corrupt. Defaults to the middle event.
        #[arg(long)]
        seq: Option<u64>,
    },
}

fn ledger_path(opt: Option<PathBuf>) -> Result<PathBuf> {
    match opt {
        Some(p) => Ok(p),
        None => {
            provedex_core::default_ledger_path().context("could not determine default ledger path")
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Verify { ledger } => commands::verify::run(ledger_path(ledger).unwrap()),
        Command::Replay { ledger } => commands::replay::run(ledger_path(ledger).unwrap()),
        Command::Export { ledger, output } => {
            commands::export::run(ledger_path(ledger).unwrap(), output)
        }
        #[cfg(feature = "demo")]
        Command::TamperTest { ledger, seq } => {
            commands::tamper_test::run(ledger_path(ledger).unwrap(), seq)
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}
