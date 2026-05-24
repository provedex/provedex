# 0006. Post-quantum signature migration path

Date: 2026-05-24
Status: proposed

## Context

Provedex v0.1 signs every event with Ed25519 over a SHA-256 digest of canonical-JSON-encoded payload bytes. Ed25519 is the right primitive for 2026: standardized (RFC 8032), small (64-byte signature, 32-byte public key), fast (microseconds per verify), widely audited, and the de-facto default for transparency logs (Sigstore Rekor, Sigsum, certificate transparency monitors).

Two competitors lean on post-quantum cryptography as a wedge:

- asqav ships ML-DSA-65 (NIST FIPS 204, August 2024) as their default signature scheme. Their pitch line is "quantum-safe audit trails."
- Aira layers RFC 3161 trusted timestamps over Ed25519 and pitches the combination as future-proof for long regulatory retention.

The buyer of Provedex (CISO, compliance officer, risk officer at a regulated company) sees evidence packets that must remain verifiable for 6 to 10 years:

- EU AI Act Article 12: high-risk AI deployments must retain audit logs for at least 6 months, in practice 6 to 10 years depending on the use case.
- FINRA Rule 17a-4: 6 years for broker-dealer books and records.
- HIPAA: 6 years for audit records of access to protected health information.
- NSA CNSA 2.0: mandatory post-quantum migration for US federal national-security systems by 2035.

That buyer asks three questions, in order:

1. What happens to these signatures when a cryptographically relevant quantum computer (CRQC) breaks Ed25519?
2. Do you have a migration path?
3. Can I verify a 2026 receipt with 2032 tooling?

Without a written answer, the wedge pitch costs the room. The literal answer (the threat is not imminent, Ed25519 is still standard, NIST has standardized the successor) is correct but unconvincing without commitment.

The threat model is well understood. Public cryptographic consensus as of 2026: no CRQC exists, Shor's algorithm against curve25519 would require thousands of logical qubits with very long coherence times, and the most aggressive published forecasts place CRQC arrival in the 2030s at the earliest. The realistic risk before that horizon is harvest-now-decrypt-later (HNDL) against confidentiality of stored ciphertext, not against signature non-repudiation. Provedex receipts are public-key-verifiable and store no encrypted secrets, so HNDL against a Provedex ledger does not buy the attacker anything: they cannot forge a 2026 signature without the private key, regardless of future quantum capability. The only quantum risk to Provedex is post-CRQC forgery of future signatures, which is mitigated by switching the signing scheme before CRQC arrives.

## Decision

Ship Ed25519 today. Commit to a documented migration path to a hybrid Ed25519 plus ML-DSA-65 mode behind a feature flag, with eventual transition to ML-DSA-65 as the default once the field consolidates. Maintain backward verifiability of every Ed25519 receipt ever produced.

Concrete commitments:

- `event-schema-v2` will extend the `signature` field of `SignedEvent` to a discriminated union:
  - `Ed25519Signature` (the current v1 shape, unchanged on the wire)
  - `Hybrid` carrying both an Ed25519 signature and an ML-DSA-65 signature over the same canonical bytes
  - `MlDsa65Signature` for ML-DSA-65 only deployments
- The verifier in `provedex-core` and `provedex-cli verify` will accept all three variants. Existing v1 receipts continue to verify forever.
- `provedex-agent` will gain a `--signature-scheme=ed25519|hybrid|ml-dsa-65` flag. Default stays `ed25519`. Operators with HNDL concerns about long-horizon ledger retention can opt into `hybrid` today.
- In hybrid mode, the agent signs the same canonical-JSON bytes with both schemes and stores both signatures. The verifier accepts the event if both signatures validate by default, with a `--verify-policy=any|all` flag to allow either-or for transition periods.

Indicative customer-facing migration guidance (illustrative, not a date commitment):

- 2026 to 2028: Ed25519 default, hybrid available for HNDL-sensitive operators.
- 2028 to 2032: hybrid default, Ed25519 still accepted for backward verification.
- 2032 onward: ML-DSA-65 default, hybrid still accepted, Ed25519-only verification remains supported for legacy ledgers.

These windows recalibrate whenever NIST publishes guidance updates, a CRQC milestone shifts, or a customer-driven need pulls the schedule forward.

## Why not hybrid today by default

