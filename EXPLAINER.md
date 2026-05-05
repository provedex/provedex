# Provedex - the simple version

This document is for someone who just heard the word "Provedex" and wants to know what it is, who would use it, and why it does not exist yet. The first half assumes you are not technical. The second half adds the engineering picture and the system-design diagrams.

## 1. What is it

Provedex is a black box flight recorder for AI agents.

When an airplane crashes, investigators pull out a sealed box that recorded everything the plane did in the last two hours. Nobody can edit that box. The data inside is the truth.

Provedex is the same idea for AI. When an AI does work in a regulated industry (a hospital, a bank, a law firm, an insurance company), it makes decisions that affect real people. If something goes wrong, somebody is going to ask: what did the AI actually do, and can you prove it?

Provedex sits next to the AI and writes a log of every step the AI took. The log is signed with cryptographic signatures, like a chain of receipts that link to each other. If anyone tries to change a single line later, the chain visibly breaks. Anyone can check the chain by running a small program. There is no vendor in the middle who has to be trusted.

That is the whole product. A signed, tamper-evident log of AI behavior.

## 2. A concrete example

Imagine a hospital that uses an AI voice scribe during patient visits.

- Patient walks in and tells the doctor about chest pain.
- Doctor and AI both listen. AI transcribes the conversation.
- AI suggests a medication dose.
- Doctor accepts. AI writes the visit note into the chart.

Six months later, the patient sues. They claim the AI told the doctor to give the wrong dose, which made them sicker.

Without Provedex, the hospital has plain log files. The plaintiff lawyer points out that those log files could have been edited at any time after the fact. There is no way to prove they are the original record. The hospital is now defending whether their log file is real, before they can even defend whether the dose was right.

With Provedex, the hospital exports a signed bundle. The lawyer (or a court-appointed expert) runs `provedex verify`. Math either says the chain is intact or it does not. If intact, the log is the original record at the original timestamps, signed by the hospital's key. If broken, somebody tampered, and the lawyer needs to ask why.

The same pattern applies to a bank's voice agent for loan eligibility, an insurance call center handling claims, a law firm running an AI intake bot. Different industry, same need: a record nobody can fake.

## 3. What happens today vs what Provedex adds

### Today

Most AI agent systems write events to ordinary log files or databases.

- The application writes whatever it wants into the log.
- The same application can edit or delete those entries later.
- Backups exist, but anyone with admin access to the database can edit a backup too.
- Compliance vendors collect "evidence" via screenshots and form-filling. Most of that evidence is itself unsigned and could be fabricated.
- Standard operating system audit logs (Linux audit daemon, AWS CloudTrail) capture what the OS or cloud did, not what the AI agent reasoned through.

Result: when a regulator or a court asks for proof, the company produces logs that the company itself wrote. Trust is anchored in the company's word.

### What Provedex adds

Three things go into every event the moment it is emitted:

1. A digital signature, made with a key that is not under the application's control.
2. A fingerprint (hash) of the previous event in the chain.
3. A timestamp.

Result: the log signs itself as it is being written. Nobody can re-sign an edit without the private key, and nobody can edit a single event without breaking the fingerprint chain. Anyone with the public key can verify the whole log later, on their own laptop, with no help from the company that produced it.

That last sentence is the part that matters. The buyer is not paying for the log file. The buyer is paying for the property that they can hand the log to a hostile third party and that third party can independently confirm the log is real.

## 4. Who would actually pay for this

The buyer is not the AI engineer. It is the person who gets fined or sued when AI goes wrong.

| Buyer role | Industry example | Why they care |
|------------|------------------|---------------|
| Hospital CISO / compliance officer | Big health system using AI scribes (Abridge, Suki, Nabla, Microsoft DAX) | HIPAA + state malpractice. AI dose errors generate lawsuits. |
| Bank CCO / model risk officer | Voice agent for loan eligibility, fraud-flagging | OCC / CFPB exam wants reproducible evidence of every model decision. |
| Insurance compliance | AI handling first-notice-of-loss calls | State insurance regulators audit claims handling. |
| Law firm partner | AI document review or intake bot | Bar association rules on confidentiality + court evidence rules. |
| EU enterprise risk officer | Anything "high-risk" under EU AI Act Article 12 | Aug 2 2026 deadline. Fines up to 15M EUR or 3% of global revenue. |
| Government contractor | AI used in any federal contract | NIST AI RMF + agency-specific audit clauses. |

Notice: the buyer is not in the AI org chart. The buyer is in legal, compliance, or risk. The AI engineer is the integrator, not the customer.

## 5. Why this does not exist yet

Five reasons, in plain terms.

