# BIL Manifest v0

`manifest.json` inventories evidence payload files included in a `.bil` bundle.

## Scope

The Phase 1 manifest includes evidence payload entries only. Control documents such as `bundle.json`, `manifest.json`, and `merkle.json` are excluded from the manifest leaf set.

For the first AXLE-first cut, the manifest contains at least one entry:

- `axle.json`

## Document Shape

The manifest contains:

- `schema_version`
- `entries`

Each `entries[]` object contains:

- `logical_path`
- `media_type`
- `canonicalization`
- `byte_length`
- `digests.sha256`
- `digests.blake3`

## Path Semantics

`logical_path` values are normalized forward-slash relative paths.

They must not:

- be absolute
- contain `..`
- contain duplicate logical paths after normalization

Entries are sorted lexicographically by normalized `logical_path` before Merkle construction.

## Canonicalization Modes

Phase 1 supports:

- `json-canonical-v0`
- `raw-bytes-v0`

`axle.json` uses `json-canonical-v0`.

## Digest Semantics

Each manifest entry stores both SHA-256 and BLAKE3 digests over the canonicalized payload bytes.

The stored `byte_length` is the canonicalized payload length, not an arbitrary source-file length.
