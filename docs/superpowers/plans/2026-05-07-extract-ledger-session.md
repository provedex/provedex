# Extract LedgerSession Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the `seal_and_append` primitive from `provedex-server`'s `AppState` into a new `LedgerSession` type in `provedex-core`, so the upcoming `provedex-agent` sidecar (issue #11) and the existing `provedex-server` share the same primitive without code duplication.

**Architecture:** Move `keypair`, `ledger`, `seq` counter, `parent_hash` mutex, `session_id`, and the `seal_and_append` method into a new `LedgerSession` type in `provedex-core`. `provedex-server`'s `AppState` wraps a `LedgerSession` and adds the server-specific tokio `broadcast::Sender<SignedEvent>`.

**Tech Stack:** Rust 1.89, tokio (server only), provedex-core existing primitives (`SigningKeypair`, `Ledger`, `SignedEvent`, `GENESIS_PARENT_HASH`).

---

## Pre-flight

- Branch: `refactor/extract-ledger-session` (already created).
- ADR not required: this is a refactor, not a new architectural decision. The decision to extract was made in ADR 0004.
- Commit cadence: one commit per completed task. Squash on merge.

## File Structure

**Create:**
- `crates/provedex-core/src/session.rs` - new `LedgerSession` type.

**Modify:**
- `crates/provedex-core/src/lib.rs` - re-export `LedgerSession`.
- `crates/provedex-server/src/state.rs` - `AppState` wraps `LedgerSession`, holds broadcast channel separately.
- `crates/provedex-server/src/main.rs` - constructor updated.
- `crates/provedex-server/src/routes/*.rs` - no change expected; `state.seal_and_append(...)` continues to work as a delegating method.

**Test:**
- `crates/provedex-core/src/session.rs` (`#[cfg(test)] mod tests`).

---

## Task 1: Plan, commit, push

**Files:**
- Modify: `docs/superpowers/plans/2026-05-07-extract-ledger-session.md` (this file)

- [ ] **Step 1: Stage and commit the plan**

```bash
git add docs/superpowers/plans/2026-05-07-extract-ledger-session.md
git commit -m "docs(plan): extract LedgerSession from provedex-server"
git push -u origin refactor/extract-ledger-session
```

Expected: branch published, plan visible in GitHub.

---

## Task 2: Write failing tests for LedgerSession

**Files:**
- Create: `crates/provedex-core/src/session.rs` (with `#[cfg(test)] mod tests` only; production code is empty/stub)

- [ ] **Step 1: Write failing tests**

```rust
// crates/provedex-core/src/session.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::AgentEvent;
    use crate::keys::SigningKeypair;
    use tempfile::tempdir;

    fn fixture(dir: &std::path::Path) -> LedgerSession {
        let kp = SigningKeypair::generate();
        let ledger = crate::ledger::Ledger::open(dir.join("ledger.ndjson")).unwrap();
        LedgerSession::new(kp, ledger, "test-session".into())
    }

    fn evt(i: u64) -> AgentEvent {
        AgentEvent::SessionStarted {
            agent_id: format!("a{i}"),
            model_id: "m".into(),
            session_id: "s".into(),
        }
    }

    #[test]
    fn first_event_uses_genesis_parent() {
        let dir = tempdir().unwrap();
        let s = fixture(dir.path());
        let signed = s.seal_and_append(evt(0)).unwrap();
        assert_eq!(signed.seq, 0);
        assert_eq!(signed.parent_hash, crate::signed::GENESIS_PARENT_HASH);
    }

    #[test]
    fn subsequent_events_chain_to_previous() {
        let dir = tempdir().unwrap();
        let s = fixture(dir.path());
        let a = s.seal_and_append(evt(0)).unwrap();
        let b = s.seal_and_append(evt(1)).unwrap();
        assert_eq!(b.seq, 1);
        assert_eq!(b.parent_hash, a.self_hash);
    }

    #[test]
    fn ledger_picks_up_pre_existing_events_on_open() {
        let dir = tempdir().unwrap();
        // first session writes 2 events
        {
            let s = fixture(dir.path());
            s.seal_and_append(evt(0)).unwrap();
            s.seal_and_append(evt(1)).unwrap();
        }
        // second session opens same ledger, must continue at seq 2 with correct parent
        let kp = SigningKeypair::generate();
        let ledger = crate::ledger::Ledger::open(dir.path().join("ledger.ndjson")).unwrap();
        let s = LedgerSession::new(kp, ledger, "resume".into());
        let c = s.seal_and_append(evt(2)).unwrap();
        assert_eq!(c.seq, 2);
        let report = crate::chain::verify_chain(&s.ledger().read_all().unwrap());
        assert_eq!(report.status, crate::chain::ChainStatus::Valid);
        assert_eq!(report.event_count, 3);
    }

    #[test]
    fn pubkey_hex_exposes_signer_identity() {
        let dir = tempdir().unwrap();
        let s = fixture(dir.path());
        let pk = s.pubkey_hex();
        assert_eq!(pk.len(), 64);
        let signed = s.seal_and_append(evt(0)).unwrap();
        assert_eq!(signed.signer_pubkey, pk);
    }
}
```

