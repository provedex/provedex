# Security policy

## Reporting a vulnerability

Email `adityasuresh22+security@gmail.com`. This address is a placeholder until the project domain is registered; it will move to `security@provedex.ai` once the domain is in hand.

Do not open public issues or pull requests for security problems. If the issue is exploitable, do not include proof-of-concept code in the initial email body; offer to send it on request.

A PGP key for encrypted reports will be published here in a future update. Until then, plaintext email is acceptable.

## Scope

The following crates and binaries are in scope:

- `provedex-core` (signing primitives, hash chain, NDJSON ledger)
- `provedex-cli` (`provedex` command-line tool)
- `provedex-server` (Axum demo server and voice pipeline)

Out of scope: the demo frontend assets, third-party dependencies (report those upstream), and runtime services (Ollama, Piper, whisper.cpp, ffmpeg) unless the bug is in how Provedex calls them.

## Response

We aim to respond to a report within 5 business days with one of:

- An acknowledgement and a tracking ID.
- A request for more information.
- A statement that the report is out of scope, with the reason.

A fix or mitigation plan will follow once the report is triaged. Coordinated disclosure is preferred; we will agree on a public-disclosure date with the reporter.

## Versions

The audit-ledger primitives are pre-1.0. Patch releases for confirmed vulnerabilities will land on `main` and a tagged release.
