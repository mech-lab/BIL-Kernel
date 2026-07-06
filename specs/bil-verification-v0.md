# BIL Verification v0

Verification is cumulative. Phase 2 extends Phase 1 bundle integrity checks with receipt validation, and Phase 3 extends the report with institutional profile validation.

## Bundle Integrity

Verification always checks:

- `.bil` directory shape
- `bundle.json`, `manifest.json`, `merkle.json`, and `axle.json` presence
- canonical payload stability for manifest-controlled JSON payloads
- manifest byte lengths
- manifest SHA-256 and BLAKE3 digests
- Merkle recomputation
- `bundle_id` consistency with the SHA-256 Merkle root

If the bundle declares the institutional layer, verification also checks:

- `institutional.json`, `risk.json`, and `controls.json` presence
- institutional marker alignment in `bundle.json`
- banking, insurance, legal governance, and AI assurance section completeness
- referenced risk/control IDs against canonical registries
- duplicated risk/control summaries against canonical registry records
- reciprocal risk/control links
- linked-profile-section accuracy against actual institutional usage
- legal and AI assurance cross-profile identifier links

## Receipt Processing

If a receipt is present or explicitly supplied, verification additionally checks:

- receipt schema validity
- claims canonical reserialization stability
- bundle identity alignment between the receipt and `bundle.json`
- cryptographic signature validity
- covered file digest matches
- covered file completeness against the actual bundle file set

## Trust Model

Receipt trust is explicit.

The verifier compares the receipt public key DER bytes against zero or more caller-supplied DER SubjectPublicKeyInfo trust keys.

- matching trust key: trusted
- no matching trust key: untrusted

Untrusted receipts do not fail verification unless `--require-trust` is set.

## Completeness Rules

For embedded receipts, the actual bundle file set is:

```text
all bundle files except receipt.json
```

For detached receipts, the actual bundle file set is:

```text
all files under the .bil directory
```

Any extra uncovered file or missing covered file fails receipt verification.

## Report Model

Verification emits a structured report with separate status fields for:

- `bundle_verified`
- `institutional_layer_present`
- `banking_profile_verified`
- `insurance_profile_verified`
- `legal_governance_profile_verified`
- `ai_assurance_profile_verified`
- `risk_registry_verified`
- `controls_registry_verified`
- `cross_profile_consistency_verified`
- `receipt_present`
- `signature_valid`
- `trust_verified`
- `overall_verified`

JSON is the canonical machine-readable format. Markdown is a rendered view of the same report data.