- [ ] **Step 2: Add empty `pub struct LedgerSession;` placeholder above the tests so the file compiles enough to fail**

```rust
// crates/provedex-core/src/session.rs

pub struct LedgerSession;
```

- [ ] **Step 3: Wire module into lib.rs**

```rust
// crates/provedex-core/src/lib.rs
// Add line:
pub mod session;
pub use session::LedgerSession;
```

- [ ] **Step 4: Run tests, verify they fail**

```bash
cargo test --workspace --all-features 2>&1 | grep -E "FAILED|test result"
```

Expected: 4 new tests fail (compile errors on missing methods).

- [ ] **Step 5: Commit**

```bash
git add crates/provedex-core/src/session.rs crates/provedex-core/src/lib.rs
git commit -m "test(core): add failing tests for LedgerSession"
```

---

## Task 3: Implement LedgerSession (GREEN)

**Files:**
- Modify: `crates/provedex-core/src/session.rs`

- [ ] **Step 1: Replace the placeholder with a real implementation**

```rust
// crates/provedex-core/src/session.rs

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use thiserror::Error;

use crate::event::AgentEvent;
use crate::keys::SigningKeypair;
use crate::ledger::{Ledger, LedgerError};
use crate::signed::{SignedEvent, SignedError, GENESIS_PARENT_HASH};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("ledger: {0}")]
    Ledger(#[from] LedgerError),
    #[error("signed: {0}")]
    Signed(#[from] SignedError),
}

/// Owns the seq counter, parent_hash mutex, signing key, and ledger handle for
/// a single signing session. Both `provedex-server` and `provedex-agent` build
/// on this primitive so the chain invariants live in one place.
#[derive(Debug)]
pub struct LedgerSession {
    session_id: String,
    keypair: SigningKeypair,
    ledger: Ledger,
    seq: AtomicU64,
    parent_hash: Mutex<String>,
}

impl LedgerSession {
    /// Construct a session, resuming from any pre-existing events on disk.
    pub fn new(keypair: SigningKeypair, ledger: Ledger, session_id: String) -> Self {
        let existing = ledger.read_all().unwrap_or_default();
        let (seq, parent_hash) = match existing.last() {
            Some(last) => (last.seq + 1, last.self_hash.clone()),
            None => (0, GENESIS_PARENT_HASH.to_string()),
        };
        Self {
            session_id,
            keypair,
            ledger,
            seq: AtomicU64::new(seq),
            parent_hash: Mutex::new(parent_hash),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn pubkey_hex(&self) -> String {
        self.keypair.pubkey_hex()
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Atomically allocate the next seq, sign the event against the current
    /// parent hash, append to the ledger, and update the parent hash. This is
    /// the only sanctioned event emitter for live runs.
    pub fn seal_and_append(&self, event: AgentEvent) -> Result<SignedEvent, SessionError> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        let mut parent = self.parent_hash.lock().expect("parent_hash mutex poisoned");
        let signed = SignedEvent::seal(seq, event, &parent, &self.keypair)?;
        self.ledger.append(&signed)?;
        *parent = signed.self_hash.clone();
        Ok(signed)
    }
}
```

