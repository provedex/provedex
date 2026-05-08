# OpenAPI 3 Spec Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans.

**Goal:** Publish an OpenAPI 3 spec for the agent HTTP API at `docs/spec/openapi.yaml`. Generate it from utoipa annotations on the route handlers + types so the spec stays in sync with code. Add a CLI flag `provedex-agent --print-openapi` that emits the spec.

**Architecture:** `utoipa` is workspace dep. `provedex-core` adds an `openapi` feature that pulls utoipa as optional and decorates AgentEvent / SignedEvent / ChainReport with `ToSchema`. `provedex-agent` enables the feature unconditionally and adds `ToSchema` on its own types (Health, SignRequest), `utoipa::path` on each route, and a `utoipa::OpenApi` derive struct that aggregates the routes.

**Tech Stack:** `utoipa` 5.x, `serde_yaml` for serializing the spec.

---

## Pre-flight

- Branch: `feat/openapi-spec` (created off main).
- Issue: #31.

## File Structure

**Create:**
- `crates/provedex-agent/src/openapi.rs` - `ApiDoc` struct with `utoipa::OpenApi` derive.
- `docs/spec/openapi.yaml` - generated spec, committed.
- `docs/superpowers/plans/2026-05-07-openapi-spec.md` (this file).

**Modify:**
- `Cargo.toml` (workspace) - add `utoipa` + `serde_yaml`.
- `crates/provedex-core/Cargo.toml` - optional `utoipa` dep + `openapi` feature.
- `crates/provedex-core/src/event.rs`, `signed.rs`, `chain.rs` - add `#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]`.
- `crates/provedex-agent/Cargo.toml` - depend on provedex-core with `openapi` feature; add utoipa + serde_yaml.
- `crates/provedex-agent/src/lib.rs` - re-export openapi module.
- `crates/provedex-agent/src/main.rs` - add `--print-openapi` flag.
- `crates/provedex-agent/src/routes/{healthz,sign,verify}.rs` - add `#[utoipa::path(...)]` + `ToSchema` on local types.
- `README.md` - link to openapi.yaml under "Specs".
- `docs/integration/sidecar.md` - note OpenAPI spec is the authoritative contract.

## Tasks

### Task 1: branch + plan + push

- [ ] Stage and commit the plan, push branch.

### Task 2: workspace + crate Cargo manifests

In root Cargo.toml workspace.dependencies:

```toml
utoipa = { version = "5.4", features = ["yaml"] }
serde_yaml = "0.9"
```

In crates/provedex-core/Cargo.toml:

```toml
[dependencies]
utoipa = { workspace = true, optional = true }

[features]
openapi = ["dep:utoipa"]
```

In crates/provedex-agent/Cargo.toml:

```toml
[dependencies]
provedex-core = { path = "../provedex-core", version = "0.1.0", features = ["openapi"] }
utoipa = { workspace = true }
serde_yaml = { workspace = true }
```

Verify cargo check builds clean.

Commit: `chore(agent): add utoipa workspace dep + openapi feature on core`.

### Task 3: ToSchema derives on provedex-core types

In `crates/provedex-core/src/event.rs`:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type", content = "payload")]
pub enum AgentEvent { ... }
```

Same pattern on `SignedEvent` (signed.rs) and `ChainReport` + `ChainStatus` (chain.rs).

`cargo build -p provedex-core --features openapi` must build clean.

Commit: `feat(core): ToSchema derives behind openapi feature`.

### Task 4: ToSchema on agent local types + utoipa::path on routes

In `routes/healthz.rs`:

```rust
#[derive(Serialize, utoipa::ToSchema)]
pub struct Health { ... }

#[utoipa::path(
    get,
    path = "/v1/healthz",
    responses(
        (status = 200, description = "Agent is healthy and ledger is writable", body = Health),
        (status = 503, description = "Agent is degraded; ledger is not writable", body = Health),
    ),
    tag = "agent",
)]
pub async fn healthz(...) -> ... { ... }
```

Same for sign + verify. Add ToSchema on SignRequest.

`cargo build -p provedex-agent` must build clean.

Commit: `feat(agent): utoipa::path annotations on /v1/* routes`.

### Task 5: ApiDoc + --print-openapi flag

`crates/provedex-agent/src/openapi.rs`:

```rust
use utoipa::OpenApi;

use provedex_core::{AgentEvent, ChainReport, ChainStatus, SignedEvent};

use crate::routes::{healthz::Health, sign::SignRequest};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "provedex-agent",
        version = env!("CARGO_PKG_VERSION"),
        description = "Provedex sidecar HTTP signing daemon. Customer applications POST signed events to /v1/sign; ...",
        license(name = "Apache-2.0", identifier = "Apache-2.0"),
    ),
    paths(
        crate::routes::healthz::healthz,
        crate::routes::sign::sign,
        crate::routes::verify::verify,
    ),
    components(schemas(
        Health,
        SignRequest,
        AgentEvent,
        SignedEvent,
        ChainReport,
        ChainStatus,
    )),
    tags(
        (name = "agent", description = "Sidecar HTTP signing daemon endpoints"),
    ),
)]
pub struct ApiDoc;
```

In `lib.rs`: `pub mod openapi;`.

In `main.rs`, add `--print-openapi` flag that prints the spec as YAML and exits 0:

```rust
#[arg(long)]
print_openapi: bool,

// in main, before tracing init:
if args.print_openapi {
    let spec = ApiDoc::openapi();
    println!("{}", spec.to_yaml().expect("yaml"));
    return Ok(());
}
```

`cargo run -p provedex-agent -- --print-openapi` should print a valid OpenAPI 3 YAML.

Commit: `feat(agent): ApiDoc + --print-openapi CLI flag`.

### Task 6: generate docs/spec/openapi.yaml + commit

```
cargo run -p provedex-agent --release -- --print-openapi > docs/spec/openapi.yaml
```

Verify the file is non-empty and valid YAML.

Commit: `docs(spec): commit generated openapi.yaml`.

### Task 7: README + sidecar.md updates

- README "Specs" section: add a row pointing to `docs/spec/openapi.yaml`.
- `docs/integration/sidecar.md`: add a leading note that the OpenAPI spec is the authoritative contract; the prose examples are illustrative.

Commit: `docs: link OpenAPI spec from README and sidecar.md`.

### Task 8: self-review using code-review-provedex skill

- ASCII grep across all modified files (note: openapi.yaml is generated and may include non-ASCII if any annotation does; verify and clean up the source if so).
- AI-slop adjective check.
- Conventional commit subjects.
- New `pub` items: `ApiDoc` (provedex-agent) - rustdoc on the struct briefly noting purpose.
- `provedex-core` ToSchema derives are feature-gated; default-feature build must still work.
- CI gate: cargo fmt + clippy -D warnings + test --all-features + audit + deny.

Fix findings, commit if any.

### Task 9: PR + merge

- gh pr create in voice register.
- Wait for CI green.
- Confidence check (95%).
- Auto-merge.
- Close #31.

## Self-review (writer's pass)

Spec coverage: all 4 issue acceptance criteria mapped to tasks (utoipa dep, ToSchema, utoipa::path, --print-openapi, openapi.yaml committed, README + sidecar.md links).

Placeholder scan: no TBD. Out-of-scope items (swagger-ui, CI drift check) explicitly deferred.

Type consistency: `ApiDoc::openapi()` returns the spec struct; `to_yaml()` from the `yaml` feature flag on utoipa.

No gaps. Ready.
