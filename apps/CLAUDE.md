# apps/ - deployable end-user apps

Distinct from `crates/` (libraries and reusable services) and `bindings/` (FFI). Each subfolder is one app.

## Today

- `demo-web/` - vanilla HTML/JS/CSS single-page UI for the live voice scribe demo. Served by `crates/provedex-server/`. See `apps/demo-web/CLAUDE.md` for design tokens.

## Planned

- `dashboard-web/` - operator dashboard for the hosted aggregator (post-funding).
- `regulator-portal/` - read-only audit portal regulators can be invited to.

## Naming

- App folder names: `<role>-<surface>` where surface is `web`, `cli`, `tui`, `desktop`. Examples: `demo-web`, `dashboard-web`, `admin-cli`.
- No `provedex-` prefix on app folders (the prefix is for Rust crates).

## Conventions

- Every app has a `README.md` and a `CLAUDE.md` in its own folder.
- Frontend apps are vanilla until proven otherwise. No SPA framework unless an ADR justifies it.
- Backend services are Rust crates under `crates/`, not apps. An "app" is a deployable unit; a "service" is a library plus its bin.
