use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::Command;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct Transcription {
    pub text: String,
    pub duration_ms: u64,
    pub lang: String,
}

pub fn default_model_path() -> PathBuf {
    if let Ok(p) = std::env::var("PROVEDEX_WHISPER_MODEL") {
        return PathBuf::from(p);
    }
    if let Some(home) = dirs::home_dir() {
        return home
            .join(".provedex")
            .join("models")
            .join("ggml-base.en.bin");
    }
    PathBuf::from("ggml-base.en.bin")
}

/// Decode an arbitrary container (WebM/Opus/WAV) into 16 kHz mono f32 PCM and
/// transcribe it with whisper.cpp. Decoding is delegated to ffmpeg because
/// browsers default to Opus-in-WebM from MediaRecorder.
pub async fn transcribe(audio_bytes: Vec<u8>, model_path: &Path) -> Result<Transcription> {
    let pcm = decode_to_pcm16k_mono(&audio_bytes).await?;
    let duration_ms = ((pcm.len() as u64) * 1000) / 16_000;

    let model_path = model_path.to_path_buf();
    let text = tokio::task::spawn_blocking(move || run_whisper(&model_path, pcm)).await??;

    Ok(Transcription {
        text: text.trim().to_string(),
        duration_ms,
        lang: "en".into(),
    })
}

fn run_whisper(model_path: &Path, samples: Vec<f32>) -> Result<String> {
    let ctx = WhisperContext::new_with_params(
        model_path
            .to_str()
            .context("whisper model path is not utf-8")?,
        WhisperContextParameters::default(),
    )
    .context("loading whisper model")?;
    let mut state = ctx.create_state().context("creating whisper state")?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_language(Some("en"));
    state
        .full(params, &samples)
        .context("whisper inference failed")?;
    let n = state.full_n_segments().context("counting segments")?;
    let mut text = String::new();
    for i in 0..n {
        if let Ok(seg) = state.full_get_segment_text(i) {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(seg.trim());
        }
    }
    Ok(text)
}

async fn decode_to_pcm16k_mono(input: &[u8]) -> Result<Vec<f32>> {
    let mut child = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            "pipe:0",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-f",
            "f32le",
            "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawning ffmpeg; install via `brew install ffmpeg`")?;

    if let Some(mut stdin) = child.stdin.take() {
        let bytes = input.to_vec();
        tokio::spawn(async move {
            let _ = tokio::io::AsyncWriteExt::write_all(&mut stdin, &bytes).await;
        });
    }

    let output = child
        .wait_with_output()
        .await
        .context("waiting on ffmpeg")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg failed: {stderr}");
    }
    let bytes = output.stdout;
    if bytes.len() % 4 != 0 {
        anyhow::bail!("ffmpeg produced misaligned f32 stream");
    }
    let mut samples = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(samples)
}
