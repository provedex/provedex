# Canonical JSON encoding (v1)

Status: normative.

This document specifies the byte-level canonical JSON encoding used by Provedex for hashing and signing. Every Provedex client implementation (Rust core, Python binding, Node binding, hosted aggregator, third-party verifier) must produce byte-identical output for the same input. A signed event's `self_hash` is the SHA-256 of the canonical-JSON encoding of the event's hashed-field map; if two implementations diverge here, signatures will not verify cross-implementation.

The decision to roll a custom canonical JSON encoder rather than reuse RFC 8785 (JCS) is recorded in ADR 0001.

## Scope

Canonical JSON applies to:

- The four-field map hashed by `compute_self_hash` (see ADR 0002).
- Any other Provedex artifact that needs reproducible byte output (export bundles, future Rekor anchoring payloads).

Canonical JSON does NOT apply to the on-the-wire NDJSON ledger lines, which use ordinary `serde_json` output. Verification recomputes canonical JSON from the parsed event, so the wire format does not need to be canonical.

## Encoding rules

### Top level

The encoder accepts a JSON value (string, number, boolean, null, array, or object) and emits a UTF-8 byte string with no surrounding whitespace.

### Whitespace

There is none. No spaces between tokens. No newlines. No leading or trailing whitespace.

### `null`

Emitted as the four ASCII bytes `null`.

### Booleans

`true` is emitted as the four ASCII bytes `true`. `false` is emitted as the five ASCII bytes `false`.

### Numbers

Numbers in the Provedex value space are unsigned 64-bit integers (sequence numbers, timestamps, token counts) and (rarely) signed integers. Float, NaN, and Infinity are NOT supported.

An integer is emitted as its decimal representation in ASCII, with no leading sign for non-negative values, no leading zeros (except a literal zero), and no trailing decimal point.

Implementations that internally represent numbers as 64-bit floats must round-trip through an integer type before encoding; if a value cannot be represented as a 64-bit signed or unsigned integer, encoding fails.

### Strings

Strings are UTF-8. The encoder emits a leading `"`, then escapes the following characters and only the following characters, then emits a trailing `"`:

| Char | Emitted as |
|------|-----------|
| U+0022 quotation mark | `\"` |
| U+005C reverse solidus | `\\` |
| U+000A line feed | `\n` |
| U+000D carriage return | `\r` |
| U+0009 horizontal tab | `\t` |
| U+0008 backspace | `\b` |
| U+000C form feed | `\f` |
| U+0000 through U+001F (other control) | `\u00xx` (lowercase hex) |

All other code points are passed through as their UTF-8 bytes. The forward slash `/` is NOT escaped. Code points outside the Basic Multilingual Plane are emitted as their direct UTF-8 byte sequence; surrogate-pair `\uHHHH\uHHHH` notation is NOT used.

### Arrays

An array is emitted as `[`, then each element in order separated by `,`, then `]`. No trailing comma. No whitespace.

### Objects

An object is emitted as `{`, then each `key:value` pair separated by `,`, then `}`. Keys must be sorted in ascending lexicographic order of their UTF-8 byte representations before emission. Duplicate keys are not permitted; if the input has duplicates, encoding fails (callers should deduplicate before encoding).

Each pair emits the canonical-JSON encoding of the key (which is always a string), then `:`, then the canonical-JSON encoding of the value. No whitespace anywhere.

## Implementation reference

The Rust reference implementation lives at `crates/provedex-core/src/signed.rs::canonical_json`. The function takes a `serde_json::Value` and returns `Vec<u8>`. The behavior matches this spec exactly.

A binding implementation must produce byte-identical output to the Rust reference for every input in the supported value space. The cross-binding test suite at `tests/compat/` (when added) enforces this.

## Test vectors

Each vector lists the input as ordinary JSON, the canonical-JSON output as bytes, and the SHA-256 of those bytes (lowercase hex). A binding implementation MUST produce the same output bytes and SHA-256 for the same input.

These vectors are reproducible by running:

```
cargo run -p provedex-core --example print_test_vectors
```

### Vector 1: object key sort

Input:

```
{"b":1,"a":2,"c":[3,2,1]}
```

Canonical bytes:

```
{"a":2,"b":1,"c":[3,2,1]}
```

Length: 25 bytes.
SHA-256: `bbf618dc23e53236ec7ba96c7d4d0b6e1d660943b309478a037cd97263de21d9`.

### Vector 2: control character escapes

Input (string `line1\nline2\t"end"` inside a key `k`):

```
{"k":"line1\nline2\t\"end\""}
```

Canonical bytes:

```
{"k":"line1\nline2\t\"end\""}
```

Length: 29 bytes.
SHA-256: `e9037e62a8378f7eb73152b7be2a02fa02d5d9f1fc541f8e8d1cf0a08d5b4e2d`.

### Vector 3: nested with key sort at every level

Input:

```
{"session_id":"abc","events":[{"type":"x","n":1},{"type":"y","n":2}]}
```

Canonical bytes (note inner object keys also sorted):

```
{"events":[{"n":1,"type":"x"},{"n":2,"type":"y"}],"session_id":"abc"}
```

Length: 69 bytes.
SHA-256: `3a600a3be5273cf0a7e21dc79a7d52d10f4983c1dda2ab85bc85d740bdbd64e1`.

### Vector 4: empty containers and null

Input:

```
{"empty_arr":[],"empty_obj":{},"null_field":null}
```

Canonical bytes:

```
{"empty_arr":[],"empty_obj":{},"null_field":null}
```

Length: 49 bytes.
SHA-256: `d58a42ea8dcb1fb350c66d28a603406696f389283f727fd37adcb671521c46e4`.

### Vector 5: unicode passthrough

Input:

```
{"name":"Aditya"}
```

Canonical bytes:

```
{"name":"Aditya"}
```

Length: 17 bytes.
SHA-256: `ffc15f3d482ee939e18cce266ec0b3f3f7d17ca42d806bfabb6c12512cc3624f`.

### Vector 6: number ranges

Input (u64 max and zero):

```
{"u":18446744073709551615,"z":0}
```

Canonical bytes:

```
{"u":18446744073709551615,"z":0}
```

Length: 32 bytes.
SHA-256: `c90b95423f8c6cfc6d6de892e0286eea742f64ef93833f69e7047dc95289d714`.

## Versioning

This document specifies canonical-JSON v1. Any change to the encoding rules requires:

- A new file `docs/spec/canonical-json-v2.md` (do not edit this v1 file in place once shipped).
- A new ADR superseding 0001.
- A bump of `ExportBundle::schema_version`.
- Coordinated upgrade across all bindings; the byte-compat test suite runs against the spec version implied by the schema_version field.

The encoding rules in this v1 spec are frozen. Edits to fix typos or clarify ambiguity are permitted; edits that change byte output are not.
