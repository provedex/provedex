# 0005. Open-core licensing: open primitive, proprietary operations layer

Date: 2026-05-07
Status: accepted

## Context

Provedex sells trust. The buyer (a CISO, a compliance officer, a risk officer at a regulated company) must read the code that signs their events before trusting it with audit-grade evidence. Closed-source crypto is dead in the post-Delve compliance market: vendors that ask the buyer to "trust us, the screenshot is real" are exactly the failure mode Provedex replaces.

At the same time, this is not a charity. The hosted aggregator (multi-tenant ingest, retention, search, regulator-export, SIEM forwarders, dashboards) is a real operational business and the right place to make money. The aggregator cannot tamper with signed events; the math handles integrity. So the aggregator can be closed without compromising the trust pitch.

We considered three licensing shapes:

1. **Fully open source** (Apache-2.0 everywhere). Maximum trust + adoption + community. Monetization weaker; service offerings compete with hyperscaler hosted versions. Sentry-style.
2. **Open primitive + proprietary operations** (open core). Apache-2.0 on the SDK and signing primitive; proprietary on the hosted aggregator and enterprise add-ons. HashiCorp Vault, GitLab, MongoDB, Sentry-on-prem all use this shape.
3. **Source-available with anti-cloud restrictions** (BSL, SSPL, Commons Clause). Looks open, blocks competing hosted offerings. Compliance procurement teams reject these; OSI does not consider them open source.

## Decision

Open core.

### What stays Apache-2.0 forever

The trust primitive. Anything the customer's signing key touches, anything that defines chain integrity, anything an auditor reads:

- `provedex-core` (signing primitives, hash chain, NDJSON ledger, canonical JSON, export bundle).
- `provedex-cli` (operator tool).
- `provedex-agent` (sidecar HTTP signing daemon).
- Future native bindings: `bindings/python/`, `bindings/node/`, and any other language wrapper that holds a customer key.
- All normative specs under `docs/spec/`.
- All ADRs under `docs/adr/`.
- Reference voice-agent demo (`provedex-server`) and demo UI (`apps/demo-web/`).

License: Apache-2.0. Patent grant matters: protects users from contributor patent claims. Aligned with the Sigstore ecosystem (Cosign, Rekor, Fulcio are all Apache-2.0).

### What ships under a proprietary commercial license

The operations layer. Built post-funding, in a separate private repo:

- Hosted aggregator service (multi-tenant ingest, storage, retention, search, regulator-export packets).
- Aggregator dashboard / control plane (operator UX).
- Multi-tenant isolation engine.
- Premium SIEM forwarders (Splunk HEC, Datadog logs, Elastic).
- BYOC (bring-your-own-cloud) deploy orchestrator for enterprise on-prem.
- Customer-success automation.

These ship under a standard commercial proprietary license (terms TBD; the structure follows post-incorporation legal review).

### Hard rules

- **No SSPL, no BSL, no Commons Clause** on any open-source component. These licenses look open but block competing hosted offerings; they fail enterprise procurement reviews and split the open-source community.
- **No dual-licensing** of the open parts. The open crates are Apache-2.0 only. We will not offer a "commercial license" of provedex-core that strips the patent grant; that is the Mongo SSPL drama in different clothes.
- **No moving an existing open component to closed.** The Apache-2.0 licensed code shipped in v0.1 stays Apache-2.0 forever. This protects users and contributors who depend on the license terms. If a future feature must be closed, it ships in a new proprietary component, not by relicensing an existing one.
- **No closed crypto code.** Anything that touches the customer's signing key or computes a `self_hash` ships under Apache-2.0. This is non-negotiable; the trust pitch requires it.

### Contribution policy

Open parts use the Developer Certificate of Origin (DCO) sign-off, not a CLA. DCO is what the Linux kernel, the Sigstore project, GitLab, and Docker use. Sign-off is a single line in the commit message:

```
Signed-off-by: Aditya Suresh <adityasuresh22@gmail.com>
```

DCO has no entity transfer of rights; contributors retain copyright and license under the project's open license. CLA would block external contributions for the small marginal benefit of consolidated copyright. We pick DCO.

Per `CLAUDE.md`, no AI-generated co-author trailers. DCO sign-off lines are allowed.

## Consequences

- A CISO can read every line of code that signs their events. The trust pitch holds.
- A future enterprise customer pays for hosted aggregator + multi-tenant ops + SIEM integration. Money lives there.
- The open primitive becomes a sales channel: customers `git clone`, run `cargo audit`, read the code, then sign the contract for the hosted tier.
- A competitor cannot legally relicense our open parts to a non-OSI license; Apache-2.0 is irrevocable for the versions we have shipped.
- We cannot compete with AWS by re-licensing under SSPL the way Mongo did. We accept this. The aggregator value is operations + multi-tenant isolation + customer success, which is hard to replicate without the team. If a hyperscaler ever builds a hosted Provedex, we still own the spec, the open agent, and the brand.
- Open contributions land via DCO sign-off PRs. No CLA paperwork, no rights transfer, no friction for first-time contributors.

## What this does not change

- `crates/provedex-core/Cargo.toml`, `crates/provedex-cli/Cargo.toml`, and `crates/provedex-agent/Cargo.toml` declare `license = "Apache-2.0"` via the workspace package. The demo crate (`provedex-server`) lives at `provedex/demo-voice` since 2026-05-08 and carries the same license. This ADR ratifies that, does not modify it.
- The `LICENSE` file at the repo root is the Apache-2.0 license text. No change.
- No future code goes under a license other than Apache-2.0 in this repo. Proprietary code lives in a separate private repo (`provedex/aggregator` or similar) when it lands.

## Update 2026-05-08

The voice-agent demo (`provedex-server`) and demo UI (`apps/demo-web/`) were extracted from this repository into a sibling repo, [`provedex/demo-voice`](https://github.com/provedex/demo-voice). The license is unchanged (Apache-2.0 forever) and the demo continues to consume `provedex-core` against the published `v0.1.0` tag. This is a relocation, not a relicensing; the open-core thesis in this ADR is unaffected.

## References

- Sigstore licensing model: Cosign, Rekor, Fulcio are Apache-2.0; the Public Good instance is operated as a Linux Foundation project.
- HashiCorp Vault open-core split: OSS Community + Enterprise (paid).
- Mongo SSPL relicensing (2018): cautionary tale of relicensing an open component to closed; we explicitly reject this path.
- BSL discussion in the OSS community (Hashicorp, Sentry, others): treated as source-available, not open source; rejected by enterprise procurement.
- Linux Kernel DCO: precedent for our contribution policy.
