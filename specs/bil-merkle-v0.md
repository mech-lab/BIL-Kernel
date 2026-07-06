# BIL Merkle v0

`merkle.json` captures the deterministic Merkle graph for the evidence payload entries listed in `manifest.json`.

## Leaf Set

Leaves are derived from the manifest entries, not from the control documents.

Each leaf records:

- `logical_path`
- `digests`

Leaf ordering is the lexicographic ordering of normalized manifest `logical_path` values.

## Tree Shape

Phase 1 uses one deterministic tree shape for both hashing algorithms.

- Leaves are paired left-to-right.
- When a level has an odd number of nodes, the final node is duplicated.
- The next level is computed by hashing the concatenated left and right node bytes.

## Algorithms

The same ordered leaf set is projected into two trees:

- `sha256`
- `blake3`

`merkle.json` stores:

- `schema_version`
- `leaf_order`
- `leaves`
- `trees.sha256`
- `trees.blake3`

Each tree stores:

- `algorithm`
- `root`
- `levels[]`

Each level stores its zero-based `level` index and the hex-encoded `nodes` at that level.

## Root Semantics

The SHA-256 root is authoritative for bundle identity in Phase 1 and feeds the `bundle_id`.

The BLAKE3 root is stored alongside it for parallel verification and future interoperability.