- [ ] **Step 2: Run tests, verify they pass**

```bash
cargo test --workspace --all-features 2>&1 | grep -E "FAILED|test result"
```

Expected: all session tests pass, all prior tests still pass (was 23, now 27).

- [ ] **Step 3: Run fmt + clippy**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: both clean.

- [ ] **Step 4: Commit**

```bash
git add crates/provedex-core/src/session.rs
git commit -m "feat(core): add LedgerSession with seal_and_append primitive"
```

---

## Task 4: Refactor provedex-server AppState to use LedgerSession

**Files:**
- Modify: `crates/provedex-server/src/state.rs`
- Modify: `crates/provedex-server/src/main.rs` (only if state constructor signature changes)

- [ ] **Step 1: Rewrite `state.rs` to wrap LedgerSession**

```rust
// crates/provedex-server/src/state.rs

use std::path::PathBuf;

use anyhow::{Context, Result};
use provedex_core::{
    default_key_path, default_ledger_path, AgentEvent, Ledger, LedgerSession, SignedEvent,
    SigningKeypair,
};
use tokio::sync::broadcast;

const EVENT_CHANNEL_CAPACITY: usize = 128;

/// Server-wide state. Wraps a `LedgerSession` (signing primitive) and adds the
/// SSE broadcast channel that the demo UI subscribes to.
pub struct AppState {
    pub session: LedgerSession,
    pub broadcast: broadcast::Sender<SignedEvent>,
}

impl AppState {
    pub fn initialize(
        ledger_override: Option<PathBuf>,
        key_override: Option<PathBuf>,
    ) -> Result<Self> {
        let ledger_path = ledger_override.unwrap_or(default_ledger_path()?);
        let key_path = key_override.unwrap_or(default_key_path()?);

        let keypair = SigningKeypair::load_or_create(&key_path)
            .with_context(|| format!("loading or creating keypair at {}", key_path.display()))?;
        let ledger = Ledger::open(&ledger_path)
            .with_context(|| format!("opening ledger at {}", ledger_path.display()))?;
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = LedgerSession::new(keypair, ledger, session_id);

        let (tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);

        Ok(Self {
            session,
            broadcast: tx,
        })
    }

    /// Server-side wrapper that seals + broadcasts the event so SSE
    /// subscribers see it without the routes needing to know about both
    /// concerns.
    pub fn seal_and_append(&self, event: AgentEvent) -> Result<SignedEvent> {
        let signed = self.session.seal_and_append(event)?;
        let _ = self.broadcast.send(signed.clone());
        Ok(signed)
    }

    pub fn session_id(&self) -> &str {
        self.session.session_id()
    }

    pub fn pubkey_hex(&self) -> String {
        self.session.pubkey_hex()
    }

    pub fn ledger(&self) -> &Ledger {
        self.session.ledger()
    }
}
```

- [ ] **Step 2: Update routes that previously read `state.session_id` / `state.keypair.pubkey_hex()` / `state.ledger`**

Most routes already call `state.seal_and_append(...)`, which continues to work. The two callers that read other fields are `routes/healthz.rs` (session_id + pubkey) and `routes/events.rs` + `routes/verify.rs` + `routes/export.rs` (ledger). Update each to use the new accessor methods (`state.session_id()`, `state.pubkey_hex()`, `state.ledger()`).

Example for healthz:

```rust
// crates/provedex-server/src/routes/healthz.rs
pub async fn healthz(State(state): State<Arc<AppState>>) -> Json<Health> {
    Json(Health {
        status: "ok",
        session_id: state.session_id().to_string(),
        pubkey: state.pubkey_hex(),
    })
}
```

For verify.rs:

```rust
let events = state
    .ledger()
    .read_all()
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
```

Mirror the pattern in `events.rs` and `export.rs`.

- [ ] **Step 3: Build + run tests + clippy**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Expected: green.

- [ ] **Step 4: Commit**

```bash
git add crates/provedex-server/src/state.rs crates/provedex-server/src/routes
git commit -m "refactor(server): AppState wraps LedgerSession, broadcasts after seal"
```

