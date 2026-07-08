use bil_core::{
    AXLE_COMPAT_PROFILE_V0, AXLE_JSON_PATH, AxleArtifactKind, AxleEvidenceRecord, BUNDLE_JSON_PATH,
    BundleDescriptor, BundleKind, BundleManifest, BundlePayloadPaths, CONTROLS_JSON_PATH,
    CanonicalizationMode, ControlRegistryDocument, CoreError, INSTITUTIONAL_JSON_PATH,
    INSTITUTIONAL_PROFILES_V0, InstitutionalProfilesDocument, MANIFEST_JSON_PATH, MERKLE_JSON_PATH,
    ManifestEntry, MerkleDocument, RECEIPT_JSON_PATH, RISK_JSON_PATH, RiskRegistryDocument,
    SCHEMA_VERSION_V0, VerificationFinding, VerificationReport,
};
use bil_hash::{HashError, canonical_json_bytes, digest_bytes};
use bil_legal::validate_legal_links;
use bil_merkle::{MerkleError, build_manifest_tree};
use bil_policy::validate_institutional_profiles;
use bil_receipt::default_detached_receipt_path;
use bil_risk::validate_registries;
use bil_verify::{BundleVerifier, VerificationError, VerificationOptions};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug)]
pub struct BundleBuilder;

impl BundleBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn create_axle_bundle<P>(
        &self,
        axle_kind: AxleArtifactKind,
        axle_json: &[u8],
        out_dir: P,
    ) -> Result<BundleMaterialization, BundleError>
    where
        P: AsRef<Path>,
    {
        let out_dir = out_dir.as_ref();
        validate_bundle_dir(out_dir)?;
        if out_dir.exists() {
            return Err(BundleError::OutputExists(out_dir.display().to_string()));
        }

        let payload: serde_json::Value = serde_json::from_slice(axle_json)?;
        let artifact = axle_kind.parse_payload(payload)?;
        let record = AxleEvidenceRecord::new(artifact)?;
        let axle_bytes = canonical_json_bytes(&record)?;

        let materialization = materialize_bundle(
            out_dir,
            AXLE_COMPAT_PROFILE_V0.to_string(),
            None,
            None,
            BundlePayloadSet {
                axle: PayloadDocument::new(AXLE_JSON_PATH, axle_bytes)?,
                institutional: None,
                risk: None,
                controls: None,
            },
        )?;

        Ok(BundleMaterialization {
            descriptor: materialization.descriptor,
            manifest: materialization.manifest,
            merkle: materialization.merkle,
            axle: record,
            output_dir: out_dir.to_path_buf(),
        })
    }

    pub fn institutionalize<P>(
        &self,
        bundle_dir: P,
        institutional_json: &[u8],
        risk_json: &[u8],
        controls_json: &[u8],
    ) -> Result<InstitutionalizationMaterialization, BundleError>
    where
        P: AsRef<Path>,
    {
        let bundle_dir = bundle_dir.as_ref();
        validate_bundle_dir(bundle_dir)?;
        if !bundle_dir.is_dir() {
            return Err(BundleError::InvalidBundleDirectory(
                bundle_dir.display().to_string(),
            ));
        }

        let existing_descriptor: BundleDescriptor = read_json(bundle_dir.join(BUNDLE_JSON_PATH))?;
        if existing_descriptor.payload_paths.institutional.is_some()
            || existing_descriptor.payload_paths.risk.is_some()
            || existing_descriptor.payload_paths.controls.is_some()
            || existing_descriptor.institutional_kind.is_some()
            || existing_descriptor.institutional_profile_version.is_some()
            || bundle_dir.join(INSTITUTIONAL_JSON_PATH).exists()
            || bundle_dir.join(RISK_JSON_PATH).exists()
            || bundle_dir.join(CONTROLS_JSON_PATH).exists()
        {
            return Err(BundleError::AlreadyInstitutionalized(
                bundle_dir.display().to_string(),
            ));
        }

        if bundle_dir.join(RECEIPT_JSON_PATH).exists() {
            return Err(BundleError::ReceiptExists(
                bundle_dir.join(RECEIPT_JSON_PATH).display().to_string(),
            ));
        }

        let default_detached = default_detached_receipt_path(bundle_dir);
        if default_detached.exists() {
            return Err(BundleError::ReceiptExists(
                default_detached.display().to_string(),
            ));
        }

        let institutional: InstitutionalProfilesDocument =
            serde_json::from_slice(institutional_json)?;
        let risk: RiskRegistryDocument = serde_json::from_slice(risk_json)?;
        let controls: ControlRegistryDocument = serde_json::from_slice(controls_json)?;
        validate_institutional_documents(&existing_descriptor, &institutional, &risk, &controls)?;

        let axle: AxleEvidenceRecord =
            read_json(bundle_dir.join(&existing_descriptor.payload_paths.axle))?;
        axle.parse_artifact()?;

        let axle_bytes = fs::read(bundle_dir.join(&existing_descriptor.payload_paths.axle))?;
        let institutional_bytes = canonical_json_bytes(&institutional)?;
        let risk_bytes = canonical_json_bytes(&risk)?;
        let controls_bytes = canonical_json_bytes(&controls)?;

        let materialization = materialize_bundle(
            bundle_dir,
            existing_descriptor.profile_version.clone(),
            Some(INSTITUTIONAL_PROFILES_V0.to_string()),
            Some(INSTITUTIONAL_PROFILES_V0.to_string()),
            BundlePayloadSet {
                axle: PayloadDocument::new(&existing_descriptor.payload_paths.axle, axle_bytes)?,
                institutional: Some(PayloadDocument::new(
                    INSTITUTIONAL_JSON_PATH,
                    institutional_bytes,
                )?),
                risk: Some(PayloadDocument::new(RISK_JSON_PATH, risk_bytes)?),
                controls: Some(PayloadDocument::new(CONTROLS_JSON_PATH, controls_bytes)?),
            },
        )?;

        Ok(InstitutionalizationMaterialization {
            descriptor: materialization.descriptor,
            manifest: materialization.manifest,
            merkle: materialization.merkle,
            output_dir: bundle_dir.to_path_buf(),
            previous_bundle_id: existing_descriptor.bundle_id,
            external_receipt_notice:
                "external detached receipts are invalidated by institutionalization and must be reissued"
                    .to_string(),
        })
    }
}

