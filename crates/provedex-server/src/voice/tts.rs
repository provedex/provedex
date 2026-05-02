use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub struct TtsResult {
    pub audio_wav: Vec<u8>,
    pub used_synthesizer: bool,
}

pub fn locate_piper() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("PROVEDEX_PIPER_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(p) = which::which("piper") {
        return Some(p);
    }
    // Fall back to common install locations that may not be on the spawned
    // process PATH (e.g. `pipx install piper-tts` puts it in ~/.local/bin).
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local").join("bin").join("piper"));
    }
    candidates.push(PathBuf::from("/usr/local/bin/piper"));
    candidates.push(PathBuf::from("/opt/homebrew/bin/piper"));
    candidates.into_iter().find(|p| p.is_file())
}

pub fn default_voice_path() -> PathBuf {
    if let Ok(p) = std::env::var("PROVEDEX_PIPER_VOICE") {
        return PathBuf::from(p);
    }
    if let Some(home) = dirs::home_dir() {
        return home
            .join(".provedex")
            .join("voices")
            .join("en_US-amy-medium.onnx");
    }
    PathBuf::from("en_US-amy-medium.onnx")
}

/// Synthesize `text` to a WAV byte buffer using Piper. If the Piper binary is
/// not present on PATH (or `PROVEDEX_PIPER_BIN`), returns an empty buffer with
/// `used_synthesizer = false` so the caller can still emit a signed event.
pub async fn synthesize(text: &str) -> Result<TtsResult> {
    let piper = match locate_piper() {
        Some(p) => p,
        None => {
            return Ok(TtsResult {
                audio_wav: Vec::new(),
                used_synthesizer: false,
            });
        }
    };
    let voice = default_voice_path();
    if !voice.exists() {
        return Ok(TtsResult {
            audio_wav: Vec::new(),
            used_synthesizer: false,
        });
    }

    // Piper's --length_scale controls duration: < 1.0 speeds up, > 1.0 slows.
    // Default 0.9 keeps speech intelligible while shaving demo runtime.
    let length_scale = std::env::var("PROVEDEX_PIPER_LENGTH_SCALE")
        .ok()
        .filter(|s| s.parse::<f32>().is_ok())
        .unwrap_or_else(|| "0.9".into());

    let mut child = Command::new(&piper)
        .args([
            "--model",
            voice.to_str().context("piper voice path is not utf-8")?,
            "--output_file",
            "-",
            "--length_scale",
            &length_scale,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawning piper at {}", piper.display()))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).await?;
        stdin.shutdown().await?;
    }
    let output = child.wait_with_output().await.context("waiting on piper")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("piper failed: {stderr}");
    }
    Ok(TtsResult {
        audio_wav: output.stdout,
        used_synthesizer: true,
    })
}
