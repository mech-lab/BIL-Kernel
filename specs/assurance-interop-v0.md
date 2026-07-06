# BIL Assurance Interop v0

Phase 5 standardizes the repository-facing example and template conventions for interoperability fixtures.

## Example Layout

Each committed example directory may contain:

- source inputs outside the bundle
- one generated `.bil/` bundle
- either an embedded receipt inside the bundle or a detached receipt adjacent to it
- one trust public key for verification
- a short README explaining source-to-bundle mapping

Phase 5 defines three example classes:

- AXLE-compatible proof artifact example
- Lean proof bundle example
- AI decision bundle example

## Signing Fixtures

Phase 5 fixture signing uses deterministic non-production Ed25519 key material committed under `examples/keys/`.

The fixture conventions are:

- fixed key pair
- fixed `issued_at` timestamp
- reproducible signed receipts
- no production trust semantics

## Receipt Expectations

Example receipts must be directly verifiable with the current CLI.

- embedded examples verify with `bil bundle inspect <bundle> --trust-key <key>`
- detached examples verify with `bil bundle inspect <bundle> --receipt <receipt> --trust-key <key>`

## Template Conventions

Phase 5 templates are committed Markdown documents under `templates/reports/`.

They are:

- jurisdiction-neutral
- grounded only in current bundle, receipt, manifest, Merkle, and institutional fields
- illustrative rather than executable rendering logic

Rendered example reports may be committed beside example fixtures, but Phase 5 does not add a `bil report` command.
