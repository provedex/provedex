# Phase 3a Release Infrastructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ship the release machinery so any future `git tag v*` produces binaries on GitHub Releases, a multi-arch container image on GHCR, and customers have ready-to-use systemd / launchd / Kubernetes sidecar manifests. Then smoke-test by tagging `v0.1.0-rc.1`.

**Architecture:** GitHub Actions workflow on tag-push. Matrix-built binaries (4 platforms) for `provedex-agent` and `provedex-cli`. Multi-stage Containerfile for the agent, built via Docker buildx for `linux/amd64` + `linux/arm64`, pushed to GHCR. Static deploy manifests under `deploy/`.

**Tech Stack:** GitHub Actions, dtolnay/rust-toolchain, taiki-e/upload-rust-binary-action, docker/build-push-action, GHCR, Cargo cross-compilation (no `cross` crate; native builds per-runner).

---

## Pre-flight

- Branch: `feat/release-infra` (created off main).
- Issue: #24.
- ADR: not required; phase 3 is delivery infrastructure, not architectural change.

## File Structure

**Create:**
- `.github/workflows/release.yml` - tag-triggered build + Release + container.
- `crates/provedex-agent/Containerfile` - multi-stage container build.
- `deploy/systemd/provedex-agent.service`
- `deploy/launchd/com.provedex.agent.plist`
- `deploy/k8s/sidecar-example.yaml`
- `deploy/README.md` - what each manifest does.
- `docs/superpowers/plans/2026-05-07-release-infra.md` (this file).

**Modify:**
- `README.md` - add `## Install` section above Quickstart with three install paths (binary, container, source).
- `CLAUDE.md` - status section gets a one-line update reflecting release machinery exists.

## Tasks

### Task 1: branch + plan + push

- [ ] Stage and commit the plan, push branch.

### Task 2: write the deploy manifests

Static files. No code, no runtime risk. Write them first, get them out of the way.

- [ ] `deploy/systemd/provedex-agent.service`:

```
[Unit]
Description=Provedex sidecar HTTP signing daemon
Documentation=https://github.com/provedex/provedex/blob/main/docs/integration/sidecar.md
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/provedex-agent
Restart=on-failure
RestartSec=5

# Operator-controlled paths via env. Override in /etc/default/provedex-agent.
EnvironmentFile=-/etc/default/provedex-agent

# Sandboxing.
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/var/lib/provedex
ProtectHome=true
PrivateTmp=true
PrivateDevices=true
LockPersonality=true
RestrictNamespaces=true
RestrictRealtime=true
SystemCallArchitectures=native

[Install]
WantedBy=multi-user.target
```

- [ ] `deploy/launchd/com.provedex.agent.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.provedex.agent</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/provedex-agent</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/usr/local/var/log/provedex-agent.log</string>
  <key>StandardErrorPath</key>
  <string>/usr/local/var/log/provedex-agent.err</string>
</dict>
</plist>
```

- [ ] `deploy/k8s/sidecar-example.yaml`:

```yaml
# Example: Provedex agent as a sidecar in a customer voice-agent pod.
# Customer's main app POSTs to http://127.0.0.1:8765/v1/sign.
apiVersion: v1
kind: Pod
metadata:
  name: voice-agent-with-provedex
  labels:
    app: voice-agent
spec:
  containers:
    - name: voice-agent
      image: example/customer-voice-agent:latest
      env:
        - name: PROVEDEX_AGENT_URL
          value: http://127.0.0.1:8765/v1/sign
    - name: provedex-agent
      image: ghcr.io/provedex/provedex-agent:latest
      args: ["--listen", "127.0.0.1:8765"]
      ports:
        - containerPort: 8765
          name: provedex
      volumeMounts:
        - name: provedex-data
          mountPath: /var/lib/provedex
        - name: provedex-key
          mountPath: /etc/provedex
          readOnly: true
      env:
        - name: PROVEDEX_LEDGER
          value: /var/lib/provedex/ledger.ndjson
        - name: PROVEDEX_KEY
          value: /etc/provedex/ed25519.key
      resources:
        requests:
          cpu: 50m
          memory: 32Mi
        limits:
          cpu: 250m
          memory: 128Mi
  volumes:
    - name: provedex-data
      persistentVolumeClaim:
        claimName: provedex-ledger
    - name: provedex-key
      secret:
        secretName: provedex-signing-key
        defaultMode: 0400
```

- [ ] `deploy/README.md`:

