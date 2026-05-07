# 0001. Canonical JSON encoding for hashed and signed events

Date: 2026-05-07
Status: accepted

## Context

A signed audit ledger is only useful if anyone, in any language, can recompute the hash and verify the signature on a recorded event. JSON is our wire format, but JSON serialization is not deterministic by default. The same logical event can produce different byte streams depending on the producer:

- `serde_json` preserves insertion order in Rust.
- `Python's json` module preserves insertion order.
- `Go's encoding/json` sorts object keys alphabetically.
- Number representations differ across implementations (e.g. trailing zeros, exponent forms).
- Whitespace and key-ordering choices are unconstrained.

If two clients sign the same event but produce different bytes, the signatures will not verify against each other. Cross-implementation interoperability is non-negotiable for a binding ecosystem (Rust core, Python binding, Node binding) and for third-party auditors who must reproduce verification independently.

We considered three options for resolving this:

1. RFC 8785 JSON Canonicalization Scheme (JCS) - widely cited, but mandates a specific number representation that is awkward for the integer-heavy event payloads we emit (everything we sign uses integers, hashes as hex strings, and small enums - no floats).
2. CBOR - a binary canonical encoding. Rejected because we want the ledger to be human-readable for incident response and operator inspection.
3. Roll our own minimal canonical-JSON profile, narrower than JCS but covering exactly the value space the AgentEvent schema uses.

## Decision

We use a custom canonical-JSON encoding implemented in `provedex-core::signed::canonical_json`. The rules are:

- Object keys are sorted in lexicographic byte order before serialization.
- No whitespace anywhere in the output.
- Strings are UTF-8. The escape rules match `serde_json` defaults: backslash escapes for `"`, `\`, `\n`, `\r`, `\t`, `\b`, `\f`, and `\uNNNN` for control characters below 0x20.
- Numbers are serialized via the standard library's `to_string` for the integer types we emit. Floats are not used in the event schema and are not supported by this encoding.
- Booleans are `true` / `false`. Null is `null`.

The byte-level rules are exact and reproducible from any language. A binding implementation that follows them produces the same bytes as the Rust reference for the same input.

## Consequences

- Every binding (Python, Node, Java, Go) must implement this encoding rather than reusing the host language's stdlib JSON serializer. The `tests/compat/` suite enforces byte equality.
- We cannot extend the value space (no floats, no NaN, no Infinity) without bumping the canonical-JSON spec version.
- Adding a new event variant is safe as long as the variant's payload uses only the supported value space (strings, integers, booleans, null, and recursive arrays/objects).
- Changing any rule in this document invalidates every previously-signed event. A change requires a new spec version (`canonical-json-v2.md`), a new `ExportBundle::schema_version`, and a coordinated upgrade across all bindings.
- The encoding is documented normatively in `docs/spec/canonical-json.md` (to be written) for binding implementers and auditors.
