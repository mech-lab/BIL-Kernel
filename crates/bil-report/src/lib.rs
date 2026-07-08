use bil_bundle::BundleInspectionContext;
use bil_core::{
    BundleKind, ControlRecord, ReceiptMode, ReceiptVerificationStatus, VerificationReport,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt::Write;

const NOT_AVAILABLE: &str = "_not available_";
const SARIF_SCHEMA_URI: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const SARIF_VERSION: &str = "2.1.0";
const TOOL_NAME: &str = "BIL Kernel";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AuditReviewReport {
    pub schema_version: String,
    pub report_kind: String,
    pub bundle_path: String,
    pub bundle_id: String,
    pub bundle_kind: String,
    pub profile_version: String,
    pub institutional_kind: String,
    pub institutional_profile_version: String,
    pub bundle_verified: bool,
    pub institutional_layer_present: bool,
    pub receipt_present: bool,
    pub receipt_status: String,
    pub signature_valid: bool,
    pub trust_verified: bool,
    pub overall_verified: bool,
    pub covered_file_count: usize,
    pub merkle_sha256: String,
    pub merkle_blake3: String,
    pub banking_profile_verified: bool,
    pub insurance_profile_verified: bool,
    pub legal_governance_profile_verified: bool,
    pub ai_assurance_profile_verified: bool,
    pub risk_registry_verified: bool,
    pub controls_registry_verified: bool,
    pub cross_profile_consistency_verified: bool,
    pub decision_context: String,
    pub primary_party: String,
    pub assurance_outcome: String,
    pub findings_summary: String,
    pub overall_conclusion: String,
}

impl AuditReviewReport {
    pub fn from_context(context: &BundleInspectionContext) -> Self {
        let report = &context.verification;
        let institutional = context.institutional.as_ref();
        let bundle_verified = report.bundle_verified;
        let institutional_complete = institutional_complete(report);
        let decision_context = institutional
            .map(|value| value.banking.decision_context.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| NOT_AVAILABLE.to_string());
        let primary_party = institutional
            .and_then(|value| {
                if !value.banking.counterparty.is_empty() {
                    Some(value.banking.counterparty.as_str())
                } else if !value.insurance.insured_party.is_empty() {
                    Some(value.insurance.insured_party.as_str())
                } else {
                    None
                }
            })
            .map(str::to_string)
            .unwrap_or_else(|| NOT_AVAILABLE.to_string());
        let assurance_outcome = institutional
            .map(|value| value.ai_assurance.assurance_outcome.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| NOT_AVAILABLE.to_string());

        Self {
            schema_version: report.schema_version.clone(),
            report_kind: "audit-review".to_string(),
            bundle_path: report.bundle_path.clone(),
            bundle_id: option_or_not_available(report.bundle_id.as_deref()),
            bundle_kind: option_or_not_available(
                report.bundle_kind.as_ref().map(bundle_kind_label),
            ),
            profile_version: option_or_not_available(report.profile_version.as_deref()),
            institutional_kind: option_or_not_available(report.institutional_kind.as_deref()),
            institutional_profile_version: option_or_not_available(
                report.institutional_profile_version.as_deref(),
            ),
            bundle_verified,
            institutional_layer_present: report.institutional_layer_present,
            receipt_present: report.receipt_present,
            receipt_status: receipt_status_label(report.receipt_status).to_string(),
            signature_valid: report.signature_valid,
            trust_verified: report.trust_verified,
            overall_verified: report.overall_verified,
            covered_file_count: report.covered_file_count,
            merkle_sha256: context.merkle.trees.sha256.root.clone(),
            merkle_blake3: context.merkle.trees.blake3.root.clone(),
            banking_profile_verified: report.banking_profile_verified,
            insurance_profile_verified: report.insurance_profile_verified,
            legal_governance_profile_verified: report.legal_governance_profile_verified,
            ai_assurance_profile_verified: report.ai_assurance_profile_verified,
            risk_registry_verified: report.risk_registry_verified,
            controls_registry_verified: report.controls_registry_verified,
            cross_profile_consistency_verified: report.cross_profile_consistency_verified,
            decision_context,
            primary_party,
            assurance_outcome,
            findings_summary: findings_summary(report),
            overall_conclusion: audit_conclusion(bundle_verified, institutional_complete),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RegulatoryReviewReport {
    pub schema_version: String,
    pub report_kind: String,
    pub bundle_id: String,
    pub receipt_status: String,
    pub overall_verified: bool,
    pub institutional_layer_present: bool,
    pub bundle_verified: bool,
    pub exposure_id: String,
    pub coverage_case_id: String,
    pub assurance_case_id: String,
    pub payload_count: usize,
    pub verified_entry_count: usize,
    pub receipt_mode: String,
    pub trust_verified: bool,
    pub banking_profile_verified: bool,
    pub insurance_profile_verified: bool,
    pub legal_governance_profile_verified: bool,
    pub ai_assurance_profile_verified: bool,
    pub risk_registry_verified: bool,
    pub controls_registry_verified: bool,
    pub cross_profile_consistency_verified: bool,
    pub banking_review_status: String,
    pub insurance_review_status: String,
    pub legal_compliance_posture: String,
    pub human_review_status: String,
    pub risk_ids: String,
    pub control_ids: String,
    pub rights_and_duties_summary: String,
    pub decision_traceability: String,
    pub control_evidence_paths: String,
    pub findings_summary: String,
    pub overall_conclusion: String,
}

impl RegulatoryReviewReport {
    pub fn from_context(context: &BundleInspectionContext) -> Self {
        let report = &context.verification;
        let institutional = context.institutional.as_ref();
        let risk_ids = referenced_risk_ids(institutional);
        let control_ids = referenced_control_ids(institutional);
        let control_evidence_paths =
            control_evidence_paths(context.controls.as_ref(), &control_ids);
        let institutional_complete = institutional_complete(report);

        Self {
            schema_version: report.schema_version.clone(),
            report_kind: "regulatory-review".to_string(),
            bundle_id: option_or_not_available(report.bundle_id.as_deref()),
            receipt_status: receipt_status_label(report.receipt_status).to_string(),
            overall_verified: report.overall_verified,
            institutional_layer_present: report.institutional_layer_present,
            bundle_verified: report.bundle_verified,
            exposure_id: institutional
                .map(|value| value.banking.exposure_id.as_str())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
            coverage_case_id: institutional
                .map(|value| value.insurance.coverage_case_id.as_str())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
            assurance_case_id: institutional
                .map(|value| value.ai_assurance.assurance_case_id.as_str())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
            payload_count: context.manifest.entries.len(),
            verified_entry_count: report.verified_entries.len(),
            receipt_mode: option_or_not_available(report.receipt_mode.map(receipt_mode_label)),
            trust_verified: report.trust_verified,
            banking_profile_verified: report.banking_profile_verified,
            insurance_profile_verified: report.insurance_profile_verified,
            legal_governance_profile_verified: report.legal_governance_profile_verified,
            ai_assurance_profile_verified: report.ai_assurance_profile_verified,
            risk_registry_verified: report.risk_registry_verified,
            controls_registry_verified: report.controls_registry_verified,
            cross_profile_consistency_verified: report.cross_profile_consistency_verified,
            banking_review_status: institutional
                .map(|value| value.banking.review_status.as_str())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
            insurance_review_status: institutional
                .map(|value| value.insurance.review_status.as_str())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
            legal_compliance_posture: institutional
                .map(|value| value.legal_governance.compliance_posture.as_str())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
            human_review_status: institutional
                .map(|value| value.ai_assurance.human_review_status.as_str())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
            risk_ids: join_or_not_available(risk_ids),
            control_ids: join_or_not_available(control_ids.clone()),
            rights_and_duties_summary: institutional
                .map(|value| value.legal_governance.rights_and_duties_summary.as_str())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
            decision_traceability: institutional
                .map(|value| value.ai_assurance.decision_traceability.as_str())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
            control_evidence_paths,
            findings_summary: findings_summary(report),
            overall_conclusion: regulatory_conclusion(
                report.bundle_verified,
                institutional_complete,
            ),
        }
    }
}

pub fn render_verification_markdown(report: &VerificationReport) -> String {
    let mut output = String::new();
    output.push_str("# BIL Verification Report\n\n");
    output.push_str("| Field | Value |\n");
    output.push_str("| --- | --- |\n");
    output.push_str(&format!("| Bundle path | `{}` |\n", report.bundle_path));
    output.push_str(&format!(
        "| Bundle id | {} |\n",
        report.bundle_id.as_deref().unwrap_or("_unknown_")
    ));
    output.push_str(&format!(
        "| Institutional kind | {} |\n",
        report.institutional_kind.as_deref().unwrap_or("_none_")
    ));
    output.push_str(&format!(
        "| Institutional profile version | {} |\n",
        report
            .institutional_profile_version
            .as_deref()
            .unwrap_or("_none_")
    ));
    output.push_str(&format!(
        "| Bundle verified | `{}` |\n",
        report.bundle_verified
    ));
    output.push_str(&format!(
        "| Institutional layer present | `{}` |\n",
        report.institutional_layer_present
    ));
    output.push_str(&format!(
        "| Receipt present | `{}` |\n",
        report.receipt_present
    ));
    output.push_str(&format!(
        "| Receipt status | `{}` |\n",
        receipt_status_label(report.receipt_status)
    ));
    output.push_str(&format!(
        "| Signature valid | `{}` |\n",
        report.signature_valid
    ));
    output.push_str(&format!(
        "| Trust verified | `{}` |\n",
        report.trust_verified
    ));
    output.push_str(&format!(
        "| Overall verified | `{}` |\n\n",
        report.overall_verified
    ));

    output.push_str("## Coverage\n\n");
    output.push_str(&format!("- Payload count: `{}`\n", report.payload_count));
    output.push_str(&format!(
        "- Covered file count: `{}`\n\n",
        report.covered_file_count
    ));

    output.push_str("## Institutional Status\n\n");
    output.push_str(&format!(
        "- Banking profile verified: `{}`\n",
        report.banking_profile_verified
    ));
    output.push_str(&format!(
        "- Insurance profile verified: `{}`\n",
        report.insurance_profile_verified
    ));
    output.push_str(&format!(
        "- Legal governance profile verified: `{}`\n",
        report.legal_governance_profile_verified
    ));
    output.push_str(&format!(
        "- AI assurance profile verified: `{}`\n",
        report.ai_assurance_profile_verified
    ));
    output.push_str(&format!(
        "- Risk registry verified: `{}`\n",
        report.risk_registry_verified
    ));
    output.push_str(&format!(
        "- Controls registry verified: `{}`\n",
        report.controls_registry_verified
    ));
    output.push_str(&format!(
        "- Cross-profile consistency verified: `{}`\n\n",
        report.cross_profile_consistency_verified
    ));

    output.push_str("## Findings\n\n");
    if report.findings.is_empty() {
        output.push_str("- None\n");
    } else {
        for finding in &report.findings {
            match &finding.logical_path {
                Some(path) => output.push_str(&format!(
                    "- `{}`: {} (`{}`)\n",
                    finding.code, finding.message, path
                )),
                None => output.push_str(&format!("- `{}`: {}\n", finding.code, finding.message)),
            }
        }
    }

    output
}

pub fn render_markdown(report: &VerificationReport) -> String {
    render_verification_markdown(report)
}

pub fn render_audit_markdown(report: &AuditReviewReport) -> String {
    let mut output = String::new();
    output.push_str("# Audit Review Report\n\n");
    output.push_str("## Objective\n\n");
    output.push_str(
        "Provide a jurisdiction-neutral audit review of a BIL bundle using only bundle-native and institutional evidence.\n\n",
    );
    output.push_str("## Bundle Identity\n\n");
    writeln!(output, "- Bundle path: `{}`", report.bundle_path).unwrap();
    writeln!(output, "- Bundle id: `{}`", report.bundle_id).unwrap();
    writeln!(output, "- Bundle kind: `{}`", report.bundle_kind).unwrap();
    writeln!(output, "- Profile version: `{}`", report.profile_version).unwrap();
    writeln!(
        output,
        "- Institutional kind: `{}`",
        report.institutional_kind
    )
    .unwrap();
    writeln!(
        output,
        "- Institutional profile version: `{}`",
        report.institutional_profile_version
    )
    .unwrap();
    output.push('\n');

    output.push_str("## Integrity And Receipt Status\n\n");
    writeln!(output, "- Bundle verified: `{}`", report.bundle_verified).unwrap();
    writeln!(output, "- Receipt present: `{}`", report.receipt_present).unwrap();
    writeln!(output, "- Receipt status: `{}`", report.receipt_status).unwrap();
    writeln!(output, "- Signature valid: `{}`", report.signature_valid).unwrap();
    writeln!(output, "- Trust verified: `{}`", report.trust_verified).unwrap();
    writeln!(
        output,
        "- Covered file count: `{}`",
        report.covered_file_count
    )
    .unwrap();
    writeln!(output, "- SHA-256 Merkle root: `{}`", report.merkle_sha256).unwrap();
    writeln!(output, "- BLAKE3 Merkle root: `{}`", report.merkle_blake3).unwrap();
    output.push('\n');

    output.push_str("## Institutional Review\n\n");
    writeln!(
        output,
        "- Banking profile verified: `{}`",
        report.banking_profile_verified
    )
    .unwrap();
    writeln!(
        output,
        "- Insurance profile verified: `{}`",
        report.insurance_profile_verified
    )
    .unwrap();
    writeln!(
        output,
        "- Legal governance profile verified: `{}`",
        report.legal_governance_profile_verified
    )
    .unwrap();
    writeln!(
        output,
        "- AI assurance profile verified: `{}`",
        report.ai_assurance_profile_verified
    )
    .unwrap();
    writeln!(
        output,
        "- Risk registry verified: `{}`",
        report.risk_registry_verified
    )
    .unwrap();
    writeln!(
        output,
        "- Controls registry verified: `{}`",
        report.controls_registry_verified
    )
    .unwrap();
    writeln!(
        output,
        "- Cross-profile consistency verified: `{}`",
        report.cross_profile_consistency_verified
    )
    .unwrap();
    output.push('\n');

    output.push_str("## Reviewer Notes\n\n");
    writeln!(output, "- Decision context: `{}`", report.decision_context).unwrap();
    writeln!(
        output,
        "- Counterparty or insured party: `{}`",
        report.primary_party
    )
    .unwrap();
    writeln!(
        output,
        "- Assurance outcome: `{}`",
        report.assurance_outcome
    )
    .unwrap();
    writeln!(output, "- Material findings: `{}`", report.findings_summary).unwrap();
    output.push('\n');

    output.push_str("## Conclusion\n\n");
    writeln!(output, "`{}`", report.overall_conclusion).unwrap();
    output
}

pub fn render_regulatory_markdown(report: &RegulatoryReviewReport) -> String {
    let mut output = String::new();
    output.push_str("# Regulatory Review Report\n\n");
    output.push_str("## Objective\n\n");
    output.push_str(
        "Summarize a BIL bundle for jurisdiction-neutral regulatory or examination-style review.\n\n",
    );
    output.push_str("## Record Overview\n\n");
    writeln!(output, "- Bundle id: `{}`", report.bundle_id).unwrap();
    writeln!(output, "- Receipt status: `{}`", report.receipt_status).unwrap();
    writeln!(output, "- Overall verified: `{}`", report.overall_verified).unwrap();
    writeln!(output, "- Exposure id: `{}`", report.exposure_id).unwrap();
    writeln!(output, "- Coverage case id: `{}`", report.coverage_case_id).unwrap();
    writeln!(
        output,
        "- Assurance case id: `{}`",
        report.assurance_case_id
    )
    .unwrap();
    output.push('\n');

    output.push_str("## Evidence Preservation\n\n");
    writeln!(output, "- Payload count: `{}`", report.payload_count).unwrap();
    writeln!(
        output,
        "- Manifest verified entries: `{}`",
        report.verified_entry_count
    )
    .unwrap();
    writeln!(
        output,
        "- Embedded or detached receipt mode: `{}`",
        report.receipt_mode
    )
    .unwrap();
    writeln!(output, "- Trust verified: `{}`", report.trust_verified).unwrap();
    output.push('\n');

    output.push_str("## Institutional Accountability\n\n");
    writeln!(
        output,
        "- Banking review status: `{}`",
        report.banking_review_status
    )
    .unwrap();
    writeln!(
        output,
        "- Insurance review status: `{}`",
        report.insurance_review_status
    )
    .unwrap();
    writeln!(
        output,
        "- Legal compliance posture: `{}`",
        report.legal_compliance_posture
    )
    .unwrap();
    writeln!(
        output,
        "- Human review status: `{}`",
        report.human_review_status
    )
    .unwrap();
    writeln!(output, "- Referenced risks: `{}`", report.risk_ids).unwrap();
    writeln!(output, "- Referenced controls: `{}`", report.control_ids).unwrap();
    output.push('\n');

    output.push_str("## Examination Notes\n\n");
    writeln!(
        output,
        "- Rights and duties summary: `{}`",
        report.rights_and_duties_summary
    )
    .unwrap();
    writeln!(
        output,
        "- Decision traceability: `{}`",
        report.decision_traceability
    )
    .unwrap();
    writeln!(
        output,
        "- Control evidence paths: `{}`",
        report.control_evidence_paths
    )
    .unwrap();
    writeln!(output, "- Findings summary: `{}`", report.findings_summary).unwrap();
    output.push('\n');

    output.push_str("## Conclusion\n\n");
    writeln!(output, "`{}`", report.overall_conclusion).unwrap();
    output
}

pub fn render_verification_sarif(report: &VerificationReport) -> Value {
    let rules = report
        .findings
        .iter()
        .map(|finding| finding.code.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|code| {
            json!({
                "id": code,
                "name": code,
                "shortDescription": { "text": code }
            })
        })
        .collect::<Vec<_>>();

    let results = report
        .findings
        .iter()
        .map(|finding| {
            let mut result = json!({
                "ruleId": finding.code,
                "level": sarif_level(&finding.code),
                "message": { "text": finding.message }
            });
            if let Some(path) = &finding.logical_path {
                result["locations"] = json!([{
                    "physicalLocation": {
                        "artifactLocation": { "uri": path }
                    }
                }]);
            }
            result
        })
        .collect::<Vec<_>>();

    json!({
        "$schema": SARIF_SCHEMA_URI,
        "version": SARIF_VERSION,
        "runs": [{
            "tool": {
                "driver": {
                    "name": TOOL_NAME,
                    "rules": rules
                }
            },
            "results": results
        }]
    })
}

fn referenced_risk_ids(
    institutional: Option<&bil_core::InstitutionalProfilesDocument>,
) -> Vec<String> {
    let Some(institutional) = institutional else {
        return Vec::new();
    };
    [
        institutional.banking.referenced_risk_ids.iter(),
        institutional.insurance.referenced_risk_ids.iter(),
        institutional.legal_governance.referenced_risk_ids.iter(),
        institutional.ai_assurance.referenced_risk_ids.iter(),
    ]
    .into_iter()
    .flatten()
    .cloned()
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect()
}

fn referenced_control_ids(
    institutional: Option<&bil_core::InstitutionalProfilesDocument>,
) -> Vec<String> {
    let Some(institutional) = institutional else {
        return Vec::new();
    };
    [
        institutional.banking.referenced_control_ids.iter(),
        institutional.insurance.referenced_control_ids.iter(),
        institutional.legal_governance.referenced_control_ids.iter(),
        institutional.ai_assurance.referenced_control_ids.iter(),
    ]
    .into_iter()
    .flatten()
    .cloned()
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect()
}

fn control_evidence_paths(
    controls: Option<&bil_core::ControlRegistryDocument>,
    control_ids: &[String],
) -> String {
    let Some(controls) = controls else {
        return NOT_AVAILABLE.to_string();
    };
    let requested = control_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let filtered = controls
        .controls
        .iter()
        .filter(|control| requested.contains(control.control_id.as_str()))
        .collect::<Vec<&ControlRecord>>();

    if filtered.is_empty() {
        return NOT_AVAILABLE.to_string();
    }

    join_or_not_available(
        filtered
            .iter()
            .flat_map(|control| control.evidence_paths.iter().cloned())
            .collect::<Vec<_>>(),
    )
}

fn option_or_not_available(value: Option<impl ToString>) -> String {
    value
        .map(|value| value.to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| NOT_AVAILABLE.to_string())
}

fn join_or_not_available(values: Vec<String>) -> String {
    if values.is_empty() {
        return NOT_AVAILABLE.to_string();
    }
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("; ")
}

fn institutional_complete(report: &VerificationReport) -> bool {
    report.institutional_layer_present
        && report.banking_profile_verified
        && report.insurance_profile_verified
        && report.legal_governance_profile_verified
        && report.ai_assurance_profile_verified
        && report.risk_registry_verified
        && report.controls_registry_verified
        && report.cross_profile_consistency_verified
}

fn audit_conclusion(bundle_verified: bool, institutional_complete: bool) -> String {
    if bundle_verified && institutional_complete {
        "The bundle preserves a verifiable audit trail for the reviewed record and its institutional context."
            .to_string()
    } else if !bundle_verified {
        "The bundle did not verify cleanly and should not be treated as an audit-ready record."
            .to_string()
    } else if !institutional_complete {
        "The bundle does not contain a complete institutional layer for audit-oriented review."
            .to_string()
    } else {
        "The bundle did not satisfy the current audit-oriented review conditions.".to_string()
    }
}

fn regulatory_conclusion(bundle_verified: bool, institutional_complete: bool) -> String {
    if bundle_verified && institutional_complete {
        "The record is materially complete for portable institutional review based on the fields present in the current bundle model."
            .to_string()
    } else if !bundle_verified {
        "The record did not verify cleanly and is not suitable for examination-style reliance."
            .to_string()
    } else if !institutional_complete {
        "The record lacks the complete institutional layer required for examination-style review."
            .to_string()
    } else {
        "The record did not satisfy the current examination-style review conditions.".to_string()
    }
}

fn findings_summary(report: &VerificationReport) -> String {
    if report.findings.is_empty() {
        "None".to_string()
    } else {
        report
            .findings
            .iter()
            .map(|finding| format!("{}: {}", finding.code, finding.message))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

fn bundle_kind_label(kind: &BundleKind) -> &'static str {
    match kind {
        BundleKind::AxleEvidence => "axle-evidence",
    }
}

fn receipt_status_label(status: ReceiptVerificationStatus) -> &'static str {
    match status {
        ReceiptVerificationStatus::Missing => "missing",
        ReceiptVerificationStatus::Verified => "verified",
        ReceiptVerificationStatus::Untrusted => "untrusted",
        ReceiptVerificationStatus::Invalid => "invalid",
    }
}

fn receipt_mode_label(mode: ReceiptMode) -> &'static str {
    match mode {
        ReceiptMode::Embedded => "embedded",
        ReceiptMode::Detached => "detached",
    }
}

fn sarif_level(code: &str) -> &'static str {
    if code == "receipt-untrusted" {
        "warning"
    } else {
        "error"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bil_core::{ManifestEntry, VerificationFinding};

    #[test]
    fn verification_markdown_renders_findings() {
        let report = VerificationReport {
            schema_version: "v0".to_string(),
            bundle_path: "proof.bil".to_string(),
            bundle_id: Some("bil:v0:sha256:abc".to_string()),
            bundle_kind: None,
            profile_version: None,
            institutional_kind: None,
            institutional_profile_version: None,
            payload_count: 1,
            verified_entries: Vec::<ManifestEntry>::new(),
            merkle_roots: None,
            receipt_present: true,
            receipt_status: ReceiptVerificationStatus::Invalid,
            receipt_mode: None,
            receipt_path: None,
            signature_algorithm: None,
            key_id: None,
            covered_file_count: 4,
            bundle_verified: true,
            institutional_layer_present: false,
            banking_profile_verified: false,
            insurance_profile_verified: false,
            legal_governance_profile_verified: false,
            ai_assurance_profile_verified: false,
            risk_registry_verified: false,
            controls_registry_verified: false,
            cross_profile_consistency_verified: false,
            signature_valid: false,
            trust_verified: false,
            overall_verified: false,
            findings: vec![VerificationFinding {
                code: "invalid-signature".to_string(),
                message: "signature mismatch".to_string(),
                logical_path: None,
            }],
        };

        let markdown = render_verification_markdown(&report);
        assert!(markdown.contains("# BIL Verification Report"));
        assert!(markdown.contains("invalid-signature"));
        assert!(markdown.contains("Institutional Status"));
    }

    #[test]
    fn sarif_warning_maps_receipt_untrusted() {
        let report = VerificationReport {
            schema_version: "v0".to_string(),
            bundle_path: "proof.bil".to_string(),
            bundle_id: Some("bil:v0:sha256:abc".to_string()),
            bundle_kind: None,
            profile_version: None,
            institutional_kind: None,
            institutional_profile_version: None,
            payload_count: 1,
            verified_entries: Vec::<ManifestEntry>::new(),
            merkle_roots: None,
            receipt_present: true,
            receipt_status: ReceiptVerificationStatus::Untrusted,
            receipt_mode: None,
            receipt_path: None,
            signature_algorithm: None,
            key_id: None,
            covered_file_count: 1,
            bundle_verified: true,
            institutional_layer_present: false,
            banking_profile_verified: false,
            insurance_profile_verified: false,
            legal_governance_profile_verified: false,
            ai_assurance_profile_verified: false,
            risk_registry_verified: false,
            controls_registry_verified: false,
            cross_profile_consistency_verified: false,
            signature_valid: true,
            trust_verified: false,
            overall_verified: true,
            findings: vec![VerificationFinding {
                code: "receipt-untrusted".to_string(),
                message: "receipt signature is valid but no trusted public key matched".to_string(),
                logical_path: Some("receipt.json".to_string()),
            }],
        };

        let sarif = render_verification_sarif(&report);
        assert_eq!(sarif["runs"][0]["results"][0]["level"], "warning");
        assert_eq!(
            sarif["runs"][0]["results"][0]["ruleId"],
            "receipt-untrusted"
        );
    }
}
