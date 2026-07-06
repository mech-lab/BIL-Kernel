# BIL Kernel

**Pure Rust evidence infrastructure for AXLE-compatible AI assurance bundles.**

BIL Kernel is an open-source Rust-native fork of AXLE-compatible proof infrastructure, rebuilt as an institutional evidence kernel for AI assurance.

AXLE establishes whether a formal reasoning artifact can be checked. BIL Kernel takes the next step: it turns checked reasoning artifacts into deterministic evidence bundles that can be hashed, signed, verified, audited, insured, and legally defended.

BIL stands for:

```text
Banking · Insurance · Legal
```

Those three institutional domains form the regulated base of the BIL model. **AI assurance is the apex**: the trust capability that emerges when financial exposure, risk transfer, and legal accountability are structurally connected.

---

## What BIL Kernel Does

BIL Kernel converts AI-enabled decisions and AXLE-compatible proof outputs into portable institutional evidence bundles.

Today, the repository provides:

* canonical Rust data structures
* a Rust AXLE-compatible client
* a Rust CLI foundation
* canonical JSON serialization
* dual SHA-256 and BLAKE3 hashing
* deterministic Merkle evidence trees
* `.bil` directory bundle creation and inspection
* receipt issuance and signature validation
* JSON and Markdown verification reports
* committed `v0` schemas and bundle specs
* a temporary Python compatibility bridge via `axle bil ...`

The near-term kernel roadmap adds:

* banking, insurance, and legal metadata profiles
* broader CLI tooling for developers and assurance teams

The kernel is designed to answer one institutional question:

> Can this AI-enabled decision survive banking, insurance, legal, audit, and regulatory review as a verifiable record?

---

## Core Thesis

BIL Kernel begins where AXLE ends.

```text
AXLE verifies formal reasoning.
BIL Kernel institutionalizes verified reasoning.
```

AXLE-compatible outputs may prove that a reasoning artifact checked successfully. BIL Kernel packages that result into a durable assurance object: a bundle with provenance, context, controls, risk metadata, legal metadata, hashes, receipts, and verification reports.

In short:

```text
From checked proofs to bankable evidence.
```

---

## Pure Rust Doctrine

BIL Kernel is a Rust-native kernel with a temporary Python compatibility bridge during migration.

The kernel runtime, typed AXLE models, first client surface, and `bil` CLI are implemented in Rust.

```text
No Python runtime dependency for the core kernel.
Temporary in-repo Python bridge during migration.
No dashboard-first architecture.
No opaque vendor lock-in.
```

The legacy Python AXLE package remains in the repository to preserve the upstream CLI and SDK workflow while the Rust kernel becomes the primary runtime.

The core runtime, CLI, schema models, hashing layer, bundle format, receipt generation, verification engine, and report generator are intended to live in Rust.

AXLE compatibility is maintained at the artifact boundary through typed Rust models and import adapters.

BIL Kernel does **not** initially replace Lean or AXLE proof checking. Instead:

```text
AXLE / Lean verifies the proof.
BIL Kernel verifies the evidence bundle.
```

---

## Institutional Model

BIL Kernel is based on a tetrahedral model of institutional trust.

```text
                 AI Assurance
                     /\
                    /  \
                   /    \
                  /      \
          Banking--------Insurance
                  \      /
                   \    /
                    \  /
               Legal Governance
```

The base consists of:

| Domain           | Institutional Function                                      |
| ---------------- | ----------------------------------------------------------- |
| Banking          | Capital exposure, credit workflow, financial infrastructure |
| Insurance        | Risk transfer, underwriting logic, loss protection          |
| Legal Governance | Rights, duties, liability, compliance boundaries            |

The apex is:

| Apex         | Function                                  |
| ------------ | ----------------------------------------- |
| AI Assurance | Verifiable trust for AI-enabled decisions |

BIL Kernel operationalizes the apex by producing institutional evidence bundles.

---

## Institutional Holonomy

The underlying mathematical idea is **institutional holonomy**.

In differential geometry, holonomy describes how an object changes when transported around a path through curved space. BIL Kernel applies that intuition to regulated institutions.

An AI-enabled decision does not remain unchanged as it moves through:

```text
AI system
    ↓
Banking workflow
    ↓
Insurance interpretation
    ↓
Legal governance
    ↓
Audit / regulatory review
    ↓
Assurance artifact
```

It accumulates:

* financial exposure
* actuarial risk
* legal interpretation
* compliance obligations
* control evidence
* audit requirements
* liability boundaries

BIL Kernel measures whether the decision’s evidentiary meaning is preserved across that institutional loop.

Assurance is achieved when the decision returns as a verifiable bundle whose provenance, context, controls, and accountability remain intact.

---

## Artifact Flow