1. The regulation that forces it is months away. EU AI Act Article 12 enforcement begins August 2, 2026. Until very recently there was no concrete deadline that compliance officers could point to in budget meetings.

2. The category is awkward. It sits in the gap between observability (Datadog, Splunk), compliance SaaS (Vanta, Drata, Delve), and cryptography (Sigstore, transparency logs). Nobody in any of those camps thinks they own this problem.

3. The skill set is rare. To build it, you need someone who can write production cryptography in Rust, ship distributed-systems infrastructure, and also understand voice agents and the regulated buyer. Most observability vendors do not have crypto people. Most crypto people are not in observability.

4. Big SaaS does not love open source primitives. The right shape of this product is a small open-source crate plus a paid hosted aggregator. Big SaaS vendors prefer closed boxes that lock the buyer in. So the obvious incumbent (a Datadog) does not move first.

5. AI agents themselves are too new. Two years ago, "what did the AI agent do" was not a real production question because there were no production AI agents at scale. The category is appearing right alongside the agents.

## 6. How it is different from things that sound similar

| Category | What it does | Why it is not Provedex |
|----------|--------------|------------------------|
| Datadog / Splunk / OpenTelemetry | Centralized logging, metrics, alerts | Logs are mutable. Vendor controls storage. No cryptographic integrity. |
| AWS CloudTrail, Linux auditd | Records OS and cloud control-plane events | Records the OS, not the agent's reasoning. No third-party-verifiable signature. |
| Vanta / Drata / Delve | Compliance workflow SaaS (collect screenshots, automate audit prep) | Trust-based: vendor swears the evidence is real. Recent fraud cases show why this matters. Provedex removes the vendor from the trust path. |
| Sentrial AI, Hamming, Coval | AI agent observability and quality testing | Tells you what the agent did. Does not prove it cryptographically. Different buyer (eng lead, not compliance). |
| Mem0 / Letta / Zep / Cognee | Agent memory infrastructure | Different primitive: recall vs evidence. They make agents remember; we make their actions provable. |
| Keycard, Auth0 for AI | Agent identity and authorization | Who the agent is allowed to act as. Provedex records what they actually did. Adjacent layer. |
| Sigstore / Rekor | Code-signing transparency log for software supply chain | Same family of cryptography. Different scope: software artifacts at build time, not agent events at runtime. We borrow primitives, not the system. |
| Public blockchains | Append-only ledgers | Too slow, too expensive, and over-engineered for high-volume agent events. We use the same hash-chain idea without the consensus overhead. |
| ELK / Loki / Grafana | Log ingest and search | Search and visualization. No integrity guarantee. |

The short way to position it: Datadog tells you what your AI agent did. Provedex proves it.

## 7. Where it lives on a business's infrastructure

Two deployment shapes. Same primitive in both.

### 7a. Self-hosted (open source crate)

The customer runs the open-source Rust crate inside their own application. The signed ledger lives on their own disk or object store. They never send data to Provedex. They run `provedex verify` themselves.

```
+-----------------------------------------------------------------+
|  Customer infrastructure (their VPC / their data center)        |
|                                                                 |
|  +-------------+   emits events   +---------------------+       |
|  | AI agent    | ---------------> | provedex-core (SDK) |       |
|  | (voice,     |                  |  Rust / Python / TS |       |
|  |  text,      |                  +---------------------+       |
|  |  tool use)  |                            |                   |
|  +-------------+                            | sign + chain      |
|                                             v                   |
|                                  +---------------------+        |
|                                  | NDJSON ledger file  |        |
|                                  | (local disk / S3 /  |        |
|                                  |  GCS / Azure Blob)  |        |
|                                  +---------------------+        |
|                                             |                   |
|                                  +---------------------+        |
|                                  | Customer's existing |        |
|                                  | SIEM / log pipeline |        |
|                                  | (forward optional)  |        |
|                                  +---------------------+        |
|                                             |                   |
|                                             v                   |
|                                  +---------------------+        |
|                                  | provedex CLI        |        |
|                                  | verify / replay /   |        |
|                                  | export              |        |
|                                  +---------------------+        |
|                                                                 |
+-----------------------------------------------------------------+

                                              |
                                              v
                                  +---------------------+
                                  | Auditor / regulator |
                                  | runs provedex       |
                                  | verify on a copy    |
                                  | of the bundle       |
                                  +---------------------+
```

Trust model: the customer's signing key never leaves the customer. Anyone with the public key can verify offline. Provedex the company is not in the trust path.

This is what big banks, defense contractors, and healthcare systems with strict data-residency rules want. They cannot send raw audio or model traces out of their network.

### 7b. Hosted aggregator (paid SaaS)

