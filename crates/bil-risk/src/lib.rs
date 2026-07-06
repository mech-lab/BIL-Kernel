use bil_core::{
    ControlRecord, ControlRegistryDocument, InstitutionalProfileSection, RiskRecord,
    RiskRegistryDocument, VerificationFinding, normalize_logical_path,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryValidationReport {
    pub risk_registry_verified: bool,
    pub controls_registry_verified: bool,
    pub findings: Vec<VerificationFinding>,
}

pub fn validate_registries(
    risk_registry: &RiskRegistryDocument,
    control_registry: &ControlRegistryDocument,
) -> RegistryValidationReport {
    let mut findings = Vec::new();
    let mut risk_registry_verified = true;
    let mut controls_registry_verified = true;

    let risk_map = build_risk_map(risk_registry, &mut findings, &mut risk_registry_verified);
    let control_map = build_control_map(
        control_registry,
        &mut findings,
        &mut controls_registry_verified,
    );

    for risk in risk_map.values() {
        validate_risk_record(risk, &mut findings, &mut risk_registry_verified);
        for control_id in &risk.linked_control_ids {
            match control_map.get(control_id) {
                None => {
                    risk_registry_verified = false;
                    findings.push(VerificationFinding {
                        code: "risk-registry-unknown-control".to_string(),
                        message: format!(
                            "risk {} references unknown control {control_id}",
                            risk.risk_id
                        ),
                        logical_path: Some("risk.json".to_string()),
                    });
                }
                Some(control) => {
                    if !control
                        .mitigated_risk_ids
                        .iter()
                        .any(|id| id == &risk.risk_id)
                    {
                        risk_registry_verified = false;
                        controls_registry_verified = false;
                        findings.push(VerificationFinding {
                            code: "risk-registry-nonreciprocal-link".to_string(),
                            message: format!(
                                "risk {} links control {control_id}, but the control does not reciprocate",
                                risk.risk_id
                            ),
                            logical_path: Some("risk.json".to_string()),
                        });
                    }
                }
            }
        }
    }

    for control in control_map.values() {
        validate_control_record(control, &mut findings, &mut controls_registry_verified);
        for risk_id in &control.mitigated_risk_ids {
            match risk_map.get(risk_id) {
                None => {
                    controls_registry_verified = false;
                    findings.push(VerificationFinding {
                        code: "controls-registry-unknown-risk".to_string(),
                        message: format!(
                            "control {} references unknown risk {risk_id}",
                            control.control_id
                        ),
                        logical_path: Some("controls.json".to_string()),
                    });
                }
                Some(risk) => {
                    if !risk
                        .linked_control_ids
                        .iter()
                        .any(|id| id == &control.control_id)
                    {
                        risk_registry_verified = false;
                        controls_registry_verified = false;
                        findings.push(VerificationFinding {
                            code: "controls-registry-nonreciprocal-link".to_string(),
                            message: format!(
                                "control {} mitigates risk {risk_id}, but the risk does not reciprocate",
                                control.control_id
                            ),
                            logical_path: Some("controls.json".to_string()),
                        });
                    }
                }
            }
        }
    }

    RegistryValidationReport {
        risk_registry_verified,
        controls_registry_verified,
        findings,
    }
}

fn build_risk_map<'a>(
    risk_registry: &'a RiskRegistryDocument,
    findings: &mut Vec<VerificationFinding>,
    risk_registry_verified: &mut bool,
) -> BTreeMap<String, &'a RiskRecord> {
    let mut seen = BTreeSet::new();
    let mut map = BTreeMap::new();

    for risk in &risk_registry.risks {
        if !seen.insert(risk.risk_id.clone()) {
            *risk_registry_verified = false;
            findings.push(VerificationFinding {
                code: "risk-registry-duplicate-id".to_string(),
                message: format!("duplicate risk id {}", risk.risk_id),
                logical_path: Some("risk.json".to_string()),
            });
            continue;
        }
        map.insert(risk.risk_id.clone(), risk);
    }

    map
}

fn build_control_map<'a>(
    control_registry: &'a ControlRegistryDocument,
    findings: &mut Vec<VerificationFinding>,
    controls_registry_verified: &mut bool,
) -> BTreeMap<String, &'a ControlRecord> {
    let mut seen = BTreeSet::new();
    let mut map = BTreeMap::new();

    for control in &control_registry.controls {
        if !seen.insert(control.control_id.clone()) {
            *controls_registry_verified = false;
            findings.push(VerificationFinding {
                code: "controls-registry-duplicate-id".to_string(),
                message: format!("duplicate control id {}", control.control_id),
                logical_path: Some("controls.json".to_string()),
            });
            continue;
        }
        map.insert(control.control_id.clone(), control);
    }

    map
}

