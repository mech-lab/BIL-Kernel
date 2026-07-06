# Lean Proof Bundle Example

This example starts from a Lean source file, captures an AXLE-compatible `CheckResponse`, packages that response into a `.bil` bundle, and signs it with a detached example receipt.

The receipt, trust key, and backing fixture signing material are demonstration-only and are backed by the deterministic key material in [`../keys`](../keys/README.md).

## Files

- `proof.lean`: Lean source input
- `check-response.json`: captured AXLE-compatible response derived from the Lean source
- `lean-proof-bundle.bil/`: generated bundle
- `lean-proof-bundle.receipt.json`: detached example receipt
- `trust-key.der`: Ed25519 trust key for verifying the detached receipt

## Verify

```bash
bil bundle inspect ./examples/lean-proof-bundle/lean-proof-bundle.bil --receipt ./examples/lean-proof-bundle/lean-proof-bundle.receipt.json --trust-key ./examples/lean-proof-bundle/trust-key.der
```
