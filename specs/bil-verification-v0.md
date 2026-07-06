# BIL Verification v0

Phase 2 verification extends Phase 1 bundle integrity checks with optional receipt and trust validation.

## Bundle Integrity

Verification always checks:

- `.bil` directory shape
- `bundle.json`, `manifest.json`, `merkle.json`, and `axle.json` presence
- canonical payload stability for manifest-controlled JSON payloads
- manifest byte lengths
- manifest SHA-256 and BLAKE3 digests
- Merkle recomputation
- `bundle_id` consistency with the SHA-256 Merkle root

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

Phase 2 verification emits a structured report with separate status fields for:

- `bundle_verified`
- `receipt_present`
- `signature_valid`
- `trust_verified`
- `overall_verified`

JSON is the canonical machine-readable format. Markdown is a rendered view of the same report data.
