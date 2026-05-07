# Signature scheme (v1)

Status: normative.

This document specifies the cryptographic primitives that bind a `SignedEvent` to its emitter. Every Provedex client implementation (Rust core, sidecar agent, future bindings, third-party verifiers) must produce signatures that satisfy this spec exactly. A binding implementation can be tested for conformance against the byte-level test vectors at the end of this document.

References:

- ADR 0002 (hash chain shape).
- `docs/spec/canonical-json.md` (the deterministic JSON encoding hashed before signing).
- `docs/spec/event-schema-v1.md` (the AgentEvent JSON shape signed over).

## Scope

This spec covers:

1. Keypair file format on disk.
2. The exact bytes that get signed.
3. Signature and public-key encoding rules.
4. Verification algorithm.

This spec does NOT cover keypair distribution, key rotation, or transparency-log anchoring. Those land in future ADRs and specs.

## Algorithm

Provedex uses Ed25519 (RFC 8032) over a SHA-256 (FIPS 180-4) digest of canonical-JSON-encoded event data. The SHA-256 digest is signed directly as the message; we do not use Ed25519ph or any pre-hash variant. The Rust reference uses the `ed25519-dalek` 2.x crate with default deterministic-nonce signing.

## Keypair on disk

- Path (default): `~/.provedex/keys/ed25519.key`.
- Override via env: `PROVEDEX_KEY=<path>`.
- File contents: exactly 32 raw bytes. No header, no trailer, no PEM wrapper, no base64. The bytes are an Ed25519 secret key (the seed; the public key is derived).
- Permissions: `0600` on Unix (owner read/write only). The Rust reference enforces this on save via `chmod 0600`.
- Public key: derived from the secret on load. Never written to a separate file; pubkey appears in every emitted `SignedEvent` as `signer_pubkey`.

A binding that loads a keypair must read exactly 32 bytes from the path and reject files of any other length.

## What gets signed

Given:

- `seq: u64`
- `timestamp_nanos: u64`
- `event: AgentEvent` (per `event-schema-v1.md`)
- `parent_hash: String` (64 lowercase hex characters)

Build the JSON value:

```
{
  "event": <serde_json::to_value(event)>,
  "parent_hash": "<parent_hash hex>",
  "seq": <seq>,
  "timestamp_nanos": <timestamp_nanos>
}
```

(The map literally contains four keys. Canonical-JSON sorts them alphabetically before encoding, so the byte stream emits `event`, `parent_hash`, `seq`, `timestamp_nanos` in that order.)

Encode the value with `canonical_json` (per `docs/spec/canonical-json.md`). Compute SHA-256 of the encoded bytes. The 32-byte raw digest is `self_hash`. Hex-encode the digest into 64 lowercase hex characters; that string lives in `SignedEvent::self_hash`.

The signature is computed by Ed25519 over the **raw 32-byte digest** (not the hex string, not the canonical-JSON bytes). The signing input is exactly those 32 bytes. The Rust reference at `crates/provedex-core/src/signed.rs` implements this in `compute_self_hash` plus `keypair.sign(&hash_bytes)`.

## Signature encoding

- Ed25519 produces a 64-byte signature.
- Encode as 128 lowercase hex characters.
- Stored in `SignedEvent::signature` as a string.

## Public key encoding

- Ed25519 public keys are 32 bytes.
- Encode as 64 lowercase hex characters.
- Stored in `SignedEvent::signer_pubkey` as a string.

## Verification algorithm

Given a `SignedEvent` and the expected pubkey of the signer:

1. Decode `signer_pubkey` from hex into 32 raw bytes. Reject if the hex is malformed or the byte length is not 32.
2. Decode `signature` from hex into 64 raw bytes. Reject if malformed or wrong length.
3. Recompute `self_hash` from `(seq, timestamp_nanos, event, parent_hash)` using the canonical-JSON + SHA-256 procedure above. Reject if the recomputed hex does not equal `SignedEvent::self_hash`.
4. Call Ed25519 verify with: pubkey, the 32 raw bytes of the recomputed digest, and the 64 raw bytes of the signature. Reject on verification failure.
5. If all four steps pass, the event is authentic.

The Rust reference is `provedex_core::SignedEvent::verify_self`. A binding's verify implementation must produce the same accept/reject decision for any input.

## Test vectors

Reproduce via:

```
cargo run -p provedex-core --example print_test_vectors
```

The signature vectors below use a fixed 32-byte seed so they are deterministic across runs and across implementations. A binding implementation MUST produce the same `self_hash`, `pubkey`, and `signature` for the same input.

### Fixed keypair

- Seed (hex, 32 bytes): `000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f`
- Public key (hex, 32 bytes): `03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8`

Place 32 raw bytes of the seed (NOT the hex string) on disk to test the load path. The pubkey is derived from the seed at load time; it does not need to be stored.

### Vector S1: SessionStarted at seq 0

Inputs:

- `seq`: 0
- `timestamp_nanos`: 1700000000000000000
- `parent_hash`: `0000000000000000000000000000000000000000000000000000000000000000`
- `event`:

```
{
  "type": "SessionStarted",
  "payload": {
    "agent_id": "agent-1",
    "model_id": "llama3.2:3b",
    "session_id": "session-demo"
  }
}
```

Expected outputs:

- `self_hash` (hex): `52299f4b70c1603526deb7b88cbaa03ca9eb1b91a3da038babb4a175041aec91`
- `signature` (hex, 128 chars): `ca2390a43c403510ba49c48814a57858df033b75fba3c027610b49afab555fd0e05cfd2ae37d4fb25cb0aee08954aab89a1fbb94f1dc8617cc57185c9275e90d`

### Vector S2: ModelInvoked at seq 1, non-genesis parent

Inputs:

- `seq`: 1
- `timestamp_nanos`: 1700000000500000000
- `parent_hash`: `abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789`
- `event`:

```
{
  "type": "ModelInvoked",
  "payload": {
    "model_id": "gpt-4o",
    "prompt_sha256": "aaaa1111bbbb2222cccc3333dddd4444eeee5555ffff6666aaaa1111bbbb2222",
    "response_sha256": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "prompt_tokens": 482,
    "response_tokens": 71
  }
}
```

Expected outputs:

- `self_hash` (hex): `19b20c09efa53c9dce162f474ea0cbb7b65889900fdbad8f4752e011483d1325`
- `signature` (hex, 128 chars): `e774e8d5f6696a8b6240ba99f663ddcae260103ea11c8a2189979c664219fa59283de9c7fe0042ba560609d50c12478f9801605459e16068fae825c6d177a20d`

## Versioning

This is signature-scheme v1. Any change to the algorithm (e.g. switching from Ed25519 to a different signature scheme), the signing input (e.g. signing the canonical-JSON bytes directly instead of the SHA-256 digest), the keypair file format, or the encoding rules requires:

- A new file `docs/spec/signature-scheme-v2.md`.
- A new ADR superseding ADR 0002.
- A bump of `ExportBundle::schema_version`.
- Coordinated upgrade across all bindings.

The rules in this v1 spec are frozen. Edits to fix typos or clarify ambiguity are permitted; edits that change the signing input or the algorithm are not.
