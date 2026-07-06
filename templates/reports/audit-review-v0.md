# Audit Review Template v0

## Objective

Provide a jurisdiction-neutral audit review of a BIL bundle using only committed bundle, receipt, manifest, Merkle, and institutional profile data.

## Bundle Identity

- Bundle path: `{{bundle_path}}`
- Bundle id: `{{bundle_id}}`
- Bundle kind: `{{bundle_kind}}`
- Profile version: `{{profile_version}}`
- Institutional kind: `{{institutional_kind}}`
- Institutional profile version: `{{institutional_profile_version}}`

## Integrity And Receipt Status

- Bundle verified: `{{bundle_verified}}`
- Receipt present: `{{receipt_present}}`
- Receipt status: `{{receipt_status}}`
- Signature valid: `{{signature_valid}}`
- Trust verified: `{{trust_verified}}`
- Covered file count: `{{covered_file_count}}`
- SHA-256 Merkle root: `{{merkle_sha256}}`
- BLAKE3 Merkle root: `{{merkle_blake3}}`

## Institutional Review

- Banking profile verified: `{{banking_profile_verified}}`
- Insurance profile verified: `{{insurance_profile_verified}}`
- Legal governance profile verified: `{{legal_governance_profile_verified}}`
- AI assurance profile verified: `{{ai_assurance_profile_verified}}`
- Risk registry verified: `{{risk_registry_verified}}`
- Controls registry verified: `{{controls_registry_verified}}`
- Cross-profile consistency verified: `{{cross_profile_consistency_verified}}`

## Reviewer Notes

- Decision context: `{{banking_decision_context}}`
- Counterparty or insured party: `{{primary_party}}`
- Assurance outcome: `{{assurance_outcome}}`
- Material findings: `{{findings_summary}}`

## Conclusion

`{{overall_conclusion}}`
