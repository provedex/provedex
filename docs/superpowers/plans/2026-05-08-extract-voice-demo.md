# Extract voice-agent demo to provedex/demo-voice

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans.

**Goal:** Extract `crates/provedex-server/` and `apps/demo-web/` from the main repo into a new public repo `provedex/demo-voice` with git history preserved. Main repo becomes SDK + tooling only. Demo consumes the published SDK via git tag `v0.1.0`.

**Architecture:** Two parallel changes:

1. New repo `provedex/demo-voice` (public, Apache-2.0) created via `git filter-repo --path crates/provedex-server --path apps/demo-web`. Demo's `Cargo.toml` switches `provedex-core` from `path = "../provedex-core"` to `git = "https://github.com/provedex/provedex", tag = "v0.1.0"`. Demo gets its own root README, CLAUDE.md, .gitignore, license, voice-aditya symlink.
2. Main repo branch removes the two paths, prunes workspace `members`, prunes workspace `dependencies` that became demo-only (`whisper-rs`, `hound`, `tokio-stream`, `bytes`, `base64`, `which`, `futures`), updates README, CLAUDE.md, integration docs.

**Tech stack:** `git-filter-repo` (installed), gh CLI for repo creation.

## Pre-flight

- Branch: `refactor/extract-voice-demo` (created off main).
- Issue: #35.
- Tag: `v0.1.0` already exists; demo will pin to it.

## File Structure

**New repo (provedex/demo-voice):**

```
crates/
  provedex-server/        (moved from provedex/provedex)
apps/
  demo-web/               (moved)
README.md                 (new, demo-specific)
CLAUDE.md                 (new, gitignored)
.gitignore                (new)
LICENSE                   (Apache-2.0, copy of main repo's)
Cargo.toml                (workspace root, single member)
```

**Main repo, removed:**
- `crates/provedex-server/`
- `apps/demo-web/`
- `apps/CLAUDE.md` (gitignored anyway)

**Main repo, modified:**
- `Cargo.toml` (workspace) - members shrinks 4 -> 3; demo-only deps pruned.
- `README.md` - "Components" table 4 -> 3 rows; remove "Voice agent reference" section, replace with link to demo repo.
- `CLAUDE.md` (gitignored) - remove provedex-server + apps from navigation + where-files-go tables.
- `crates/CLAUDE.md` (gitignored) - remove provedex-server bullet.
- `docs/integration/sidecar.md` - mention demo repo as the working integration.
- Probably: nothing in `RELEASING.md` since release.yml never built provedex-server.

## Tasks

### Task 1: branch + plan + push

- [x] Branch created.
- [ ] Commit plan, push.

### Task 2: create demo repo via filter-repo

```
cd /tmp
git clone --no-local https://github.com/provedex/provedex provedex-fork
cd provedex-fork
git filter-repo --path crates/provedex-server --path apps/demo-web --force
```

Verify result:
- `git log --oneline | head -5` shows commits that touched those paths.
- `ls` shows only `crates/`, `apps/`.

### Task 3: scaffold demo repo

In `/tmp/provedex-fork`:

- Add new `Cargo.toml` workspace root (single member: `crates/provedex-server`).
- Edit `crates/provedex-server/Cargo.toml`: replace `provedex-core = { path = "../provedex-core", version = "0.1.0" }` with `provedex-core = { git = "https://github.com/provedex/provedex", tag = "v0.1.0" }`.
- Add `LICENSE` (Apache-2.0, copied byte-identical from main repo).
- Add `README.md` (new, see below).
- Add `.gitignore` (claude.md, agents.md, .claude, target, .DS_Store, .env).
- Symlink AGENTS.md -> CLAUDE.md (after CLAUDE.md added; CLAUDE.md is gitignored).
- Symlink `.claude/skills/voice-aditya` -> `~/.claude/skills/voice-aditya`.

README content for demo repo:

```
# provedex/demo-voice

Reference voice-agent integration on top of the Provedex SDK. Records audio, transcribes via whisper.cpp, calls a local Ollama model, signs every step with the public Provedex primitive, optionally speaks the reply via Piper.

This is a working dogfood of `provedex-core` v0.1.0 from https://github.com/provedex/provedex.

[Run instructions, deps, screenshots]
```

CLAUDE.md (gitignored): conventions for the demo repo, voice-aditya skill ref, frontend-design + css + ts skills inherited from user-global.

### Task 4: cargo build the demo repo locally

```
cd /tmp/provedex-fork
cargo build --workspace
```

Must succeed pulling provedex-core from the v0.1.0 tag. If it fails, the published v0.1.0 tag has a regression; abort and fix in main repo first.

### Task 5: gh repo create + push demo

```
gh repo create provedex/demo-voice --public --source=/tmp/provedex-fork --remote=origin --push
```

Verify URL, license badge.

### Task 6: prune main repo

In `/Users/adi/Desktop/provedex` on `refactor/extract-voice-demo`:

- `git rm -r crates/provedex-server apps/demo-web apps/CLAUDE.md` (the apps/CLAUDE.md is gitignored anyway; just rm).
- Edit `Cargo.toml` workspace.members: drop `"crates/provedex-server"`.
- Edit `Cargo.toml` workspace.dependencies: remove `whisper-rs`, `hound`, `tokio-stream`, `which`, `futures`, `base64`. Keep deps still used by core/cli/agent (axum, tower, tower-http, tokio, reqwest used elsewhere? - check via grep before removal).

Verify which deps are exclusively used by provedex-server before removing from workspace:

```
for dep in whisper-rs hound tokio-stream which futures base64 reqwest bytes; do
  count=$(grep -l "$dep" crates/{provedex-core,provedex-cli,provedex-agent}/Cargo.toml 2>/dev/null | wc -l | tr -d ' ')
  echo "$dep: still used in $count surviving crates"
done
```

Only remove from workspace.dependencies those with count = 0.

- Update README.md:
  - Components table: drop the provedex-server row.
  - Quickstart-Rust crate: keep.
  - "Voice agent reference (optional)" section: replace with a one-line "see provedex/demo-voice for a working voice-agent integration that consumes this SDK".
  - Repository layout: drop apps/, drop provedex-server line.
- Update root CLAUDE.md (gitignored): drop provedex-server + apps rows from navigation + where-files-go.
- Update crates/CLAUDE.md (gitignored): drop provedex-server bullet.
- Update docs/integration/sidecar.md: append a section "see also: provedex/demo-voice for a full voice-agent integration".

### Task 7: cargo build + full CI gate

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
cargo deny check
```

All five must pass.

### Task 8: self-review with code-review-provedex skill

- Auto-block invariants: file deletes are inside the documented layout, no canonical-JSON change, no schema bump, conventional commits, ascii.
- Workspace-deps removed from `[workspace.dependencies]` only after grep-verified they are exclusively in provedex-server.
- Cargo.lock will shrink considerably; commit it.

### Task 9: PR + merge

- voice-aditya register PR body, link issue #35, link demo repo URL.
- Wait CI green.
- Auto-merge.
- Close #35.

## Self-review (writer's pass)

Risk: workspace-dep prune. Easy to remove a dep that another crate still uses. Mitigation: grep-verify (Task 6) before removing each.

Risk: integration tests in `tests/` that hit provedex-server. Mitigation: scan `tests/` for `provedex_server` references before delete.

Risk: docs/integration mentioning provedex-server commands. Mitigation: scan and update in same PR.

Risk: demo repo build fails because v0.1.0 tag references something demo needs that was post-tag (unlikely - demo paths were stable). Mitigation: Task 4 build is the gate; if it fails, abort.

Out-of-scope items deferred: renaming binaries, rebuilding the demo UI, releasing the demo as a binary.