```text
Lean / AXLE Proof Output
        ↓
AXLE-Compatible Rust Type
        ↓
BIL Evidence Record
        ↓
Canonical JSON
        ↓
Merkle Evidence Graph
        ↓
Cryptographic Receipt
        ↓
.bil Bundle
        ↓
Institutional Verification Report
```

---

## What Is a `.bil` Bundle?

A `.bil` bundle is a portable evidence object.

In Phase 1, the canonical format is an unpacked directory ending in `.bil/`.

Today it contains:

```text
<name>.bil/
├── axle.json
├── bundle.json
├── manifest.json
├── merkle.json
└── receipt.json           # optional embedded Phase 2 receipt
```

Future phases may extend the payload set with files such as:

```text
bundle.json
manifest.json
decision.json
proof.json
axle.json
model.json
policy.json
risk.json
legal.json
controls.json
receipt.json
merkle.json
signatures/
attachments/
reports/
```

The bundle preserves the institutional context around an AI-enabled decision.

It is designed to be:

* deterministic
* content-addressed
* tamper-evident
* auditable
* machine-verifiable
* portable across institutions
* suitable for banking, insurance, legal, and regulatory workflows

---

## AXLE Compatibility

BIL Kernel is designed to ingest AXLE-compatible proof artifacts.

Initial compatibility targets include response models such as:

```text
VerifyProofResponse
CheckResponse
Document
ExtractDeclsResponse
ExtractTheoremsResponse
RenameResponse
MergeResponse
Theorem2SorryResponse
Theorem2LemmaResponse
SimplifyTheoremsResponse
RepairProofsResponse
Have2LemmaResponse
Have2SorryResponse
Sorry2LemmaResponse
DisproveResponse
NormalizeResponse
```

These are represented as Rust `serde` types and converted into BIL evidence records.

