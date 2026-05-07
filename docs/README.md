# Provedex docs

Reference documentation for the Provedex audit-ledger primitives.

## Layout

- `spec/` - byte-level specifications. Anything a third-party implementation needs to reproduce. Event schema, canonical-JSON format, signature scheme, ledger format. Versioned in filename (`event-schema-v1.md`).
- `adr/` - architecture decision records. Numbered, immutable once merged. Use `NNNN-kebab-title.md`.
- `integration/` - guides for plugging Provedex into a customer stack. LangChain, Letta, Python apps, voice agent frameworks.
- `compliance/` - regulator-side mappings. EU AI Act Article 12, HIPAA, FINRA, NIST AI RMF.

## Conventions

- Plain ASCII. No em dashes. No emojis.
- Specs are normative: clients implementing them must match byte-for-byte.
- ADRs document why, not how. Code documents how.
- Compliance docs name specific clauses they map to (e.g. "EU AI Act Art 12(2)(a)").
