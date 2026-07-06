# AXLE Proof Artifact Example

This example starts from a raw AXLE-compatible `VerifyProofResponse`, packages it into a `.bil` bundle, and signs the bundle with an embedded example receipt.

The receipt, trust key, and backing fixture signing material are demonstration-only and are backed by the deterministic key material in [`../keys`](../keys/README.md).

## Files

- `verify-proof-response.json`: source AXLE-compatible proof artifact
- `axle-proof-artifact.bil/`: generated bundle with embedded receipt
- `trust-key.der`: Ed25519 trust key for verifying the embedded example receipt

## Verify

```bash
bil bundle inspect ./examples/axle-proof-artifact/axle-proof-artifact.bil --trust-key ./examples/axle-proof-artifact/trust-key.der
```