fn validate_risk_record(
    risk: &RiskRecord,
    findings: &mut Vec<VerificationFinding>,
    risk_registry_verified: &mut bool,
) {
    for (field, value) in [
        ("risk_id", &risk.risk_id),
        ("title", &risk.title),
        ("category", &risk.category),
        ("severity", &risk.severity),
        ("status", &risk.status),
        ("owner", &risk.owner),
        ("description", &risk.description),
    ] {
        if value.trim().is_empty() {
            *risk_registry_verified = false;
            findings.push(VerificationFinding {
                code: "risk-registry-empty-field".to_string(),
                message: format!("risk {} has an empty {field}", risk.risk_id),
                logical_path: Some("risk.json".to_string()),
            });
        }
    }

    ensure_unique_sections(
        "risk-registry-duplicate-linked-section",
        "risk",
        &risk.risk_id,
        &risk.linked_profile_sections,
        findings,
        risk_registry_verified,
        "risk.json",
    );
}

fn validate_control_record(
    control: &ControlRecord,
    findings: &mut Vec<VerificationFinding>,
    controls_registry_verified: &mut bool,
) {
    for (field, value) in [
        ("control_id", &control.control_id),
        ("title", &control.title),
        ("control_type", &control.control_type),
        ("status", &control.status),
        ("owner", &control.owner),
        ("description", &control.description),
    ] {
        if value.trim().is_empty() {
            *controls_registry_verified = false;
            findings.push(VerificationFinding {
                code: "controls-registry-empty-field".to_string(),
                message: format!("control {} has an empty {field}", control.control_id),
                logical_path: Some("controls.json".to_string()),
            });
        }
    }

    ensure_unique_sections(
        "controls-registry-duplicate-linked-section",
        "control",
        &control.control_id,
        &control.linked_profile_sections,
        findings,
        controls_registry_verified,
        "controls.json",
    );

    for evidence_path in &control.evidence_paths {
        if normalize_logical_path(evidence_path).is_err() {
            *controls_registry_verified = false;
            findings.push(VerificationFinding {
                code: "controls-registry-invalid-evidence-path".to_string(),
                message: format!(
                    "control {} includes an invalid evidence path {evidence_path}",
                    control.control_id
                ),
                logical_path: Some("controls.json".to_string()),
            });
        }
    }
}

fn ensure_unique_sections(
    code: &str,
    label: &str,
    id: &str,
    sections: &[InstitutionalProfileSection],
    findings: &mut Vec<VerificationFinding>,
    verified: &mut bool,
    logical_path: &str,
) {
    let mut seen = BTreeSet::new();
    for section in sections {
        if !seen.insert(section) {
            *verified = false;
            findings.push(VerificationFinding {
                code: code.to_string(),
                message: format!(
                    "{label} {id} references {} more than once in linked_profile_sections",
                    section.as_str()
                ),
                logical_path: Some(logical_path.to_string()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bil_core::{
        ControlRegistryDocument, InstitutionalProfileSection, RiskRegistryDocument,
        SCHEMA_VERSION_V0,
    };

    fn valid_risk_registry() -> RiskRegistryDocument {
        RiskRegistryDocument {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            risks: vec![RiskRecord {
                risk_id: "risk-1".to_string(),
                title: "Model drift".to_string(),
                category: "operational".to_string(),
                severity: "high".to_string(),
                status: "open".to_string(),
                owner: "risk".to_string(),
                description: "description".to_string(),
                linked_control_ids: vec!["control-1".to_string()],
                linked_profile_sections: vec![InstitutionalProfileSection::AiAssurance],
            }],
        }
    }

    fn valid_control_registry() -> ControlRegistryDocument {
        ControlRegistryDocument {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            controls: vec![ControlRecord {
                control_id: "control-1".to_string(),
                title: "Human review".to_string(),
                control_type: "review".to_string(),
                status: "active".to_string(),
                owner: "ops".to_string(),
                description: "description".to_string(),
                evidence_paths: vec!["axle.json".to_string()],
                mitigated_risk_ids: vec!["risk-1".to_string()],
                linked_profile_sections: vec![InstitutionalProfileSection::AiAssurance],
            }],
        }
    }

    #[test]
    fn registries_validate_when_links_are_reciprocal() {
        let report = validate_registries(&valid_risk_registry(), &valid_control_registry());
        assert!(report.risk_registry_verified);
        assert!(report.controls_registry_verified);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn registries_detect_unknown_risks_and_controls() {
        let mut risks = valid_risk_registry();
        risks.risks[0].linked_control_ids = vec!["missing".to_string()];

        let report = validate_registries(&risks, &valid_control_registry());
        assert!(!report.risk_registry_verified);
        assert!(!report.findings.is_empty());
    }
}
