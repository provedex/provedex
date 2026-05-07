# Release process

This is the checklist for publishing a `provedex-core` release to crates.io and tagging the workspace. Run it from `main` after the change set is merged and CI is green.

`provedex-cli` and `provedex-server` may publish later; for now the only crates.io target is `provedex-core`.

## Pre-flight

1. On `main`, fully synced.

   ```
   git checkout main
   git pull
   git status
   ```

   Working tree must be clean.

2. CI must be green on the latest commit.

   ```
   gh run list --limit 1
   ```

3. Local checks must pass.

   ```
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace --all-features
   cargo audit
   cargo deny check
   ```

4. Version is settled. Pre-1.0 minor bumps may break (per ADR 0001 and 0002). Post-1.0, breaking changes require a major bump and an ADR.

## Decide the version

Pre-1.0 (current state):

- Patch (0.1.0 -> 0.1.1): bug fixes, no API change.
- Minor (0.1.0 -> 0.2.0): any change, including breaking, since pre-1.0.

Post-1.0 (future state):

- Patch: bug fix, fully backward-compatible.
- Minor: additive API change, no breaks.
- Major: any breaking change. Requires a "breaking" ADR.

## Bump the version

In `crates/provedex-core/Cargo.toml`, update `version` (or rely on `version.workspace = true` and bump `Cargo.toml` workspace package version).

```
cargo set-version --package provedex-core <NEW_VERSION>
```

`cargo-set-version` is part of `cargo-edit`. If not installed:

```
cargo install --locked cargo-edit
```

## Update CHANGELOG.md

If `CHANGELOG.md` does not exist, create it. Format: Keep a Changelog (https://keepachangelog.com).

```
## [0.2.0] - 2026-MM-DD

### Added
- New event variant: ToolReturned latency breakdown.

### Changed
- Canonical-JSON now allows ...

### Fixed
- ...

### Breaking
- Bumped ExportBundle::schema_version from 1 to 2 because ...
```

Reference the ADRs that justify breaking changes.

## Dry run

```
cargo publish --dry-run --package provedex-core
```

This compiles a release and verifies the package would upload cleanly. Catches missing fields in `Cargo.toml`, oversized files, and license issues before they hit crates.io.

## Publish

```
cargo publish --package provedex-core
```

`provedex-core` lands on crates.io. The version is permanent; you cannot overwrite a published version.

## Tag the release

```
git add Cargo.toml CHANGELOG.md
git commit -m "chore(release): provedex-core <VERSION>"
git push
git tag -a v<VERSION> -m "provedex-core <VERSION>"
git push --tags
```

## Cut a GitHub release

```
gh release create v<VERSION> --title "provedex-core <VERSION>" \
  --notes-file <(awk '/^## \['<VERSION>'\]/,/^## \[/{print}' CHANGELOG.md | head -n -1)
```

Or open the GitHub UI, paste the relevant CHANGELOG section.

## Post-release verification

```
cargo install --version <VERSION> provedex-cli
provedex --help
```

Verifies the published binary actually runs from a clean machine. If anything is broken, yank the version with `cargo yank`.

```
cargo yank --version <VERSION> --package provedex-core
```

A yank does not remove the version; it stops new dependents from picking it up. Existing dependents keep their lockfile pin.

## When something goes wrong

- Build fails after `cargo publish`: the version is still permanent. Bump and fix in the next release. Do not try to re-upload the same version.
- Critical bug found within an hour: yank, then publish a fixed `<VERSION>+1` with a CHANGELOG entry referencing the yanked version.
- Wrong files published: yank, regenerate `Cargo.toml` `include`/`exclude` patterns, publish a new version.

## Things this doc deliberately leaves out

- `provedex-cli` and `provedex-server` publish process. Either follow the same pattern when ready, or keep them un-published if they remain demo-only. Decide with an ADR before the first attempt.
- Python and Node binding releases. Each binding has its own publish surface (PyPI, npm) and its own checklist (lives in `bindings/python/RELEASING.md` and `bindings/node/RELEASING.md` once the bindings exist).
- Hosted aggregator deployment. Different shape (containers, not packages). Will live in `deploy/RELEASING.md` if/when we ship a hosted service.