- Ed25519-only signatures are still the standard expected by every current auditor and every existing transparency-log stack (Sigstore Rekor, Sigsum, certificate transparency, GPG-style signed git tags).
- ML-DSA-65 signatures are 3293 bytes versus Ed25519's 64 bytes (51x larger). For a customer at 1000 events per second in hybrid mode, the per-second storage delta is roughly 3 MB. Most voice agents in the target wedge emit fewer than 10 events per second and the cost is negligible. For high-volume telemetry pipelines it is not.
- ML-DSA-65 verification is meaningfully slower than Ed25519 (rough rule of thumb: 4x verify time, more on architectures without dedicated SIMD). On the hot path this matters.
- The threat (CRQC capable of breaking curve25519) does not exist in 2026 and is not credibly forecast to exist before the 2032 retention horizon for receipts produced today.
- Optionality is the asymmetric play. Customers who want it today opt in. Customers who do not pay the cost defer until the threat is closer.

## Why not ML-DSA-65 only today

- Loses interoperability with every transparency-log and signing-primitive stack in production in 2026.
- Every auditor on Earth in 2026 understands Ed25519 by default; ML-DSA-65 is in the standards but not yet in the auditor playbook.
- 3293-byte signatures encoded as hex in NDJSON make per-line records 6 KB plus payload, which makes operator inspection (`jq`, `tail -f`, manual `cat`) measurably worse.

## Alternatives considered

- **Option A: Ed25519-only today, retroactive migration on demand.** Cleanest, but leaves the wedge unaddressed. The buyer hears "we have not thought about it." Rejected on positioning grounds.
- **Option B: Hybrid by default today.** Future-proofs every receipt. Pays the 51x signature size cost on every operator regardless of their threat model. Forces the auditor playbook update before the field is ready. Rejected on cost grounds.
- **Option C: ML-DSA-65 only today.** Resolves the wedge by overcommitting. Sacrifices interoperability with the entire current verifier ecosystem. Rejected on interoperability grounds.
- **Option D (chosen): Ed25519 default with documented hybrid mode behind a flag, future-default migration on a published roadmap.** Closes the wedge in conversations without forcing today-cost on customers who do not need it.

## Consequences

What this commits Provedex to:

- Maintain backward verifiability of all v1 (Ed25519-only) receipts forever, even after the default migrates.
- Publish a normative `event-schema-v2.md` defining the hybrid signature wire format before any hybrid-mode code ships. Bump `ExportBundle::schema_version` from 1 to 2 at that point.
- Track NIST PQC standardization updates. If NIST deprecates ML-DSA-65 or promotes a successor (HQC, FALCON, or a newer ML-DSA variant), update this ADR with a successor record and ship a corresponding scheme variant.
- Keep the verifier as a pure library so any customer or auditor can verify offline without a vendor round trip. This is the offline-verifiability moat and post-quantum migration must not break it.

What this leaves room for:

- A future CRQC milestone or customer-driven need can pull the hybrid-default switchover earlier than 2028. The ADR commits to the migration shape, not the timing.
- A customer that wants ML-DSA-65 only (for example a US federal customer aligning with CNSA 2.0) can be served with the same agent binary by setting `--signature-scheme=ml-dsa-65` once that flag ships.

What this does not commit to:

- Specific dates for hybrid default switchover.
- A specific successor algorithm if NIST deprecates ML-DSA-65.
- A claim that Provedex is "quantum-safe" today. The product is Ed25519-signed in v0.1 and that statement holds until hybrid mode ships.

## Implementation timing

- Feature-flagged hybrid mode targeted for v0.3.x (rough, no date committed). The exact release version depends on whether a design partner pulls the work forward.
- First design partner that asks for PQ signatures schedules the implementation. Until then this ADR is the answer to the wedge question; no engineering work is in flight.

## References

- NIST FIPS 204 (Module-Lattice-Based Digital Signature Standard, August 2024): https://csrc.nist.gov/pubs/fips/204/final
- NIST CNSA 2.0 (Commercial National Security Algorithm Suite 2.0): https://media.defense.gov/2022/Sep/07/2003071834/-1/-1/0/CSA_CNSA_2.0_ALGORITHMS_.PDF
- NSA Cybersecurity Advisory on CNSA 2.0 timeline: software and firmware signing must migrate by 2030, other categories by 2035.
- RFC 8032 (Ed25519, current scheme).
- ADR 0002 (hash chain shape, signature scheme location in the record).
- ADR 0001 (canonical JSON, signed payload encoding).
- Sigstore project conventions (Rekor transparency log, current Ed25519 + ECDSA defaults).
- Issue tracking the implementation: filed alongside this ADR.
