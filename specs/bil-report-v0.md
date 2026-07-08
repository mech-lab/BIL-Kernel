# BIL Report Generation v0

Phase 6 standardizes first-class report generation for existing `v0` bundles.

## CLI Surface

The top-level command is:

```bash
bil report <bundle.bil> --kind verification|audit|regulatory --format json|markdown|sarif
```

Defaults:

- `--kind verification`
- `--format json`

Verification-input flags mirror `bil bundle inspect`:

- `--receipt <file>`
- `--trust-key <file>` repeatable
- `--require-receipt`
- `--require-trust`

`--format sarif` is valid only for `--kind verification`.

## Report Kinds

### Verification

Verification reports reuse the structured verification model already produced by the kernel.

- JSON output is the existing structured verification report
- Markdown output is the human-readable verification rendering
- SARIF output is SARIF 2.1.0 derived from verification findings

### Audit

Audit reports summarize bundle identity, integrity state, institutional verification state, and selected reviewer-facing fields derived from the banking and AI assurance sections.

### Regulatory

Regulatory reports summarize bundle verification state, institutional accountability fields, referenced risk/control identifiers, and linked evidence paths for examination-style review.

## Deterministic Autofill Rules

Audit and regulatory reports are derived only from bundle-native artifacts:

- `bundle.json`
- `manifest.json`
- `merkle.json`
- receipt verification state
- institutional, risk, and control documents when present

Rules:

- missing business fields render as `_not available_`
- list-valued fields are deduplicated, lexicographically sorted, and joined with `; `
- `findings_summary` is `None` when there are no findings
- otherwise `findings_summary` is `code: message` entries joined with `; `
- conclusions are fixed deterministic strings based on bundle verification and institutional completeness

## SARIF Mapping

SARIF output uses:

- version `2.1.0`
- one run
- tool name `BIL Kernel`
- one deduplicated rule per verification finding code
- one result per finding
- `artifactLocation.uri` populated from `logical_path` when present
- `receipt-untrusted` mapped to `warning`
- all other findings mapped to `error`

Verified bundles with no findings produce an empty `results` array.

## Exit Codes

- `0` when the requested report passes its success conditions
- `2` when the report renders successfully but verification or institutional requirements are not satisfied
- `1` for operational or CLI contract failures, including invalid kind/format combinations

Success conditions:

- `verification`: `overall_verified == true`
- `audit` and `regulatory`: bundle verification passes and the institutional layer is present and fully verified
