# BIL Bundle v0

`v0` defines the canonical on-disk `.bil` bundle as an unpacked directory whose name ends with `.bil`.

## Scope

Phase 1 is AXLE-first. A valid bundle contains one required evidence payload, `axle.json`, plus three control documents:

- `bundle.json`
- `manifest.json`
- `merkle.json`

Future phases may add additional evidence payload files, but Phase 1 verification is defined around the single AXLE payload.

## Directory Layout

```text
<name>.bil/
├── axle.json
├── bundle.json
├── manifest.json
└── merkle.json
```

## Control Documents

`bundle.json` is the top-level descriptor. It records:

- `schema_version`
- `bundle_kind`
- `bundle_id`
- `profile_version`
- `manifest_path`
- `merkle_path`
- `payload_paths.axle`

For Phase 1, `bundle_kind` is `axle-evidence`.

`bundle_id` is derived from the SHA-256 Merkle root:

```text
bil:v0:sha256:<hex-root>
```

## Payload Rules

`axle.json` is the canonical AXLE evidence envelope. It stores:

- `schema_version`
- `artifact_kind`
- `payload`

The payload is serialized into canonical JSON before hashing and persistence.

## Determinism

A Phase 1 bundle is deterministic when the AXLE input bytes and explicit CLI options are the same.

The implementation must not inject wall-clock timestamps, random identifiers, or host-specific metadata into canonical bundle documents.

## Verification

`bil bundle inspect <dir>.bil` verifies:

- the bundle directory shape
- canonical payload byte stability
- manifest byte lengths
- manifest SHA-256 and BLAKE3 digests
- Merkle tree recomputation
- `bundle_id` consistency with the SHA-256 Merkle root
