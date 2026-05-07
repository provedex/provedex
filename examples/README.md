# Examples

Runnable integration examples. Each example is self-contained and documented enough that a new engineer can copy it into their project as a starting point.

## Layout

- `voice-scribe/` - full healthcare voice scribe demo (mirrors what the demo server runs).
- `basic-signing/` - minimal Rust sign-then-verify (also lives at `crates/provedex-core/examples/basic_signing.rs` for `cargo run --example`).
- `langchain-callback/` - FUTURE. LangChain CallbackHandler wired to Provedex.
- `letta-hook/` - FUTURE. Letta tool wrapper.
- `python-quickstart/` - FUTURE. `pip install provedex` end-to-end.
- `node-quickstart/` - FUTURE. `npm i @provedex/core` end-to-end.

## Convention

Each example has a `README.md` explaining: prerequisites, how to run, what to expect, what files were emitted, how to verify the output.