```markdown
# Deployment manifests

Reference manifests for running `provedex-agent` in common environments. Copy + adapt; do not consider these final production configurations.

## Files

- `systemd/provedex-agent.service` - Linux systemd unit. Sandboxed via `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome=true`. Drops env-file overrides at `/etc/default/provedex-agent`.
- `launchd/com.provedex.agent.plist` - macOS LaunchDaemon. Logs to `/usr/local/var/log/`.
- `k8s/sidecar-example.yaml` - Kubernetes Pod with the agent as a sidecar container alongside a customer voice agent. Demonstrates the localhost-only signing pattern: customer app POSTs to `127.0.0.1:8765/v1/sign`.

## Quick install

### Linux (systemd)

```
sudo install -m 755 ./target/release/provedex-agent /usr/local/bin/
sudo install -m 644 deploy/systemd/provedex-agent.service /etc/systemd/system/
sudo mkdir -p /var/lib/provedex
sudo systemctl daemon-reload
sudo systemctl enable --now provedex-agent
```

### macOS (launchd)

```
sudo install -m 755 ./target/release/provedex-agent /usr/local/bin/
sudo install -m 644 deploy/launchd/com.provedex.agent.plist /Library/LaunchDaemons/
sudo launchctl load /Library/LaunchDaemons/com.provedex.agent.plist
```

### Kubernetes

Adapt `k8s/sidecar-example.yaml` for your customer pod. Provision the `provedex-ledger` PVC and `provedex-signing-key` secret first; the secret is the customer's signing key (32 raw bytes).
```

- [ ] Commit: `feat(deploy): systemd unit, launchd plist, k8s sidecar manifest`.

### Task 3: write the Containerfile

`crates/provedex-agent/Containerfile`:

```dockerfile
# syntax=docker/dockerfile:1.7

# --- builder ---
FROM rust:1.89-slim-bookworm AS builder

WORKDIR /src

# Install build deps for any -sys crates (none today; future-proofing).
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Copy the workspace and build only provedex-agent.
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./
COPY crates ./crates

RUN cargo build --release --locked -p provedex-agent

# --- runtime ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --no-create-home --shell /sbin/nologin provedex \
 && mkdir -p /var/lib/provedex /etc/provedex \
 && chown provedex:provedex /var/lib/provedex /etc/provedex

COPY --from=builder /src/target/release/provedex-agent /usr/local/bin/provedex-agent

USER provedex
EXPOSE 8765

ENV PROVEDEX_LEDGER=/var/lib/provedex/ledger.ndjson \
    PROVEDEX_KEY=/etc/provedex/ed25519.key \
    PROVEDEX_AGENT_LISTEN=0.0.0.0:8765 \
    RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/provedex-agent"]
CMD ["--insecure-allow-public"]
```

Note: the container binds 0.0.0.0 because in K8s + Docker, "loopback" inside a container is an isolated network namespace; the container itself is the trust boundary. The `--insecure-allow-public` flag is appropriate here. Customers must front it with TLS + auth at the proxy / service mesh.

- [ ] Build locally to verify (Apple silicon native): `docker build -f crates/provedex-agent/Containerfile -t provedex-agent:test .`. Expected: clean build.
- [ ] Smoke run: `docker run --rm -p 8765:8765 provedex-agent:test &`; `curl http://127.0.0.1:8765/v1/healthz`; expect 200 with status ok.
- [ ] Commit: `feat(agent): multi-stage Containerfile`.

### Task 4: release workflow

`.github/workflows/release.yml`:

```yaml
name: release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write
  packages: write

env:
  CARGO_TERM_COLOR: always

jobs:
  binaries:
    name: build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: install rust toolchain from rust-toolchain.toml
        run: rustup show active-toolchain || rustup toolchain install

      - name: add target
        run: rustup target add ${{ matrix.target }}

      - name: install cross-build deps (linux aarch64)
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu
          mkdir -p .cargo
          printf '[target.aarch64-unknown-linux-gnu]\nlinker = "aarch64-linux-gnu-gcc"\n' >> .cargo/config.toml

      - name: cargo build provedex-agent
        run: cargo build --release --locked --target ${{ matrix.target }} -p provedex-agent

      - name: cargo build provedex-cli
        run: cargo build --release --locked --target ${{ matrix.target }} -p provedex-cli

      - name: archive
        shell: bash
        run: |
          set -euo pipefail
          VERSION="${GITHUB_REF#refs/tags/}"
          mkdir -p dist
          for bin in provedex-agent provedex-cli; do
            BIN_OUT="target/${{ matrix.target }}/release/${bin}"
            ARCHIVE="dist/${bin}-${VERSION}-${{ matrix.target }}.tar.gz"
            tar -C "$(dirname "$BIN_OUT")" -czf "$ARCHIVE" "$(basename "$BIN_OUT")"
            shasum -a 256 "$ARCHIVE" > "$ARCHIVE.sha256"
          done

      - name: upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.target }}
          path: dist/

  container:
    name: container image
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4

      - name: log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: set up buildx
        uses: docker/setup-buildx-action@v3

      - name: meta
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ghcr.io/provedex/provedex-agent
          tags: |
            type=ref,event=tag
            type=raw,value=latest,enable=${{ !contains(github.ref_name, '-rc') }}

      - name: build + push
        uses: docker/build-push-action@v6
        with:
          context: .
          file: crates/provedex-agent/Containerfile
          platforms: linux/amd64,linux/arm64
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  release:
    name: github release
    runs-on: ubuntu-latest
    needs: [binaries, container]
    steps:
      - uses: actions/checkout@v4

      - name: download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: create release
        uses: softprops/action-gh-release@v2
        with:
          files: dist/*
          generate_release_notes: true
          prerelease: ${{ contains(github.ref_name, '-rc') }}
```

