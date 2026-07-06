use bil_core::{InstitutionalProfilesDocument, VerificationFinding};

pub fn validate_legal_links(profiles: &InstitutionalProfilesDocument) -> Vec<VerificationFinding> {
    let mut findings = Vec::new();
    let legal = &profiles.legal_governance;
    let banking = &profiles.banking;
    let insurance = &profiles.insurance;
    let assurance = &profiles.ai_assurance;

    for field in [
        ("matter_id", legal.matter_id.as_str()),
        (
            "rights_and_duties_summary",
            legal.rights_and_duties_summary.as_str(),
        ),
        ("liability_posture", legal.liability_posture.as_str()),
        ("compliance_posture", legal.compliance_posture.as_str()),
    ] {
        if field.1.trim().is_empty() {
            findings.push(VerificationFinding {
                code: "legal-governance-profile-empty-field".to_string(),
                message: format!("legal governance profile has an empty {}", field.0),
                logical_path: Some("institutional.json".to_string()),
            });
        }
    }

    for exposure_id in &legal.linked_exposure_ids {
        if exposure_id != &banking.exposure_id {
            findings.push(VerificationFinding {
                code: "legal-governance-profile-missing-exposure-link".to_string(),
                message: format!(
                    "legal governance profile references unknown exposure id {exposure_id}"
                ),
                logical_path: Some("institutional.json".to_string()),
            });
        }
    }

    for coverage_case_id in &legal.linked_coverage_case_ids {
        if coverage_case_id != &insurance.coverage_case_id {
            findings.push(VerificationFinding {
                code: "legal-governance-profile-missing-coverage-link".to_string(),
                message: format!(
                    "legal governance profile references unknown coverage case id {coverage_case_id}"
                ),
                logical_path: Some("institutional.json".to_string()),
            });
        }
    }

    for assurance_case_id in &legal.linked_assurance_case_ids {
        if assurance_case_id != &assurance.assurance_case_id {
            findings.push(VerificationFinding {
                code: "legal-governance-profile-missing-assurance-link".to_string(),
                message: format!(
                    "legal governance profile references unknown assurance case id {assurance_case_id}"
                ),
                logical_path: Some("institutional.json".to_string()),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use bil_core::{
        AiAssuranceProfile, BankingProfile, ControlSummaryRef, InstitutionalProfilesDocument,
        InsuranceProfile, LegalGovernanceProfile, RiskSummaryRef, SCHEMA_VERSION_V0,
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
                referenced_risk_ids: vec![],
                referenced_control_ids: vec![],
                risk_summaries: Vec::<RiskSummaryRef>::new(),
                control_summaries: Vec::<ControlSummaryRef>::new(),
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
                referenced_risk_ids: vec![],
                referenced_control_ids: vec![],
                risk_summaries: Vec::<RiskSummaryRef>::new(),
                control_summaries: Vec::<ControlSummaryRef>::new(),
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
                referenced_risk_ids: vec![],
                referenced_control_ids: vec![],
                risk_summaries: Vec::<RiskSummaryRef>::new(),
                control_summaries: Vec::<ControlSummaryRef>::new(),
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
                referenced_risk_ids: vec![],
                referenced_control_ids: vec![],
                risk_summaries: Vec::<RiskSummaryRef>::new(),
                control_summaries: Vec::<ControlSummaryRef>::new(),
            },
        }
    }

    #[test]
    fn legal_links_validate_when_targets_exist() {
        assert!(validate_legal_links(&profiles()).is_empty());
    }

    #[test]
    fn legal_links_fail_when_targets_are_missing() {
        let mut doc = profiles();
        doc.legal_governance.linked_exposure_ids = vec!["exp-404".to_string()];
        assert!(!validate_legal_links(&doc).is_empty());
    }
}
