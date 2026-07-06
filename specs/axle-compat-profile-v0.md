# AXLE Compatibility Profile v0

`axle-compat-v0` defines the AXLE artifact boundary used by the Phase 1 evidence kernel.

## Purpose

The kernel does not replace AXLE proof checking in Phase 1. It accepts typed AXLE-compatible response artifacts and turns them into deterministic evidence payloads that can be hashed, bundled, and verified.

## Supported Artifact Kinds

The Phase 1 profile recognizes the AXLE response families modeled in `bil-axle`:

- `verify-proof`
- `check`
- `document`
- `extract-decls`
- `extract-theorems`
- `rename`
- `merge`
- `theorem2-sorry`
- `theorem2-lemma`
- `simplify-theorems`
- `repair-proofs`
- `have2-lemma`
- `have2-sorry`
- `sorry2-lemma`
- `disprove`
- `normalize`

The CLI accepts kebab-case aliases. The canonical Rust enum is `bil_core::AxleArtifactKind`.

## Evidence Envelope

Phase 1 persists AXLE artifacts into `axle.json` as:

```json
{
  "schema_version": "v0",
  "artifact_kind": "<kind>",
  "payload": { "...": "typed AXLE response JSON" }
}
```

`payload` must deserialize into the typed AXLE model selected by `artifact_kind`.

## Canonicalization

The saved `axle.json` document is canonical JSON:

- UTF-8
- no trailing newline
- sorted object keys
- stable array ordering
- stable number and string rendering as produced by the canonical serializer

The canonical bytes are the bytes hashed into the manifest and Merkle tree.