impl Default for BundleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct BundleReader {
    root: PathBuf,
}

impl BundleReader {
    pub fn open<P>(root: P) -> Result<Self, BundleError>
    where
        P: AsRef<Path>,
    {
        validate_bundle_dir(root.as_ref())?;
        if !root.as_ref().is_dir() {
            return Err(BundleError::InvalidBundleDirectory(
                root.as_ref().display().to_string(),
            ));
        }
        Ok(Self {
            root: root.as_ref().to_path_buf(),
        })
    }

    pub fn inspect(&self) -> Result<BundleInspection, BundleError> {
        self.inspect_with_options(&BundleInspectOptions::default())
    }

    pub fn inspect_with_options(
        &self,
        options: &BundleInspectOptions,
    ) -> Result<BundleInspection, BundleError> {
        BundleVerifier::new()
            .verify(&self.root, options)
            .map_err(BundleError::Verify)
    }

    pub fn report_context_with_options(
        &self,
        options: &BundleInspectOptions,
    ) -> Result<BundleInspectionContext, BundleError> {
        let verification = self.inspect_with_options(options)?;
        let descriptor: BundleDescriptor = read_json(self.root.join(BUNDLE_JSON_PATH))?;
        let manifest: BundleManifest = read_json(self.root.join(&descriptor.manifest_path))?;
        let merkle: MerkleDocument = read_json(self.root.join(&descriptor.merkle_path))?;
        let institutional = read_optional_json(
            &self.root,
            descriptor.payload_paths.institutional.as_deref(),
        )?;
        let risk = read_optional_json(&self.root, descriptor.payload_paths.risk.as_deref())?;
        let controls =
            read_optional_json(&self.root, descriptor.payload_paths.controls.as_deref())?;

        Ok(BundleInspectionContext {
            verification,
            descriptor,
            manifest,
            merkle,
            institutional,
            risk,
            controls,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BundleMaterialization {
    pub descriptor: BundleDescriptor,
    pub manifest: BundleManifest,
    pub merkle: MerkleDocument,
    pub axle: AxleEvidenceRecord,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InstitutionalizationMaterialization {
    pub descriptor: BundleDescriptor,
    pub manifest: BundleManifest,
    pub merkle: MerkleDocument,
    pub output_dir: PathBuf,
    pub previous_bundle_id: String,
    pub external_receipt_notice: String,
}

#[derive(Debug, Clone)]
pub struct BundleInspectionContext {
    pub verification: VerificationReport,
    pub descriptor: BundleDescriptor,
    pub manifest: BundleManifest,
    pub merkle: MerkleDocument,
    pub institutional: Option<InstitutionalProfilesDocument>,
    pub risk: Option<RiskRegistryDocument>,
    pub controls: Option<ControlRegistryDocument>,
}

#[derive(Debug, Clone)]
struct BundlePayloadSet {
    axle: PayloadDocument,
    institutional: Option<PayloadDocument>,
    risk: Option<PayloadDocument>,
    controls: Option<PayloadDocument>,
}

#[derive(Debug, Clone)]
struct PayloadDocument {
    path: String,
    bytes: Vec<u8>,
}

impl PayloadDocument {
    fn new(path: &str, bytes: Vec<u8>) -> Result<Self, BundleError> {
        Ok(Self {
            path: bil_core::normalize_logical_path(path)?,
            bytes,
        })
    }

    fn manifest_entry(&self) -> ManifestEntry {
        ManifestEntry {
            logical_path: self.path.clone(),
            media_type: "application/json".to_string(),
            canonicalization: CanonicalizationMode::JsonCanonicalV0,
            byte_length: self.bytes.len() as u64,
            digests: digest_bytes(&self.bytes),
        }
    }
}

#[derive(Debug, Clone)]
struct BundleFilesMaterialization {
    descriptor: BundleDescriptor,
    manifest: BundleManifest,
    merkle: MerkleDocument,
}

pub type BundleInspectOptions = VerificationOptions;
pub type BundleInspection = VerificationReport;

#[derive(Debug, Error)]
pub enum BundleError {
    #[error("bundle directories must end with .bil: {0}")]
    InvalidBundleDirectory(String),
    #[error("output path already exists: {0}")]
    OutputExists(String),
    #[error("failed to read or write bundle files: {0}")]
    Io(#[from] std::io::Error),
    #[error("bundle JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bundle core validation failed: {0}")]
    Core(#[from] CoreError),
    #[error("hashing failed: {0}")]
    Hash(#[from] HashError),
    #[error("merkle construction failed: {0}")]
    Merkle(#[from] MerkleError),
    #[error("bundle verification failed: {0}")]
    Verify(#[from] VerificationError),
    #[error("bundle already contains an institutional layer: {0}")]
    AlreadyInstitutionalized(String),
    #[error("bundle cannot be institutionalized while a receipt exists: {0}")]
    ReceiptExists(String),
    #[error("institutional profile validation failed: {0}")]
    InstitutionalValidation(String),
}

fn validate_bundle_dir(path: &Path) -> Result<(), BundleError> {
    let is_bil = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|name| name.ends_with(".bil"))
        .unwrap_or(false);
    if !is_bil {
        return Err(BundleError::InvalidBundleDirectory(
            path.display().to_string(),
        ));
    }
    Ok(())
}

fn materialize_bundle(
    out_dir: &Path,
    profile_version: String,
    institutional_kind: Option<String>,
    institutional_profile_version: Option<String>,
    payloads: BundlePayloadSet,
) -> Result<BundleFilesMaterialization, BundleError> {
    let mut entries = vec![payloads.axle.manifest_entry()];
    if let Some(institutional) = &payloads.institutional {
        entries.push(institutional.manifest_entry());
    }
    if let Some(risk) = &payloads.risk {
        entries.push(risk.manifest_entry());
    }
    if let Some(controls) = &payloads.controls {
        entries.push(controls.manifest_entry());
    }

    let manifest = BundleManifest {
        schema_version: SCHEMA_VERSION_V0.to_string(),
        entries,
    }
    .normalized()?;
    let merkle = build_manifest_tree(&manifest)?;
    let descriptor = BundleDescriptor {
        schema_version: SCHEMA_VERSION_V0.to_string(),
        bundle_kind: BundleKind::AxleEvidence,
        bundle_id: format!("bil:v0:sha256:{}", merkle.trees.sha256.root),
        profile_version,
        institutional_kind,
        institutional_profile_version,
        manifest_path: MANIFEST_JSON_PATH.to_string(),
        merkle_path: MERKLE_JSON_PATH.to_string(),
        payload_paths: BundlePayloadPaths {
            axle: payloads.axle.path.clone(),
            institutional: payloads
                .institutional
                .as_ref()
                .map(|value| value.path.clone()),
            risk: payloads.risk.as_ref().map(|value| value.path.clone()),
            controls: payloads.controls.as_ref().map(|value| value.path.clone()),
        },
    };

    fs::create_dir_all(out_dir)?;
    fs::write(out_dir.join(&payloads.axle.path), &payloads.axle.bytes)?;
    if let Some(institutional) = &payloads.institutional {
        fs::write(out_dir.join(&institutional.path), &institutional.bytes)?;
    }
    if let Some(risk) = &payloads.risk {
        fs::write(out_dir.join(&risk.path), &risk.bytes)?;
    }
    if let Some(controls) = &payloads.controls {
        fs::write(out_dir.join(&controls.path), &controls.bytes)?;
    }
    write_canonical_json(out_dir.join(MANIFEST_JSON_PATH), &manifest)?;
    write_canonical_json(out_dir.join(MERKLE_JSON_PATH), &merkle)?;
    write_canonical_json(out_dir.join(BUNDLE_JSON_PATH), &descriptor)?;

    Ok(BundleFilesMaterialization {
        descriptor,
        manifest,
        merkle,
    })
}

fn validate_institutional_documents(
    descriptor: &BundleDescriptor,
    institutional: &InstitutionalProfilesDocument,
    risk: &RiskRegistryDocument,
    controls: &ControlRegistryDocument,
) -> Result<(), BundleError> {
    let registry_report = validate_registries(risk, controls);
    let policy_report = validate_institutional_profiles(
        institutional,
        risk,
        controls,
        &descriptor.payload_paths.axle,
    );
    let legal_findings = validate_legal_links(institutional);

    let mut findings = Vec::<VerificationFinding>::new();
    findings.extend(registry_report.findings);
    findings.extend(policy_report.findings);
    findings.extend(legal_findings);

    if findings.is_empty() {
        Ok(())
    } else {
        Err(BundleError::InstitutionalValidation(render_findings(
            &findings,
        )))
    }
}

fn render_findings(findings: &[VerificationFinding]) -> String {
    findings
        .iter()
        .take(5)
        .map(|finding| format!("{}: {}", finding.code, finding.message))
        .collect::<Vec<_>>()
        .join("; ")
}

fn write_canonical_json<T>(path: PathBuf, value: &T) -> Result<(), BundleError>
where
    T: Serialize,
{
    let bytes = canonical_json_bytes(value)?;
    fs::write(path, bytes)?;
    Ok(())
}

fn read_json<T>(path: PathBuf) -> Result<T, BundleError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let bytes = fs::read(&path)?;
    serde_json::from_slice(&bytes).map_err(BundleError::Json)
}

fn read_optional_json<T>(root: &Path, path: Option<&str>) -> Result<Option<T>, BundleError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let Some(path) = path else {
        return Ok(None);
    };
    let resolved = root.join(path);
    if !resolved.exists() {
        return Ok(None);
    }
    read_json(resolved).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bil_axle::{Messages, VerifyProofResponse};
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    fn verify_payload() -> Vec<u8> {
        serde_json::to_vec(&VerifyProofResponse {
            okay: true,
            content: "theorem foo : 1 = 1 := rfl".to_string(),
            lean_messages: Messages::default(),
            tool_messages: Messages::default(),
            failed_declarations: vec![],
            timings: BTreeMap::from([("total".to_string(), 10)]),
            info: None,
        })
        .unwrap()
    }

    fn institutional_json() -> Vec<u8> {
        serde_json::to_vec(&InstitutionalProfilesDocument {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            banking: bil_core::BankingProfile {
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
                risk_summaries: vec![bil_core::RiskSummaryRef {
                    risk_id: "risk-1".to_string(),
                    title: "Model drift".to_string(),
                    severity: "high".to_string(),
                    status: "open".to_string(),
                }],
                control_summaries: vec![bil_core::ControlSummaryRef {
                    control_id: "control-1".to_string(),
                    title: "Human review".to_string(),
                    control_type: "review".to_string(),
                    status: "active".to_string(),
                }],
            },
            insurance: bil_core::InsuranceProfile {
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
                risk_summaries: vec![bil_core::RiskSummaryRef {
                    risk_id: "risk-1".to_string(),
                    title: "Model drift".to_string(),
                    severity: "high".to_string(),
                    status: "open".to_string(),
                }],
                control_summaries: vec![bil_core::ControlSummaryRef {
                    control_id: "control-1".to_string(),
                    title: "Human review".to_string(),
                    control_type: "review".to_string(),
                    status: "active".to_string(),
                }],
            },
            legal_governance: bil_core::LegalGovernanceProfile {
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
                risk_summaries: vec![bil_core::RiskSummaryRef {
                    risk_id: "risk-1".to_string(),
                    title: "Model drift".to_string(),
                    severity: "high".to_string(),
                    status: "open".to_string(),
                }],
                control_summaries: vec![bil_core::ControlSummaryRef {
                    control_id: "control-1".to_string(),
                    title: "Human review".to_string(),
                    control_type: "review".to_string(),
                    status: "active".to_string(),
                }],
            },
            ai_assurance: bil_core::AiAssuranceProfile {
                assurance_case_id: "assure-1".to_string(),
                system_identifier: "system".to_string(),
                model_identifier: "model".to_string(),
                decision_traceability: "trace".to_string(),
                human_review_status: "complete".to_string(),
                assurance_outcome: "pass".to_string(),
                linked_axle_artifact_path: AXLE_JSON_PATH.to_string(),
                linked_exposure_ids: vec!["exp-1".to_string()],
                linked_coverage_case_ids: vec!["cov-1".to_string()],
                referenced_risk_ids: vec!["risk-1".to_string()],
                referenced_control_ids: vec!["control-1".to_string()],
                risk_summaries: vec![bil_core::RiskSummaryRef {
                    risk_id: "risk-1".to_string(),
                    title: "Model drift".to_string(),
                    severity: "high".to_string(),
                    status: "open".to_string(),
                }],
                control_summaries: vec![bil_core::ControlSummaryRef {
                    control_id: "control-1".to_string(),
                    title: "Human review".to_string(),
                    control_type: "review".to_string(),
                    status: "active".to_string(),
                }],
            },
        })
        .unwrap()
    }

    fn risk_json() -> Vec<u8> {
        serde_json::to_vec(&RiskRegistryDocument {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            risks: vec![bil_core::RiskRecord {
                risk_id: "risk-1".to_string(),
                title: "Model drift".to_string(),
                category: "operational".to_string(),
                severity: "high".to_string(),
                status: "open".to_string(),
                owner: "risk".to_string(),
                description: "desc".to_string(),
                linked_control_ids: vec!["control-1".to_string()],
                linked_profile_sections: vec![
                    bil_core::InstitutionalProfileSection::Banking,
                    bil_core::InstitutionalProfileSection::Insurance,
                    bil_core::InstitutionalProfileSection::LegalGovernance,
                    bil_core::InstitutionalProfileSection::AiAssurance,
                ],
            }],
        })
        .unwrap()
    }

    fn controls_json() -> Vec<u8> {
        serde_json::to_vec(&ControlRegistryDocument {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            controls: vec![bil_core::ControlRecord {
                control_id: "control-1".to_string(),
                title: "Human review".to_string(),
                control_type: "review".to_string(),
                status: "active".to_string(),
                owner: "ops".to_string(),
                description: "desc".to_string(),
                evidence_paths: vec![AXLE_JSON_PATH.to_string()],
                mitigated_risk_ids: vec!["risk-1".to_string()],
                linked_profile_sections: vec![
                    bil_core::InstitutionalProfileSection::Banking,
                    bil_core::InstitutionalProfileSection::Insurance,
                    bil_core::InstitutionalProfileSection::LegalGovernance,
                    bil_core::InstitutionalProfileSection::AiAssurance,
                ],
            }],
        })
        .unwrap()
    }

    #[test]
    fn bundle_create_is_deterministic() {
        let root = tempdir().unwrap();
        let bundle_a = root.path().join("a.bil");
        let bundle_b = root.path().join("b.bil");
        let builder = BundleBuilder::new();

        builder
            .create_axle_bundle(AxleArtifactKind::VerifyProof, &verify_payload(), &bundle_a)
            .unwrap();
        builder
            .create_axle_bundle(AxleArtifactKind::VerifyProof, &verify_payload(), &bundle_b)
            .unwrap();

        for name in [
            AXLE_JSON_PATH,
            BUNDLE_JSON_PATH,
            MANIFEST_JSON_PATH,
            MERKLE_JSON_PATH,
        ] {
            let left = fs::read(bundle_a.join(name)).unwrap();
            let right = fs::read(bundle_b.join(name)).unwrap();
            assert_eq!(left, right, "{name} should be deterministic");
        }
    }

    #[test]
    fn bundle_inspect_verifies_valid_bundle() {
        let root = tempdir().unwrap();
        let bundle_path = root.path().join("evidence.bil");
        BundleBuilder::new()
            .create_axle_bundle(
                AxleArtifactKind::VerifyProof,
                &verify_payload(),
                &bundle_path,
            )
            .unwrap();

        let inspection = BundleReader::open(&bundle_path).unwrap().inspect().unwrap();
        assert!(inspection.bundle_verified);
        assert!(inspection.overall_verified);
        assert_eq!(inspection.payload_count, 1);
        assert_eq!(inspection.verified_entries[0].logical_path, AXLE_JSON_PATH);
    }

    #[test]
    fn bundle_inspect_detects_tampering() {
        let root = tempdir().unwrap();
        let bundle_path = root.path().join("tampered.bil");
        BundleBuilder::new()
            .create_axle_bundle(
                AxleArtifactKind::VerifyProof,
                &verify_payload(),
                &bundle_path,
            )
            .unwrap();

        fs::write(
            bundle_path.join(AXLE_JSON_PATH),
            br#"{"artifact_kind":"verify_proof","payload":{"okay":false},"schema_version":"v0"}"#,
        )
        .unwrap();

        let inspection = BundleReader::open(&bundle_path).unwrap().inspect().unwrap();
        assert!(!inspection.bundle_verified);
        assert!(!inspection.overall_verified);
        assert!(!inspection.findings.is_empty());
    }

    #[test]
    fn bundle_create_rejects_invalid_kind_payload_match() {
        let root = tempdir().unwrap();
        let bundle_path = root.path().join("invalid.bil");
        let error = BundleBuilder::new()
            .create_axle_bundle(AxleArtifactKind::Check, br#""not-an-object""#, &bundle_path)
            .unwrap_err();

        assert!(matches!(
            error,
            BundleError::Core(CoreError::DeserializePayload { .. })
        ));
    }

    #[test]
    fn institutionalize_rewrites_bundle_identity_and_payloads() {
        let root = tempdir().unwrap();
        let bundle_path = root.path().join("institutional.bil");
        let builder = BundleBuilder::new();
        let initial = builder
            .create_axle_bundle(
                AxleArtifactKind::VerifyProof,
                &verify_payload(),
                &bundle_path,
            )
            .unwrap();

        let materialized = builder
            .institutionalize(
                &bundle_path,
                &institutional_json(),
                &risk_json(),
                &controls_json(),
            )
            .unwrap();

        assert_ne!(
            initial.descriptor.bundle_id,
            materialized.descriptor.bundle_id
        );
        assert!(bundle_path.join(INSTITUTIONAL_JSON_PATH).exists());
        assert!(bundle_path.join(RISK_JSON_PATH).exists());
        assert!(bundle_path.join(CONTROLS_JSON_PATH).exists());
    }

    #[test]
    fn institutionalize_rejects_receipt_bearing_bundle_and_second_run() {
        let root = tempdir().unwrap();
        let bundle_path = root.path().join("institutional.bil");
        let builder = BundleBuilder::new();
        builder
            .create_axle_bundle(
                AxleArtifactKind::VerifyProof,
                &verify_payload(),
                &bundle_path,
            )
            .unwrap();

        fs::write(bundle_path.join(RECEIPT_JSON_PATH), b"{}").unwrap();
        let error = builder
            .institutionalize(
                &bundle_path,
                &institutional_json(),
                &risk_json(),
                &controls_json(),
            )
            .unwrap_err();
        assert!(matches!(error, BundleError::ReceiptExists(_)));
        fs::remove_file(bundle_path.join(RECEIPT_JSON_PATH)).unwrap();

        builder
            .institutionalize(
                &bundle_path,
                &institutional_json(),
                &risk_json(),
                &controls_json(),
            )
            .unwrap();
        let error = builder
            .institutionalize(
                &bundle_path,
                &institutional_json(),
                &risk_json(),
                &controls_json(),
            )
            .unwrap_err();
        assert!(matches!(error, BundleError::AlreadyInstitutionalized(_)));
    }
}
