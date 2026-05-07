# docs/ - reference documentation

Four sections, four purposes. Pick the right one before writing.

## Sections

| Folder | Purpose | Audience |
|--------|---------|----------|
| `spec/` | Byte-level normative documents. Anything a third-party implementation needs to reproduce. | Binding implementers, future Provedex engineers |
| `adr/` | Architecture decision records. Why we chose X over Y. | Future engineers, reviewers |
| `integration/` | Framework-specific how-to guides. | Customer engineers |
| `compliance/` | Regulator clause-to-event mappings. | Compliance officers, auditors |

## Spec rules

- Specs are normative. Clients implementing them must match byte-for-byte.
- Filename includes a version: `event-schema-v1.md`, `canonical-json-v1.md`. Never edit a v1 file in place once it has shipped binding code; cut a v2 instead and bump `ExportBundle::schema_version`.
- Every spec has a "Test vectors" section with concrete input/output byte sequences a binding can paste into a unit test.

## ADR rules

- Filename: `NNNN-kebab-title.md`, monotonically numbered.
- Once merged, never renumber, never silently rewrite.
- To change a decision, write a new ADR with `Status: accepted, supersedes 0007` and update the old one's status to `superseded by NNNN`.
- Template:
  ```
  # NNNN. Title

  Date: YYYY-MM-DD
  Status: proposed | accepted | superseded by NNNN

  ## Context
  ## Decision
  ## Consequences
  ```
- Required ADRs we owe early: canonical JSON format, hash chain shape, keypair scope (per-session vs per-agent), NDJSON over binary format.

## Integration guide rules

- One guide per framework or runtime: LangChain, Letta, FastAPI, Express, voice-agent stacks.
- Every guide ends with a "Verify" section: how the customer proves their integration emits real signed events that pass `provedex verify`.

## Compliance doc rules

- One file per regulation: `eu-ai-act.md`, `hipaa.md`, `finra-recordkeeping.md`, `nist-ai-rmf.md`.
- Each clause we map to gets a heading with the exact clause text quoted.
- Below each clause: a) what Provedex emits, b) what is still on the customer to do.
- Never claim Provedex makes a customer compliant. We provide the evidence layer; humans certify compliance.

## Style

- Plain ASCII. No em dashes. No emojis.
- Specs use ABNF / pseudocode for byte-level rules.
- ADRs use plain prose plus the four-section template.
- Integration guides include copy-pasteable code blocks.
- Compliance docs link back to ADRs and specs by relative path.