---

## Task 5: End-to-end smoke

**Files:** none modified, only verification.

- [ ] **Step 1: Reset ledger**

```bash
rm -f ~/.provedex/ledger.ndjson
```

- [ ] **Step 2: Start server**

```bash
cargo run -p provedex-server --features demo &
sleep 5
```

- [ ] **Step 3: Health check + verify chain start**

```bash
curl -sS http://127.0.0.1:3000/api/healthz | jq .
curl -sS -X POST http://127.0.0.1:3000/api/verify | jq .
```

Expected: healthz returns `status: ok` with session_id + pubkey; verify returns `status: valid`, event_count 1 (the SessionStarted event emitted on boot).

- [ ] **Step 4: Tamper + verify**

```bash
curl -sS -X POST http://127.0.0.1:3000/api/tamper-test | jq .
curl -sS -X POST http://127.0.0.1:3000/api/verify | jq .
```

Expected: tamper returns the seq it corrupted; verify returns `status: broken` with `broken_at_seq` set.

- [ ] **Step 5: Stop server**

```bash
pkill -f provedex-server
```

- [ ] **Step 6: Reset ledger to a clean state**

```bash
rm -f ~/.provedex/ledger.ndjson
```

No commit; this step is verification only.

---

## Task 6: Self-review using code-review-provedex skill

**Files:** none modified.

- [ ] **Step 1: Generate diff for review**

```bash
git diff main...HEAD --stat
git diff main...HEAD
```

- [ ] **Step 2: Apply auto-block invariants from `code-review-provedex` skill**

Walk the diff. For each file, verify:
- canonical_json + compute_self_hash + GENESIS_PARENT_HASH unchanged. (LedgerSession only orchestrates; it does not change the crypto primitives.)
- public API in provedex-core: `LedgerSession`, `SessionError` are new pub items. Both must have `///` doc. `LedgerSession::new` and `seal_and_append` are public methods on a public struct, so each needs at least one runnable doctest.
- conventional commit subjects across the branch.
- ASCII only. Run `grep -rnP '[^\x00-\x7F]' crates/`.
- AI slop adjective audit.
- No new top-level dir.
- No `unsafe` in core.
- No `unwrap` outside tests.

- [ ] **Step 3: Add doctests to LedgerSession::new and seal_and_append**

Both are public. The skill says "Public methods on public structs need at least one runnable doctest". Add doctests now.

```rust
/// Construct a session, resuming from any pre-existing events on disk.
///
/// ```
/// use provedex_core::{Ledger, LedgerSession, SigningKeypair};
/// let dir = tempfile::tempdir().unwrap();
/// let kp = SigningKeypair::generate();
/// let ledger = Ledger::open(dir.path().join("ledger.ndjson")).unwrap();
/// let s = LedgerSession::new(kp, ledger, "demo".into());
/// assert_eq!(s.session_id(), "demo");
/// ```
pub fn new(...) {...}

/// Atomically allocate the next seq, sign the event against the current
/// parent hash, append to the ledger, and update the parent hash. This is
/// the only sanctioned event emitter for live runs.
///
/// ```
/// use provedex_core::{
///     AgentEvent, Ledger, LedgerSession, SigningKeypair,
/// };
/// let dir = tempfile::tempdir().unwrap();
/// let kp = SigningKeypair::generate();
/// let ledger = Ledger::open(dir.path().join("ledger.ndjson")).unwrap();
/// let s = LedgerSession::new(kp, ledger, "demo".into());
/// let signed = s
///     .seal_and_append(AgentEvent::SessionStarted {
///         agent_id: "a".into(),
///         model_id: "m".into(),
///         session_id: "s".into(),
///     })
///     .unwrap();
/// assert_eq!(signed.seq, 0);
/// ```
pub fn seal_and_append(...) {...}
```

- [ ] **Step 4: Re-run full CI gate**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
cargo deny check
```

Expected: all green. Doctest count goes from 2 to 4 (or higher).

- [ ] **Step 5: Commit doctests**

```bash
git add crates/provedex-core/src/session.rs
git commit -m "docs(core): add doctests for LedgerSession public API"
```

