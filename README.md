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

It provides:

* canonical Rust data structures
* deterministic JSON serialization
* AXLE-compatible artifact ingestion
* Merkleized evidence bundles
* cryptographic receipts
* bundle verification
* audit-ready reports
* banking, insurance, and legal metadata profiles
* CLI tooling for developers and assurance teams

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

BIL Kernel is not a Python wrapper.

The kernel runtime is implemented in Rust.

```text
No Python runtime dependency.
No embedded Python client.
No dashboard-first architecture.
No opaque vendor lock-in.
```

The core runtime, CLI, schema models, hashing layer, bundle format, receipt generation, verification engine, and report generator are written in Rust.

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

It can contain:

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
│   ├── bil-core/              # Core institutional artifact types
│   ├── bil-axle/              # AXLE-compatible response and artifact models
│   ├── bil-client/            # Rust async client for AXLE-compatible APIs
│   ├── bil-schema/            # Canonical JSON schemas
│   ├── bil-hash/              # Hashing and content addressing
│   ├── bil-merkle/            # Merkle DAG evidence graph
│   ├── bil-receipt/           # Cryptographic receipt generation
│   ├── bil-bundle/            # .bil bundle packaging
│   ├── bil-verify/            # Bundle verification engine
│   ├── bil-policy/            # Policy and control boundary checks
│   ├── bil-risk/              # Banking and insurance risk metadata
│   ├── bil-legal/             # Legal governance metadata
│   ├── bil-report/            # JSON, Markdown, and SARIF reports
│   ├── bil-cli/               # Command-line interface
│   └── bil-wasm/              # Browser and embedded verifier target
├── specs/
│   ├── bil-bundle-v0.md
│   ├── bil-receipt-v0.md
│   ├── axle-compat-profile-v0.md
│   ├── institutional-holonomy.md
│   └── assurance-apex.md
├── examples/
│   ├── axle-proof-output/
│   ├── ai-credit-decision/
│   ├── underwriting-review/
│   └── legal-governance-record/
└── README.md
```

---

## Planned Crates

### `bil-core`

Core domain types for evidence records, institutional events, decision objects, and assurance metadata.

### `bil-axle`

AXLE-compatible artifact and response models.

### `bil-client`

Rust async client for interacting with AXLE-compatible proof APIs.

### `bil-schema`

Canonical schemas and deterministic serialization rules.

### `bil-hash`

Content addressing, hashing utilities, and canonical digest generation.

### `bil-merkle`

Merkle DAG construction for evidence graphs.

### `bil-receipt`

Cryptographic receipt generation and validation.

### `bil-bundle`

Creation, packaging, unpacking, and inspection of `.bil` bundles.

### `bil-verify`

Verification engine for bundle integrity, receipt validity, schema conformance, and institutional completeness.

### `bil-policy`

Policy and control-boundary evaluation.

### `bil-risk`

Banking, insurance, actuarial, and operational risk metadata.

### `bil-legal`

Legal governance, liability, compliance, and evidentiary metadata.

### `bil-report`

Machine-readable and human-readable reports.

### `bil-cli`

Command-line interface.

### `bil-wasm`

Portable verifier target for browser, embedded, and client-side workflows.

---

## CLI Concept

```bash
bil init
bil import axle ./verify-proof-response.json
bil bundle create ./evidence --out decision.bil
bil receipt issue decision.bil
bil verify decision.bil
bil inspect decision.bil
bil hash decision.bil
bil report decision.bil --format markdown
bil report decision.bil --format json
bil report decision.bil --format sarif
```

---

## Example Workflow

Import an AXLE-compatible proof response:

```bash
bil import axle ./examples/axle-proof-output/verify-proof-response.json
```

Create a BIL evidence bundle:

```bash
bil bundle create ./examples/axle-proof-output --out proof-assurance.bil
```

Issue a receipt:

```bash
bil receipt issue proof-assurance.bil
```

Verify the bundle:

```bash
bil verify proof-assurance.bil
```

Generate an institutional report:

```bash
bil report proof-assurance.bil --format markdown
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

Initial development priorities:

1. Rust workspace scaffold
2. AXLE-compatible response models
3. canonical JSON serialization
4. deterministic hashing
5. `.bil` bundle manifest
6. receipt format
7. verification CLI
8. Markdown and JSON reports
9. institutional metadata profiles
10. example evidence bundles

---

## Roadmap

### Phase 0 — Fork Boundary

* establish Rust workspace
* define crate layout
* port AXLE response models into Rust
* document AXLE compatibility profile

### Phase 1 — Evidence Kernel

* canonical schemas
* deterministic serialization
* hashing
* Merkle evidence graph
* bundle manifest
* `.bil` bundle format

### Phase 2 — Receipts and Verification

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
