# AI Decision Bundle Example

This example packages an AXLE-compatible proof artifact together with Phase 3 institutional metadata for a credit decision review scenario.

The committed bundle is institutionalized, signed with an embedded example receipt, and accompanied by jurisdiction-neutral audit and regulatory review examples.

The receipt, trust key, and backing fixture signing material are demonstration-only and are backed by the deterministic key material in [`../keys`](../keys/README.md).

## Files

- `decision-source.json`: source decision context outside the bundle
- `check-response.json`: source AXLE-compatible artifact for the bundle
- `institutional.json`: institutional profile input
- `risk.json`: canonical risk registry input
- `controls.json`: canonical control registry input
- `ai-decision-bundle.bil/`: institutionalized bundle with embedded receipt
- `trust-key.der`: Ed25519 trust key for the embedded receipt
- `reports/`: committed rendered example audit and regulatory reviews, maintained as documentation rather than CLI output

## Verify

```bash
bil bundle inspect ./examples/ai-decision-bundle/ai-decision-bundle.bil --trust-key ./examples/ai-decision-bundle/trust-key.der
```
