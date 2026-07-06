use bil_core::{
    AiAssuranceProfile, BankingProfile, ControlRecord, ControlRegistryDocument, ControlSummaryRef,
    InstitutionalProfileSection, InstitutionalProfilesDocument, InsuranceProfile,
    LegalGovernanceProfile, RiskRecord, RiskRegistryDocument, RiskSummaryRef, VerificationFinding,
    normalize_logical_path,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionalPolicyReport {
    pub banking_profile_verified: bool,
    pub insurance_profile_verified: bool,
    pub legal_governance_profile_verified: bool,
    pub ai_assurance_profile_verified: bool,
    pub cross_profile_consistency_verified: bool,
    pub findings: Vec<VerificationFinding>,
}

pub fn validate_institutional_profiles(
    profiles: &InstitutionalProfilesDocument,
    risk_registry: &RiskRegistryDocument,
    control_registry: &ControlRegistryDocument,
    expected_axle_path: &str,
) -> InstitutionalPolicyReport {
    let risk_map = risk_registry
        .risks
        .iter()
        .map(|risk| (risk.risk_id.clone(), risk))
        .collect::<BTreeMap<_, _>>();
    let control_map = control_registry
        .controls
        .iter()
        .map(|control| (control.control_id.clone(), control))
        .collect::<BTreeMap<_, _>>();

    let mut findings = Vec::new();
    let mut banking_profile_verified = true;
    let mut insurance_profile_verified = true;
    let mut legal_governance_profile_verified = true;
    let mut ai_assurance_profile_verified = true;
    let mut cross_profile_consistency_verified = true;

    validate_banking_profile(
        &profiles.banking,
        &risk_map,
        &control_map,
        &mut findings,
        &mut banking_profile_verified,
    );
    validate_insurance_profile(
        &profiles.insurance,
        &risk_map,
        &control_map,
        &mut findings,
        &mut insurance_profile_verified,
    );
    validate_legal_profile(
        &profiles.legal_governance,
        &risk_map,
        &control_map,
        &mut findings,
        &mut legal_governance_profile_verified,
    );
    validate_ai_profile(
        &profiles.ai_assurance,
        &risk_map,
        &control_map,
        expected_axle_path,
        &mut findings,
        &mut ai_assurance_profile_verified,
    );

    validate_cross_profile_consistency(
        profiles,
        risk_registry,
        control_registry,
        &mut findings,
        &mut cross_profile_consistency_verified,
    );

    InstitutionalPolicyReport {
        banking_profile_verified,
        insurance_profile_verified,
        legal_governance_profile_verified,
        ai_assurance_profile_verified,
        cross_profile_consistency_verified,
        findings,
    }
}

fn validate_banking_profile(
    profile: &BankingProfile,
    risk_map: &BTreeMap<String, &RiskRecord>,
    control_map: &BTreeMap<String, &ControlRecord>,
    findings: &mut Vec<VerificationFinding>,
    verified: &mut bool,
) {
    for (field, value) in [
        ("exposure_id", profile.exposure_id.as_str()),
        ("decision_context", profile.decision_context.as_str()),
        ("counterparty", profile.counterparty.as_str()),
        ("product_type", profile.product_type.as_str()),
        ("currency", profile.currency.as_str()),
        ("exposure_amount", profile.exposure_amount.as_str()),
        ("decision_outcome", profile.decision_outcome.as_str()),
        ("review_status", profile.review_status.as_str()),
    ] {
        ensure_non_blank(
            "banking-profile-empty-field",
            "banking",
            field,
            value,
            findings,
            verified,
        );
    }

    validate_risk_references(
        "banking",
        &profile.referenced_risk_ids,
        &profile.risk_summaries,
        risk_map,
        findings,
        verified,
    );
    validate_control_references(
        "banking",
        &profile.referenced_control_ids,
        &profile.control_summaries,
        control_map,
        findings,
        verified,
    );
}

fn validate_insurance_profile(
    profile: &InsuranceProfile,
    risk_map: &BTreeMap<String, &RiskRecord>,
    control_map: &BTreeMap<String, &ControlRecord>,
    findings: &mut Vec<VerificationFinding>,
    verified: &mut bool,
) {
    for (field, value) in [
        ("coverage_case_id", profile.coverage_case_id.as_str()),
        ("coverage_context", profile.coverage_context.as_str()),
        ("insured_party", profile.insured_party.as_str()),
        ("coverage_type", profile.coverage_type.as_str()),
        ("insured_amount", profile.insured_amount.as_str()),
        ("decision_outcome", profile.decision_outcome.as_str()),
        ("review_status", profile.review_status.as_str()),
    ] {
        ensure_non_blank(
            "insurance-profile-empty-field",
            "insurance",
            field,
            value,
            findings,
            verified,
        );
    }

    validate_risk_references(
        "insurance",
        &profile.referenced_risk_ids,
        &profile.risk_summaries,
        risk_map,
        findings,
        verified,
    );
    validate_control_references(
        "insurance",
        &profile.referenced_control_ids,
        &profile.control_summaries,
        control_map,
        findings,
        verified,
    );
}

fn validate_legal_profile(
    profile: &LegalGovernanceProfile,
    risk_map: &BTreeMap<String, &RiskRecord>,
    control_map: &BTreeMap<String, &ControlRecord>,
    findings: &mut Vec<VerificationFinding>,
    verified: &mut bool,
) {
    for (field, value) in [
        ("matter_id", profile.matter_id.as_str()),
        (
            "rights_and_duties_summary",
            profile.rights_and_duties_summary.as_str(),
        ),
        ("liability_posture", profile.liability_posture.as_str()),
        ("compliance_posture", profile.compliance_posture.as_str()),
    ] {
        ensure_non_blank(
            "legal-governance-profile-empty-field",
            "legal governance",
            field,
            value,
            findings,
            verified,
        );
    }

    validate_risk_references(
        "legal-governance",
        &profile.referenced_risk_ids,
        &profile.risk_summaries,
        risk_map,
        findings,
        verified,
    );
    validate_control_references(
        "legal-governance",
        &profile.referenced_control_ids,
        &profile.control_summaries,
        control_map,
        findings,
        verified,
    );
}

fn validate_ai_profile(
    profile: &AiAssuranceProfile,
    risk_map: &BTreeMap<String, &RiskRecord>,
    control_map: &BTreeMap<String, &ControlRecord>,
    expected_axle_path: &str,
    findings: &mut Vec<VerificationFinding>,
    verified: &mut bool,
) {
    for (field, value) in [
        ("assurance_case_id", profile.assurance_case_id.as_str()),
        ("system_identifier", profile.system_identifier.as_str()),
        ("model_identifier", profile.model_identifier.as_str()),
        (
            "decision_traceability",
            profile.decision_traceability.as_str(),
        ),
        ("human_review_status", profile.human_review_status.as_str()),
        ("assurance_outcome", profile.assurance_outcome.as_str()),
        (
            "linked_axle_artifact_path",
            profile.linked_axle_artifact_path.as_str(),
        ),
    ] {
        ensure_non_blank(
            "ai-assurance-profile-empty-field",
            "ai assurance",
            field,
            value,
            findings,
            verified,
        );
    }

    match normalize_logical_path(&profile.linked_axle_artifact_path) {
        Ok(path) if path == expected_axle_path => {}
        Ok(path) => {
            *verified = false;
            findings.push(VerificationFinding {
                code: "ai-assurance-profile-axle-path-mismatch".to_string(),
                message: format!(
                    "ai assurance profile links {path}, expected {expected_axle_path}"
                ),
                logical_path: Some("institutional.json".to_string()),
            });
        }
        Err(_) => {
            *verified = false;
            findings.push(VerificationFinding {
                code: "ai-assurance-profile-invalid-axle-path".to_string(),
                message: "ai assurance profile contains an invalid linked_axle_artifact_path"
                    .to_string(),
                logical_path: Some("institutional.json".to_string()),
            });
        }
    }

    validate_risk_references(
        "ai-assurance",
        &profile.referenced_risk_ids,
        &profile.risk_summaries,
        risk_map,
        findings,
        verified,
    );
    validate_control_references(
        "ai-assurance",
        &profile.referenced_control_ids,
        &profile.control_summaries,
        control_map,
        findings,
        verified,
    );
}

fn validate_cross_profile_consistency(
    profiles: &InstitutionalProfilesDocument,
    risk_registry: &RiskRegistryDocument,
    control_registry: &ControlRegistryDocument,
    findings: &mut Vec<VerificationFinding>,
    verified: &mut bool,
) {
    let banking_exposure = &profiles.banking.exposure_id;
    let insurance_case = &profiles.insurance.coverage_case_id;
    let assurance_case = &profiles.ai_assurance.assurance_case_id;

    for exposure_id in &profiles.ai_assurance.linked_exposure_ids {
        if exposure_id != banking_exposure {
            *verified = false;
            findings.push(VerificationFinding {
                code: "cross-profile-missing-exposure-link".to_string(),
                message: format!(
                    "ai assurance profile references unknown exposure id {exposure_id}"
                ),
                logical_path: Some("institutional.json".to_string()),
            });
        }
    }

    for coverage_case_id in &profiles.ai_assurance.linked_coverage_case_ids {
        if coverage_case_id != insurance_case {
            *verified = false;
            findings.push(VerificationFinding {
                code: "cross-profile-missing-coverage-link".to_string(),
                message: format!(
                    "ai assurance profile references unknown coverage case id {coverage_case_id}"
                ),
                logical_path: Some("institutional.json".to_string()),
            });
        }
    }

    let mut risk_usage = BTreeMap::<String, BTreeSet<InstitutionalProfileSection>>::new();
    let mut control_usage = BTreeMap::<String, BTreeSet<InstitutionalProfileSection>>::new();
    collect_usage(
        &mut risk_usage,
        &mut control_usage,
        InstitutionalProfileSection::Banking,
        &profiles.banking.referenced_risk_ids,
        &profiles.banking.referenced_control_ids,
    );
    collect_usage(
        &mut risk_usage,
        &mut control_usage,
        InstitutionalProfileSection::Insurance,
        &profiles.insurance.referenced_risk_ids,
        &profiles.insurance.referenced_control_ids,
    );
    collect_usage(
        &mut risk_usage,
        &mut control_usage,
        InstitutionalProfileSection::LegalGovernance,
        &profiles.legal_governance.referenced_risk_ids,
        &profiles.legal_governance.referenced_control_ids,
    );
    collect_usage(
        &mut risk_usage,
        &mut control_usage,
        InstitutionalProfileSection::AiAssurance,
        &profiles.ai_assurance.referenced_risk_ids,
        &profiles.ai_assurance.referenced_control_ids,
    );

    for risk in &risk_registry.risks {
        let actual = risk_usage
            .get(&risk.risk_id)
            .cloned()
            .unwrap_or_else(BTreeSet::new);
        let declared = risk
            .linked_profile_sections
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if actual != declared {
            *verified = false;
            findings.push(VerificationFinding {
                code: "cross-profile-risk-linked-section-mismatch".to_string(),
                message: format!(
                    "risk {} linked_profile_sections do not match actual institutional usage",
                    risk.risk_id
                ),
                logical_path: Some("risk.json".to_string()),
            });
        }
    }

    for control in &control_registry.controls {
        let actual = control_usage
            .get(&control.control_id)
            .cloned()
            .unwrap_or_else(BTreeSet::new);
        let declared = control
            .linked_profile_sections
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if actual != declared {
            *verified = false;
            findings.push(VerificationFinding {
                code: "cross-profile-control-linked-section-mismatch".to_string(),
                message: format!(
                    "control {} linked_profile_sections do not match actual institutional usage",
                    control.control_id
                ),
                logical_path: Some("controls.json".to_string()),
            });
        }
    }

    for assurance_id in &profiles.legal_governance.linked_assurance_case_ids {
        if assurance_id != assurance_case {
            *verified = false;
            findings.push(VerificationFinding {
                code: "cross-profile-missing-legal-assurance-link".to_string(),
                message: format!(
                    "legal governance profile references unknown assurance case id {assurance_id}"
                ),
                logical_path: Some("institutional.json".to_string()),
            });
        }
    }
}

fn collect_usage(
    risk_usage: &mut BTreeMap<String, BTreeSet<InstitutionalProfileSection>>,
    control_usage: &mut BTreeMap<String, BTreeSet<InstitutionalProfileSection>>,
    section: InstitutionalProfileSection,
    risk_ids: &[String],
    control_ids: &[String],
) {
    for risk_id in risk_ids {
        risk_usage
            .entry(risk_id.clone())
            .or_default()
            .insert(section);
    }
    for control_id in control_ids {
        control_usage
            .entry(control_id.clone())
            .or_default()
            .insert(section);
    }
}

fn ensure_non_blank(
    code: &str,
    profile: &str,
    field: &str,
    value: &str,
    findings: &mut Vec<VerificationFinding>,
    verified: &mut bool,
) {
    if value.trim().is_empty() {
        *verified = false;
        findings.push(VerificationFinding {
            code: code.to_string(),
            message: format!("{profile} profile has an empty {field}"),
            logical_path: Some("institutional.json".to_string()),
        });
    }
}

fn validate_risk_references(
    section: &str,
    referenced_ids: &[String],
    summaries: &[RiskSummaryRef],
    risk_map: &BTreeMap<String, &RiskRecord>,
    findings: &mut Vec<VerificationFinding>,
    verified: &mut bool,
) {
    let referenced = referenced_ids.iter().cloned().collect::<BTreeSet<_>>();
    let summarized = summaries
        .iter()
        .map(|summary| summary.risk_id.clone())
        .collect::<BTreeSet<_>>();

    for risk_id in referenced.difference(&summarized) {
        *verified = false;
        findings.push(VerificationFinding {
            code: format!("{section}-profile-missing-risk-summary"),
            message: format!("{section} profile references risk {risk_id} without a summary"),
            logical_path: Some("institutional.json".to_string()),
        });
    }

    for risk_id in summarized.difference(&referenced) {
        *verified = false;
        findings.push(VerificationFinding {
            code: format!("{section}-profile-unreferenced-risk-summary"),
            message: format!(
                "{section} profile includes a summary for unreferenced risk {risk_id}"
            ),
            logical_path: Some("institutional.json".to_string()),
        });
    }

    for risk_id in referenced_ids {
        if !risk_map.contains_key(risk_id) {
            *verified = false;
            findings.push(VerificationFinding {
                code: format!("{section}-profile-unknown-risk"),
                message: format!("{section} profile references unknown risk {risk_id}"),
                logical_path: Some("institutional.json".to_string()),
            });
        }
    }

    for summary in summaries {
        match risk_map.get(&summary.risk_id) {
            None => {
                *verified = false;
                findings.push(VerificationFinding {
                    code: format!("{section}-profile-unknown-risk-summary"),
                    message: format!(
                        "{section} profile summary references unknown risk {}",
                        summary.risk_id
                    ),
                    logical_path: Some("institutional.json".to_string()),
                });
            }
            Some(risk) => {
                if summary.title != risk.title
                    || summary.severity != risk.severity
                    || summary.status != risk.status
                {
                    *verified = false;
                    findings.push(VerificationFinding {
                        code: format!("{section}-profile-risk-summary-mismatch"),
                        message: format!(
                            "{section} profile summary does not match canonical risk {}",
                            summary.risk_id
                        ),
                        logical_path: Some("institutional.json".to_string()),
                    });
                }
            }
        }
    }
}

fn validate_control_references(
    section: &str,
    referenced_ids: &[String],
    summaries: &[ControlSummaryRef],
    control_map: &BTreeMap<String, &ControlRecord>,
    findings: &mut Vec<VerificationFinding>,
    verified: &mut bool,
) {
    let referenced = referenced_ids.iter().cloned().collect::<BTreeSet<_>>();
    let summarized = summaries
        .iter()
        .map(|summary| summary.control_id.clone())
        .collect::<BTreeSet<_>>();

    for control_id in referenced.difference(&summarized) {
        *verified = false;
        findings.push(VerificationFinding {
            code: format!("{section}-profile-missing-control-summary"),
            message: format!("{section} profile references control {control_id} without a summary"),
            logical_path: Some("institutional.json".to_string()),
        });
    }

    for control_id in summarized.difference(&referenced) {
        *verified = false;
        findings.push(VerificationFinding {
            code: format!("{section}-profile-unreferenced-control-summary"),
            message: format!(
                "{section} profile includes a summary for unreferenced control {control_id}"
            ),
            logical_path: Some("institutional.json".to_string()),
        });
    }

    for control_id in referenced_ids {
        if !control_map.contains_key(control_id) {
            *verified = false;
            findings.push(VerificationFinding {
                code: format!("{section}-profile-unknown-control"),
                message: format!("{section} profile references unknown control {control_id}"),
                logical_path: Some("institutional.json".to_string()),
            });
        }
    }

    for summary in summaries {
        match control_map.get(&summary.control_id) {
            None => {
                *verified = false;
                findings.push(VerificationFinding {
                    code: format!("{section}-profile-unknown-control-summary"),
                    message: format!(
                        "{section} profile summary references unknown control {}",
                        summary.control_id
                    ),
                    logical_path: Some("institutional.json".to_string()),
                });
            }
            Some(control) => {
                if summary.title != control.title
                    || summary.control_type != control.control_type
                    || summary.status != control.status
                {
                    *verified = false;
                    findings.push(VerificationFinding {
                        code: format!("{section}-profile-control-summary-mismatch"),
                        message: format!(
                            "{section} profile summary does not match canonical control {}",
                            summary.control_id
                        ),
                        logical_path: Some("institutional.json".to_string()),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bil_core::{
        AiAssuranceProfile, BankingProfile, ControlRecord, ControlRegistryDocument,
        ControlSummaryRef, InstitutionalProfileSection, InstitutionalProfilesDocument,
        InsuranceProfile, LegalGovernanceProfile, RiskRecord, RiskRegistryDocument, RiskSummaryRef,
        SCHEMA_VERSION_V0,
    };

    fn profiles() -> InstitutionalProfilesDocument {
        InstitutionalProfilesDocument {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            banking: BankingProfile {
                exposure_id: "exp-1".to_string(),
                decision_context: "ctx".to_string(),
                counterparty: "cp".to_string(),
                product_type: "loan".to_string(),
                currency: "USD".to_string(),
                exposure_amount: "100".to_string(),
                decision_outcome: "approved".to_string(),
                review_status: "reviewed".to_string(),
                governing_policy_refs: vec![],
                referenced_risk_ids: vec!["risk-1".to_string()],
                referenced_control_ids: vec!["control-1".to_string()],
                risk_summaries: vec![RiskSummaryRef {
                    risk_id: "risk-1".to_string(),
                    title: "Model drift".to_string(),
                    severity: "high".to_string(),
                    status: "open".to_string(),
                }],
                control_summaries: vec![ControlSummaryRef {
                    control_id: "control-1".to_string(),
                    title: "Human review".to_string(),
                    control_type: "review".to_string(),
                    status: "active".to_string(),
                }],
            },
            insurance: InsuranceProfile {
                coverage_case_id: "cov-1".to_string(),
                coverage_context: "ctx".to_string(),
                insured_party: "party".to_string(),
                coverage_type: "liability".to_string(),
                insured_amount: "100".to_string(),
                decision_outcome: "bound".to_string(),
                review_status: "reviewed".to_string(),
                policy_refs: vec![],
                referenced_risk_ids: vec!["risk-1".to_string()],
                referenced_control_ids: vec!["control-1".to_string()],
                risk_summaries: vec![RiskSummaryRef {
                    risk_id: "risk-1".to_string(),
                    title: "Model drift".to_string(),
                    severity: "high".to_string(),
                    status: "open".to_string(),
                }],
                control_summaries: vec![ControlSummaryRef {
                    control_id: "control-1".to_string(),
                    title: "Human review".to_string(),
                    control_type: "review".to_string(),
                    status: "active".to_string(),
                }],
            },
            legal_governance: LegalGovernanceProfile {
                matter_id: "matter-1".to_string(),
                rights_and_duties_summary: "summary".to_string(),
                liability_posture: "contained".to_string(),
                compliance_posture: "compliant".to_string(),
                governing_authority_refs: vec![],
                linked_exposure_ids: vec!["exp-1".to_string()],
                linked_coverage_case_ids: vec!["cov-1".to_string()],
                linked_assurance_case_ids: vec!["assure-1".to_string()],
                referenced_risk_ids: vec!["risk-1".to_string()],
                referenced_control_ids: vec!["control-1".to_string()],
                risk_summaries: vec![RiskSummaryRef {
                    risk_id: "risk-1".to_string(),
                    title: "Model drift".to_string(),
                    severity: "high".to_string(),
                    status: "open".to_string(),
                }],
                control_summaries: vec![ControlSummaryRef {
                    control_id: "control-1".to_string(),
                    title: "Human review".to_string(),
                    control_type: "review".to_string(),
                    status: "active".to_string(),
                }],
            },
            ai_assurance: AiAssuranceProfile {
                assurance_case_id: "assure-1".to_string(),
                system_identifier: "system".to_string(),
                model_identifier: "model".to_string(),
                decision_traceability: "trace".to_string(),
                human_review_status: "complete".to_string(),
                assurance_outcome: "pass".to_string(),
                linked_axle_artifact_path: "axle.json".to_string(),
                linked_exposure_ids: vec!["exp-1".to_string()],
                linked_coverage_case_ids: vec!["cov-1".to_string()],
                referenced_risk_ids: vec!["risk-1".to_string()],
                referenced_control_ids: vec!["control-1".to_string()],
                risk_summaries: vec![RiskSummaryRef {
                    risk_id: "risk-1".to_string(),
                    title: "Model drift".to_string(),
                    severity: "high".to_string(),
                    status: "open".to_string(),
                }],
                control_summaries: vec![ControlSummaryRef {
                    control_id: "control-1".to_string(),
                    title: "Human review".to_string(),
                    control_type: "review".to_string(),
                    status: "active".to_string(),
                }],
            },
        }
    }

    fn risks() -> RiskRegistryDocument {
        RiskRegistryDocument {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            risks: vec![RiskRecord {
                risk_id: "risk-1".to_string(),
                title: "Model drift".to_string(),
                category: "operational".to_string(),
                severity: "high".to_string(),
                status: "open".to_string(),
                owner: "risk".to_string(),
                description: "desc".to_string(),
                linked_control_ids: vec!["control-1".to_string()],
                linked_profile_sections: vec![
                    InstitutionalProfileSection::Banking,
                    InstitutionalProfileSection::Insurance,
                    InstitutionalProfileSection::LegalGovernance,
                    InstitutionalProfileSection::AiAssurance,
                ],
            }],
        }
    }

    fn controls() -> ControlRegistryDocument {
        ControlRegistryDocument {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            controls: vec![ControlRecord {
                control_id: "control-1".to_string(),
                title: "Human review".to_string(),
                control_type: "review".to_string(),
                status: "active".to_string(),
                owner: "ops".to_string(),
                description: "desc".to_string(),
                evidence_paths: vec!["axle.json".to_string()],
                mitigated_risk_ids: vec!["risk-1".to_string()],
                linked_profile_sections: vec![
                    InstitutionalProfileSection::Banking,
                    InstitutionalProfileSection::Insurance,
                    InstitutionalProfileSection::LegalGovernance,
                    InstitutionalProfileSection::AiAssurance,
                ],
            }],
        }
    }

    #[test]
    fn policy_report_is_clean_for_valid_profiles() {
        let report =
            validate_institutional_profiles(&profiles(), &risks(), &controls(), "axle.json");
        assert!(report.banking_profile_verified);
        assert!(report.insurance_profile_verified);
        assert!(report.legal_governance_profile_verified);
        assert!(report.ai_assurance_profile_verified);
        assert!(report.cross_profile_consistency_verified);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn policy_report_detects_summary_mismatches() {
        let mut doc = profiles();
        doc.banking.risk_summaries[0].status = "closed".to_string();
        let report = validate_institutional_profiles(&doc, &risks(), &controls(), "axle.json");
        assert!(!report.banking_profile_verified);
        assert!(!report.findings.is_empty());
    }
}
