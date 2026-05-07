# crates/provedex-server - demo voice agent server

Axum server that runs a local voice scribe pipeline and emits signed events. Bound to port 3000. Frontend (apps/demo-web/) and API share the same port.

## Layout

```
src/
  main.rs              axum boot, router, frontend resolver
  state.rs             AppState: keypair, ledger, broadcast, seq counter, parent_hash
  voice/
    stt.rs             whisper-rs + ffmpeg PCM decode
    llm.rs             ollama HTTP client
    tts.rs             piper subprocess
    mod.rs             module declarations
  routes/
    healthz.rs         GET /api/healthz
    chat.rs            POST /api/chat (multipart audio in, signed events emitted, audio reply out)
    events.rs          GET /api/events (SSE stream of SignedEvents)
    verify.rs          POST /api/verify
    export.rs          POST /api/export (downloadable bundle)
    tamper.rs          POST /api/tamper-test (demo-only, gated #[cfg(feature = "demo")])
    mod.rs             module declarations
```

## State mutation rule

`AppState::seal_and_append` is the ONLY sanctioned event emitter. It atomically:
1. Increments `seq` (AtomicU64).
2. Locks `parent_hash`.
3. Calls `SignedEvent::seal(seq, event, parent, &keypair)`.
4. Appends to the on-disk ledger (fsynced).
5. Updates `parent_hash` to the new `self_hash`.
6. Broadcasts the SignedEvent to SSE subscribers.

Never call `Ledger::append` directly from a route. Never bump `seq` outside `seal_and_append`. Never broadcast a SignedEvent that has not been written.

## Adding a new route

1. Create `src/routes/<name>.rs` with an async handler taking `State<Arc<AppState>>`.
2. Wire in `routes/mod.rs` and `main.rs::router`.
3. Demo-only routes: gate the module declaration AND the route registration with `#[cfg(feature = "demo")]`.

## Adding a voice pipeline stage

1. Create `src/voice/<name>.rs` exposing an async function (e.g. `pub async fn transcribe(...)`).
2. Each stage emits exactly one signed event via `AppState::seal_and_append` after its work completes.
3. Errors bubble as `anyhow::Result`. Routes map to `(StatusCode, String)` via the `internal` helper.

## Default ports + paths + env vars

- Port: 3000 (override via `--port`).
- Frontend dir: `apps/demo-web/` (override via `--frontend-dir`). Resolver walks several candidate paths so cargo-run from anywhere works.
- Whisper model: `~/.provedex/models/ggml-base.en.bin` or `$PROVEDEX_WHISPER_MODEL`.
- Piper binary: `which piper` or `~/.local/bin/piper` or `$PROVEDEX_PIPER_BIN`.
- Piper voice: `~/.provedex/voices/en_US-amy-medium.onnx` or `$PROVEDEX_PIPER_VOICE`.
- Piper length scale (speech speed): default 0.9, override via `$PROVEDEX_PIPER_LENGTH_SCALE`. < 1.0 is faster.

## Multipart parsing

`/api/chat` reads a single field named `audio`. Other fields are ignored. Bytes go directly to `voice::stt::transcribe`, which delegates to ffmpeg for container decoding.

## SSE conventions

- Event name: `signed`.
- Data: full `SignedEvent` JSON.
- Backlog is replayed on subscribe so a fresh page load sees prior events.
- Keep-alive every 15s.

## Frontend integration

The server static-serves `apps/demo-web/` at `/`. API mounted under `/api`. CORS open to any origin (demo). Single port keeps the browser inside same-origin so EventSource works without preflight.

## Forbidden

- No persistent connections to the customer's ledger from this binary. This is a demo server, not a production gateway.
- No background workers that emit events without a request. All events come from request flow.
- No state in module-level statics. All state lives on `AppState`.