For customers who do not want to operate the storage layer themselves, Provedex offers a hosted aggregator. The SDK still signs locally; only signed events go upstream. The hosted side adds storage, search, retention policy, multi-tenant isolation, and a verification API that auditors can hit.

```
+--------------------------------------------+      +-------------------------------+
|  Customer application                      |      |  Provedex hosted (cloud)      |
|                                            |      |                               |
|  +---------------+    +-------------+      |      |  +-------------------------+  |
|  | AI agent      |--> | provedex-   | ---- HTTPS ---> | Aggregator ingest       |  |
|  | (voice/text)  |    | core SDK    |      |      |  | (verifies signatures)   |  |
|  +---------------+    | sign locally|      |      |  +-------------------------+  |
|                       +-------------+      |      |             |                 |
|         (private key never leaves customer)|      |             v                 |
|                                            |      |  +-------------------------+  |
|                                            |      |  | Long-term store         |  |
|                                            |      |  | (object storage,        |  |
|                                            |      |  |  per-tenant isolated)   |  |
|                                            |      |  +-------------------------+  |
+--------------------------------------------+      |             |                 |
                                                    |             v                 |
                                                    |  +-------------------------+  |
                                                    |  | Verification API        |<-+- Auditor / regulator
                                                    |  | (read-only, signed)     |     pulls bundle
                                                    |  +-------------------------+
                                                    |             |                 |
                                                    |             v                 |
                                                    |  +-------------------------+  |
                                                    |  | Optional: SIEM forwarder|  |
                                                    |  | (Splunk, Datadog,       |  |
                                                    |  |  Elastic, Sentinel)     |  |
                                                    |  +-------------------------+  |
                                                    +-------------------------------+
```

Trust model: still no trust required for log integrity, because every event was already signed by the customer's key before it left their network. The hosted side is just storage and search; if it tampered, the chain breaks and anyone can see it. The hosted side adds operational convenience (retention, search, SLA), not crypto.

This is what regulated startups and mid-market SaaS companies want. They do not have an internal SecOps team to operate the storage layer themselves.

## 8. The cryptographic part, in plain terms

Two ideas. That is all.

### Idea 1: digital signatures

A digital signature is the math version of signing a check. The signer has two keys: a private one nobody else sees, and a public one anyone can have. The private key produces signatures; the public key verifies them. You cannot forge a signature without the private key. You cannot edit the signed thing without the signature stopping verifying.

Provedex uses Ed25519. Each event is signed with the customer's private key the moment it is emitted. Anyone with the public key can later check: this event was emitted by this signer.

### Idea 2: hash chain

A hash is a fingerprint of a piece of data. Same data goes in, same fingerprint comes out. Change a single byte, the fingerprint changes completely.

Each Provedex event includes the fingerprint of the previous event. So event 5 has the fingerprint of event 4 inside it. Event 6 has the fingerprint of event 5. And so on. This is called a hash chain.

If somebody edits event 4, its fingerprint changes. But event 5 still has the old fingerprint of event 4 stored inside it. So event 5 no longer verifies. Verification stops at event 4 with a clear "broken at sequence 4" message.

To tamper successfully, an attacker would have to re-sign every event from the tampered point forward. They cannot, because they do not have the private key.

### Why this is enough

A signed hash chain gives three properties for free:

- Integrity: any edit is detectable.
- Authenticity: the events came from the holder of the private key.
- Order: events cannot be silently reordered, because each one references the previous.

Those three properties are what a regulator, auditor, or court is asking for when they ask "is this log real". They are also what compliance SaaS today does not give you.

## 9. What is not in scope for Provedex

To keep the picture honest:

- Provedex does not store the AI's training data. That is not its job.
- Provedex does not stop the AI from being wrong. It only proves what the AI did.
- Provedex does not redact PHI or PII for you. The customer decides what to put in events. We provide the signing and chaining, not the privacy policy.
- Provedex does not replace your SIEM or your observability vendor. It sits beside them and feeds them signed evidence.
- Provedex does not decide whether your AI is compliant. A human auditor still has to read the export.

## 10. The thought process in one paragraph

AI agents are about to make decisions in regulated industries at scale. Regulators will demand proof, and that proof has to survive an adversarial review. Today's logging and compliance tools assume the company writing the log is trusted to write the truth. That assumption fails the moment a lawsuit, a fine, or a fraud allegation lands. The right answer is the same answer the software supply chain landed on a decade ago: cryptographic primitives, hash chains, public verifiability, open-source crate at the bottom, paid managed offering on top. We are doing for AI agent runtime what Sigstore did for code provenance. The forcing function is the EU AI Act, but the buyer logic outlives any single regulation.

That is why Provedex exists.
