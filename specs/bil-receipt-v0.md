# BIL Receipt v0

`receipt.json` is the Phase 2 signature envelope for a `.bil` evidence bundle.

## Persistence Modes

Phase 2 supports two receipt locations:

- embedded: `<bundle>.bil/receipt.json`
- detached: `<bundle-name>.receipt.json` adjacent to the bundle, or an explicit output path

Embedded receipts are part of the bundle directory after issuance. Detached receipts remain outside the bundle tree.

## Envelope Shape

The receipt contains:

- `claims`
- `signature`

`claims` is the canonical JSON payload that is signed.

## Claims

Phase 2 claims record:

- `schema_version`
- `receipt_mode`
- `coverage_scope`
- `bundle_id`
- `bundle_kind`
- `profile_version`
- `issued_at`
- `covered_files[]`

`coverage_scope` is fixed to `pre-receipt-bundle-files-v0`.

## Covered Files

`covered_files[]` enumerates the pre-receipt bundle snapshot. Each entry stores:

- `logical_path`
- `byte_length`
- `digests.sha256`
- `digests.blake3`

Covered file digests are over raw on-disk bytes, not semantic re-canonicalization.

For embedded receipts, `receipt.json` is excluded from the covered file set by definition.

For detached receipts, the detached receipt file is never covered because it is outside the bundle directory.

## Signature

The signature block stores:

- `algorithm`
- `key_id`
- `public_key_der_b64`
- `signature_b64`

`key_id` is:

```text
sha256:<hex-of-public-key-der>
```

The embedded public key is DER SubjectPublicKeyInfo encoded as base64.

The signature bytes are base64 encoded.

## Algorithms

Phase 2 implements:

- `ed25519`
- `ecdsa-p256-sha256`
- `rsa-pss-sha256`

Private key input is DER PKCS#8.
