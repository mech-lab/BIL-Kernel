# BIL Institutional Profiles v0

Phase 3 adds a structured institutional layer to a `.bil` bundle through three canonical payload files:

- `institutional.json`
- `risk.json`
- `controls.json`

## Institutional Layer Marker

`bundle.json` keeps `bundle_kind = axle-evidence` and adds:

- `institutional_kind = institutional-profiles-v0`
- `institutional_profile_version = institutional-profiles-v0`

The payload paths must point to:

- `institutional.json`
- `risk.json`
- `controls.json`

## Institutional Document

`institutional.json` stores four required sections:

- `banking`
- `insurance`
- `legal_governance`
- `ai_assurance`

Each section contains its own identifiers plus:

- `referenced_risk_ids`
- `referenced_control_ids`
- `risk_summaries`
- `control_summaries`

The summary arrays duplicate authoritative registry display fields for the referenced records.

## Canonical Registries

`risk.json` is the authoritative registry for risk records.

`controls.json` is the authoritative registry for control records.

The verifier treats registry records as canonical for:

- ID existence
- severity / status display values
- control type / status display values
- reciprocal risk/control linkage
- linked-profile-section declarations

## Receipt Impact

Institutionalization rewrites `bundle.json`, `manifest.json`, and `merkle.json`, changes `bundle_id`, and invalidates any prior receipt. Receipts must be reissued after the institutional layer is added.
