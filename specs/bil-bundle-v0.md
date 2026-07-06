# BIL Bundle v0

`v0` defines the canonical on-disk `.bil` bundle as an unpacked directory whose name ends with `.bil`.

## Scope

Phase 1 is AXLE-first. A baseline bundle contains one required evidence payload, `axle.json`, plus three control documents:

- `bundle.json`
- `manifest.json`
- `merkle.json`

Phase 3 extends the payload set with structured institutional files:

- `institutional.json`
- `risk.json`
- `controls.json`

Those files become manifest-controlled payloads and change the bundle Merkle root and `bundle_id`.

## Directory Layout

```text
<name>.bil/
├── axle.json
├── bundle.json
├── institutional.json   # Phase 3 institutional layer, when present
├── manifest.json
├── merkle.json
├── risk.json            # Phase 3 canonical risk registry, when present
└── controls.json        # Phase 3 canonical control registry, when present
```

## Control Documents

`bundle.json` is the top-level descriptor. It records:

- `schema_version`
- `bundle_kind`
- `bundle_id`
- `profile_version`
- `institutional_kind` (Phase 3, optional)
- `institutional_profile_version` (Phase 3, optional)
- `manifest_path`
- `merkle_path`
- `payload_paths.axle`
- `payload_paths.institutional` (Phase 3, optional)
- `payload_paths.risk` (Phase 3, optional)
- `payload_paths.controls` (Phase 3, optional)

For Phase 1, `bundle_kind` is `axle-evidence`.

For Phase 3, `bundle_kind` remains `axle-evidence` and the institutional layer is marked by:

- `institutional_kind = institutional-profiles-v0`
- `institutional_profile_version = institutional-profiles-v0`

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

When present, the institutional payload set stores:

- `institutional.json`: banking, insurance, legal governance, and AI assurance sections
- `risk.json`: the canonical risk registry
- `controls.json`: the canonical control registry

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
- institutional marker and payload consistency when the Phase 3 layer is present