Example:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Messages {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub infos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyProofResponse {
    pub okay: bool,
    pub content: String,
    pub lean_messages: Messages,
    pub tool_messages: Messages,
    pub failed_declarations: Vec<String>,
    pub timings: BTreeMap<String, u64>,
    pub info: Option<serde_json::Value>,
}
```

---

## Repository Structure

```text
bil-kernel/
├── Cargo.toml
├── crates/
│   ├── bil-axle/              # Implemented Rust AXLE-compatible response models
│   ├── bil-core/              # Implemented evidence-kernel domain types
│   ├── bil-schema/            # Implemented v0 JSON Schema generation
│   ├── bil-hash/              # Implemented canonical JSON and dual digests
│   ├── bil-merkle/            # Implemented deterministic Merkle trees
│   ├── bil-bundle/            # Implemented `.bil` bundle create / inspect
│   ├── bil-receipt/           # Implemented receipt issuance and signature handling
│   ├── bil-verify/            # Implemented bundle and receipt verification
│   ├── bil-report/            # Implemented Markdown verification rendering
│   ├── bil-client/            # Implemented Rust async client for AXLE-compatible APIs
│   └── bil-cli/               # Implemented Rust CLI
├── schemas/v0/                # Committed Phase 1 and Phase 2 schema artifacts
├── specs/                     # Committed bundle, manifest, merkle, receipt, and verification specs
├── axle/                      # Legacy Python AXLE SDK and CLI, including `axle bil ...`
├── docs/                      # Existing Python-oriented documentation during transition
├── tests/                     # Python bridge and legacy AXLE tests
├── .github/workflows/         # Combined Python and Rust CI
├── pyproject.toml
└── README.md
```

---

## Current Crates

### `bil-axle`

Implemented today. Contains the AXLE-compatible response and artifact models as Rust `serde` types.

### `bil-client`

Implemented today. Provides the Rust async client for AXLE-compatible proof APIs, including health checks, environment discovery, and typed endpoint helpers.

### `bil-cli`

Implemented today. Provides the first `bil` commands:

```bash
bil status
bil environments
bil hash ./payload.json --canonical-json
bil bundle create --axle ./verify-proof-response.json --axle-kind verify-proof --out proof.bil
bil bundle inspect ./proof.bil
bil receipt issue ./proof.bil --mode embedded --algorithm ed25519 --private-key ./signing-key.der
bil axle verify-proof ./proof.lean --formal-statement "..." --environment lean-4.28.0
bil axle check ./proof.lean --environment lean-4.28.0
bil axle extract-decls ./proof.lean --environment lean-4.28.0
bil axle normalize ./proof.lean --environment lean-4.28.0
```

The existing Python CLI exposes the same path through:

```bash
axle bil ...
```

---

## Phase 1 Crates

### `bil-core`

Implemented today. Defines `AxleEvidenceRecord`, manifest entries, bundle descriptors, Merkle document types, logical-path normalization, and typed AXLE artifact dispatch.

### `bil-schema`

Implemented today. Generates the committed `schemas/v0/*.schema.json` artifacts from the Rust types that define the bundle surface.

### `bil-hash`

Implemented today. Provides canonical JSON serialization plus dual SHA-256 and BLAKE3 digests for bundle-controlled documents.

### `bil-merkle`

Implemented today. Builds deterministic manifest-backed Merkle trees with stable leaf ordering and odd-node duplication rules.

### `bil-bundle`

Implemented today. Creates and inspects unpacked `.bil/` directories containing `axle.json`, `bundle.json`, `manifest.json`, and `merkle.json`.

### `bil-receipt`

Implemented today. Issues embedded and detached receipts, signs canonical claims, and supports `ed25519`, `ecdsa-p256-sha256`, and `rsa-pss-sha256`.

### `bil-verify`

Implemented today. Verifies bundle integrity, receipt coverage, cryptographic signatures, and optional trust-key matches.

### `bil-report`

Implemented today. Renders Markdown verification reports from the structured JSON verification model.

---

## Planned Crates

### `bil-policy`

Policy and control-boundary evaluation.

### `bil-risk`

Banking, insurance, actuarial, and operational risk metadata.

### `bil-legal`

Legal governance, liability, compliance, and evidentiary metadata.

### `bil-wasm`

Portable verifier target for browser, embedded, and client-side workflows.

---

## CLI Today

```bash
bil status
bil environments
bil hash ./payload.json --canonical-json
bil bundle create --axle ./verify-proof-response.json --axle-kind verify-proof --out proof.bil
bil bundle inspect ./proof.bil --format json
bil bundle inspect ./proof.bil --format markdown
bil receipt issue ./proof.bil --mode embedded --algorithm ed25519 --private-key ./signing-key.der
bil axle verify-proof ./proof.lean --formal-statement "1 = 1" --environment lean-4.28.0
bil axle check ./proof.lean --environment lean-4.28.0
bil axle extract-decls ./proof.lean --environment lean-4.28.0
bil axle normalize ./proof.lean --environment lean-4.28.0 --normalization remove_sections
axle bil status
```

Policy/risk/legal profiles, archive packaging, and richer interop remain roadmap work.

---

## CLI Roadmap

```bash
bil init
bil import axle ./verify-proof-response.json
bil report decision.bil --format markdown
bil report decision.bil --format json
bil report decision.bil --format sarif
```

---

## Example Workflow

Check service health:

```bash
bil status
```

List available environments:

```bash
bil environments
```

Hash a JSON document in canonical mode:

```bash
bil hash ./verify-proof-response.json --canonical-json
```

Create a deterministic Phase 1 `.bil` evidence directory from a typed AXLE payload:

```bash
bil bundle create --axle ./verify-proof-response.json --axle-kind verify-proof --out proof.bil
```

Inspect and verify the bundle:

```bash
bil bundle inspect ./proof.bil
```

Issue an embedded receipt:

```bash
bil receipt issue ./proof.bil --mode embedded --algorithm ed25519 --private-key ./signing-key.der
```

Render the verification report in Markdown:

```bash
bil bundle inspect ./proof.bil --format markdown
```

Verify a proof through the Rust client:

```bash
bil axle verify-proof ./proof.lean --formal-statement "1 = 1" --environment lean-4.28.0
```

Normalize Lean code:

```bash
bil axle normalize ./proof.lean --environment lean-4.28.0 --normalization remove_sections
```

Run the same Rust CLI through the legacy Python entrypoint:

```bash
axle bil axle check ./proof.lean --environment lean-4.28.0
```

---

## Verification Questions

BIL Kernel is designed to answer infrastructure-grade assurance questions:

| Question                                   | Kernel Function                  |
| ------------------------------------------ | -------------------------------- |
| Was the artifact altered?                  | Hash verification                |
| Was the evidence preserved?                | Bundle verification              |
| Was the proof result captured?             | AXLE artifact ingestion          |
| Was provenance preserved?                  | Manifest and metadata validation |
| Was the policy context included?           | Policy profile validation        |
| Was legal context included?                | Legal metadata validation        |
| Was risk context included?                 | Risk metadata validation         |
| Can another party inspect it?              | Portable bundle format           |
| Can the receipt be verified independently? | Cryptographic receipt validation |

---

## Institutional Use Cases

### Banking

* credit decision evidence
* SBA lending workflow assurance
* adverse action record preservation
* vendor AI review
* model risk management
* fraud review audit trails
* portfolio decision evidence

### Insurance

* AI risk underwriting
* professional liability review
* claims evidence reconstruction
* control-quality assessment
* premium/risk adjustment
* loss event documentation

### Legal Governance

* compliance evidence
* contractual accountability
* regulatory examination support
* litigation hold support
* duty-of-care documentation
* adverse action defensibility
* evidentiary preservation

### AI Assurance

* decision provenance
* model context preservation
* prompt/output traceability
* proof artifact preservation
* policy boundary verification
* human review evidence
* tamper-evident audit records

---

## Design Principles

### 1. Rust-Native

The kernel is built in Rust for reliability, portability, performance, and memory safety.

### 2. Deterministic

The same input should produce the same canonical representation and digest.

### 3. Verifiable

Every bundle should be independently inspectable and cryptographically checkable.

### 4. Institutionally Portable

Evidence should move across banks, insurers, auditors, legal teams, regulators, and vendors.

### 5. Minimal Runtime Assumptions

The verifier should not require access to the original AI system, vendor dashboard, or proprietary runtime.

### 6. Artifact First

BIL Kernel treats assurance as an artifact problem before it treats it as a dashboard, workflow, or policy problem.

### 7. Open Infrastructure

The evidence layer for AI assurance should be inspectable, testable, forkable, and extensible.

---

## Non-Goals

BIL Kernel is not:

* a chatbot
* a model provider
* a Lean replacement
* a full theorem prover
* a policy dashboard
* a bank core system
* an insurance rating engine
* a legal advice engine
* a compliance automation platform by itself

BIL Kernel is the evidence substrate beneath those systems.

---

## Mathematical Framing

Let:

```text
B = banking exposure
I = insurance risk logic
L = legal governance boundary
d = AI-enabled decision
K_BIL = BIL Kernel
A = assurance artifact
```

Then:

```text
A = K_BIL(B, I, L, d)
```

BIL Kernel maps an AI-enabled decision and its institutional context into an assurance artifact.

In institutional holonomy terms:

```text
Hγ(E_d) ≈ 0
```

Meaning:

```text
The evidence bundle preserves its institutional meaning after transport through the banking–insurance–legal loop.
```

---

## Development Status

BIL Kernel is early-stage infrastructure.

Implemented in this bootstrap:

1. Rust workspace scaffold
2. AXLE-compatible response models in `bil-axle`
3. Rust async AXLE client in `bil-client`
4. Evidence-kernel domain types in `bil-core`
5. Canonical schemas in `bil-schema`
6. Deterministic serialization and dual hashing in `bil-hash`
7. Merkle evidence graph construction in `bil-merkle`
8. `.bil` bundle manifest and bundle inspection in `bil-bundle`
9. Receipt issuance in `bil-receipt`
10. Structured verification in `bil-verify`
11. Markdown report rendering in `bil-report`
12. Expanded Rust CLI in `bil-cli`
13. Python compatibility bridge via `axle bil ...`

Next development priorities:

1. institutional metadata profiles
2. example evidence bundles
3. browser and embedded verification targets
4. archive packaging and portability options
5. richer interoperability outputs

---

## Roadmap

### Phase 0 — Fork Boundary

* establish Rust workspace
* define crate layout
* port AXLE response models into Rust
* document AXLE compatibility profile

### Phase 1 — Evidence Kernel

Implemented:

* canonical schemas
* deterministic serialization
* hashing
* Merkle evidence graph
* bundle manifest
* `.bil` bundle directory format

### Phase 2 — Receipts and Verification

Implemented:

* receipt generation
* signature validation
* bundle verification
* CLI verification reports
* JSON and Markdown output

### Phase 3 — Institutional Profiles

* banking profile
* insurance profile
* legal governance profile
* AI assurance profile
* risk and control metadata

### Phase 4 — WASM Verifier

* browser verifier
* client-side bundle inspection
* embedded verification mode
* no-upload verification path

### Phase 5 — Assurance Interop

* AXLE-compatible proof artifact examples
* Lean proof bundle examples
* AI decision bundle examples
* audit and regulatory report templates

---

## Suggested Dependencies

Initial Rust ecosystem candidates:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
sha2 = "0.10"
blake3 = "1"
hex = "0.4"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
ed25519-dalek = "2"
schemars = "0.8"
```

---

## License

To be determined.

The intended direction is open-source infrastructure suitable for broad institutional review, implementation, and extension.

---

## Relationship to AXLE

BIL Kernel is built as a Rust-native downstream fork of AXLE-compatible proof infrastructure.

AXLE provides the upstream formal reasoning context. BIL Kernel extends that context into institutional evidence infrastructure.

```text
AXLE:       proof verification
BIL:        evidence preservation
Banking:    exposure and capital context
Insurance:  risk transfer and loss context
Legal:      accountability and compliance context
Assurance:  verifiable institutional trust
```

---

## Tagline

```text
BIL Kernel is not the AI.
It is the evidence kernel underneath bankable AI.
```