---

## Task 7: Open PR

**Files:** none modified.

- [ ] **Step 1: Push final state**

```bash
git push
```

- [ ] **Step 2: Open PR**

```bash
gh pr create --base main --head refactor/extract-ledger-session \
  --title "refactor(core): extract LedgerSession from provedex-server" \
  --body "$(cat <<EOF
## Summary

Extracts the seal_and_append primitive (seq counter + parent_hash mutex + ledger append + signing) from provedex-server's AppState into a new LedgerSession type in provedex-core. Sets up the upcoming provedex-agent sidecar (issue #11) to share the same primitive without duplicating crypto state.

## What changed

- Added \`provedex-core::session::LedgerSession\` with \`seal_and_append\`, doctests, and 4 unit tests.
- provedex-server's \`AppState\` now wraps a \`LedgerSession\` and adds the broadcast channel for SSE.
- Routes use new accessor methods (\`state.session_id()\`, \`state.pubkey_hex()\`, \`state.ledger()\`).

## Test plan

- [x] \`cargo test --workspace --all-features\` green (27 tests, was 23)
- [x] \`cargo clippy --workspace --all-targets --all-features -- -D warnings\` clean
- [x] \`cargo fmt --check\` clean
- [x] \`cargo audit\` + \`cargo deny check\` clean
- [x] Manual smoke: server boots, healthz returns 200, tamper-test breaks chain at the expected seq

Refs ADR 0004. Unblocks #11.
EOF
)"
```

- [ ] **Step 3: Wait for CI green**

```bash
gh run watch --exit-status
```

Expected: ci workflow passes (fmt + clippy + test + supply-chain).

---

## Task 8: Self-review on the PR + merge

**Files:** none modified.

- [ ] **Step 1: Read the PR diff one more time as if you wrote nothing**

```bash
gh pr diff
```

- [ ] **Step 2: Confidence check (95% bar)**

Self-questions:
- Could anything break the chain across a server restart? (Addressed by `LedgerSession::new` resuming from `ledger.read_all().last()`.)
- Could two threads call `seal_and_append` concurrently and produce out-of-order seq? (No: `fetch_add` allocates seq, then mutex serializes the parent_hash + signing + append.)
- Are all routes ported to the new accessors? (Yes: healthz, events, verify, export, tamper.)
- Does the broadcast channel still emit only after the ledger write? (Yes: `seal_and_append` on the server wraps `LedgerSession::seal_and_append` then calls `self.broadcast.send`.)
- Public API: does anything that was `pub` before become non-`pub`? (No: AppState fields are still pub, just wrap a LedgerSession.)

If any "no" or "uncertain", do NOT merge; fix first.

- [ ] **Step 3: Merge**

```bash
gh pr merge --squash --delete-branch
git checkout main
git pull
```

Expected: branch deleted, main has the squashed commit, working tree clean.

- [ ] **Step 4: Mark issue closure**

```bash
# Tag the PR as unblocking #11
gh issue comment 11 --body "PR for the LedgerSession refactor merged. Unblocks the sidecar phase 1 work."
```

---

## Self-review (writer's pass on this plan)

Spec coverage:
- Refactor primitive into core: covered (Tasks 2-3).
- Server still works: covered (Task 4 + Task 5 smoke).
- No drift between server and future agent: covered (LedgerSession is the single source of truth).
- Public API discipline (rustdoc + doctest): covered (Task 6 step 3).
- Self-review and 95% confidence bar: covered (Task 6 + Task 8 step 2).
- PR with code review before merge: covered (Tasks 7 + 8).
- CI gate and supply chain: covered (Task 6 step 4).

Placeholder scan: none of the patterns from the skill's "No Placeholders" list appear.

Type consistency: `LedgerSession::new(SigningKeypair, Ledger, String) -> Self`, `seal_and_append(&self, AgentEvent) -> Result<SignedEvent, SessionError>`, `session_id() -> &str`, `pubkey_hex() -> String`, `ledger() -> &Ledger`. AppState mirrors the same surface where it needs to.

No gaps found. Plan ready for execution.