- [ ] Verify the YAML parses (run `yq`/`yamllint` or just push and watch CI).
- [ ] Commit: `ci(release): tag-triggered binaries + multi-arch container + GitHub Release`.

### Task 5: README install section

Insert before the "## Quickstart - sidecar" section.

```markdown
## Install

### Pre-built binary (recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/provedex/provedex/releases) and extract:

```
tar -xzf provedex-agent-vN.N.N-aarch64-apple-darwin.tar.gz
sudo install -m 755 provedex-agent /usr/local/bin/
```

### Container (Kubernetes / Docker)

```
docker pull ghcr.io/provedex/provedex-agent:latest
docker run --rm -p 8765:8765 ghcr.io/provedex/provedex-agent:latest
```

Multi-arch: `linux/amd64`, `linux/arm64`. Customer apps in the same pod / Docker network POST to the agent's `/v1/sign` endpoint.

### From source

```
cargo install --locked --git https://github.com/provedex/provedex --bin provedex-agent
```

### systemd / launchd / Kubernetes manifests

See [`deploy/`](deploy/) for ready-to-adapt manifests.
```

- [ ] Commit: `docs(readme): install section with binary, container, source paths`.

### Task 6: self-review using code-review-provedex skill

- ASCII grep across new files (workflow YAML, Containerfile, manifests, README diff).
- AI-slop adjective check.
- Conventional commit subjects.
- Workflow YAML structure: matrix coverage, permissions, secrets usage.
- Containerfile: non-root user, no leaked secrets, ENTRYPOINT uses array form.
- systemd unit sandboxing fields present.
- launchd plist DOCTYPE present.
- K8s manifest uses Secret volume with `defaultMode: 0400` for the key.
- Run full local CI gate: `cargo fmt --check`, `cargo clippy --all-features -D warnings`, `cargo test --all-features`, `cargo audit`, `cargo deny check`.
- Local container build smoke test: `docker build` clean, `docker run` healthz returns 200.

Fix findings, commit if any.

### Task 7: PR + merge

- gh pr create in voice register, body lists all artifacts, references issue #24.
- Wait for CI green.
- Confidence check (95% bar).
- Auto-merge.

### Task 8: smoke-test the release pipeline

After PR merges:

- [ ] `git tag v0.1.0-rc.1` on main.
- [ ] `git push --tags`.
- [ ] Watch the release workflow run; expect 4 binary archives (8 if both bins) attached to a Pre-release on GitHub, plus `ghcr.io/provedex/provedex-agent:v0.1.0-rc.1` available.
- [ ] Verify each archive extracts and the binary runs `--version` correctly on the host.
- [ ] Verify `docker pull ghcr.io/provedex/provedex-agent:v0.1.0-rc.1 && docker run --rm` works.
- [ ] If anything fails, file a follow-up issue and patch the workflow on a new branch + PR. Do not delete the tag (we want the failure on record). Tag `v0.1.0-rc.2` after fix.

This task does NOT cut `v0.1.0`. That waits for separate go-ahead.

## Self-review (writer's pass)

Spec coverage: workflow (Task 4), container (Task 3), deploy manifests (Task 2), README install (Task 5), smoke tag (Task 8). All issue #24 acceptance criteria mapped.

Placeholder scan: none of the patterns from the no-placeholders list. The container build commands assume amd64 host; on arm64 macOS the local docker test needs no modification because Docker Desktop builds the right arch by default.

Type consistency: workflow targets match the README install snippets. Container env defaults match the agent's CLI defaults from main.rs. Deploy manifests reference the correct binary path (`/usr/local/bin/provedex-agent`) consistent with the README install steps.

No gaps. Ready.
