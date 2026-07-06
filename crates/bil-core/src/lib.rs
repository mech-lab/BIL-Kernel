use bil_axle::{
    CheckResponse, DisproveResponse, ExtractDeclsResponse, ExtractTheoremsResponse,
    Have2LemmaResponse, Have2SorryResponse, MergeResponse, NormalizeResponse, RenameResponse,
    RepairProofsResponse, SimplifyTheoremsResponse, Sorry2LemmaResponse, Theorem2LemmaResponse,
    Theorem2SorryResponse, VerifyProofResponse,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt::{Display, Formatter};
use std::path::{Component, Path};
use std::str::FromStr;
use thiserror::Error;

pub const SCHEMA_VERSION_V0: &str = "v0";
pub const AXLE_COMPAT_PROFILE_V0: &str = "axle-compat-v0";
pub const INSTITUTIONAL_PROFILES_V0: &str = "institutional-profiles-v0";
pub const AXLE_JSON_PATH: &str = "axle.json";
pub const BUNDLE_JSON_PATH: &str = "bundle.json";
pub const MANIFEST_JSON_PATH: &str = "manifest.json";
pub const MERKLE_JSON_PATH: &str = "merkle.json";
pub const RECEIPT_JSON_PATH: &str = "receipt.json";
pub const INSTITUTIONAL_JSON_PATH: &str = "institutional.json";
pub const RISK_JSON_PATH: &str = "risk.json";
pub const CONTROLS_JSON_PATH: &str = "controls.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
pub enum AxleArtifactKind {
    VerifyProof,
    Check,
    Document,
    ExtractDecls,
    ExtractTheorems,
    Rename,
    Merge,
    Theorem2Sorry,
    Theorem2Lemma,
    SimplifyTheorems,
    RepairProofs,
    Have2Lemma,
    Have2Sorry,
    Sorry2Lemma,
    Disprove,
    Normalize,
}

impl AxleArtifactKind {
    pub const ALL: [Self; 16] = [
        Self::VerifyProof,
        Self::Check,
        Self::Document,
        Self::ExtractDecls,
        Self::ExtractTheorems,
        Self::Rename,
        Self::Merge,
        Self::Theorem2Sorry,
        Self::Theorem2Lemma,
        Self::SimplifyTheorems,
        Self::RepairProofs,
        Self::Have2Lemma,
        Self::Have2Sorry,
        Self::Sorry2Lemma,
        Self::Disprove,
        Self::Normalize,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::VerifyProof => "verify_proof",
            Self::Check => "check",
            Self::Document => "document",
            Self::ExtractDecls => "extract_decls",
            Self::ExtractTheorems => "extract_theorems",
            Self::Rename => "rename",
            Self::Merge => "merge",
            Self::Theorem2Sorry => "theorem2sorry",
            Self::Theorem2Lemma => "theorem2lemma",
            Self::SimplifyTheorems => "simplify_theorems",
            Self::RepairProofs => "repair_proofs",
            Self::Have2Lemma => "have2lemma",
            Self::Have2Sorry => "have2sorry",
            Self::Sorry2Lemma => "sorry2lemma",
            Self::Disprove => "disprove",
            Self::Normalize => "normalize",
        }
    }

    pub fn parse_payload(self, payload: Value) -> Result<AxleArtifact, CoreError> {
        match self {
            Self::VerifyProof => parse_variant(payload, self, AxleArtifact::VerifyProof),
            Self::Check => parse_variant(payload, self, AxleArtifact::Check),
            Self::Document => parse_variant(payload, self, AxleArtifact::Document),
            Self::ExtractDecls => parse_variant(payload, self, AxleArtifact::ExtractDecls),
            Self::ExtractTheorems => parse_variant(payload, self, AxleArtifact::ExtractTheorems),
            Self::Rename => parse_variant(payload, self, AxleArtifact::Rename),
            Self::Merge => parse_variant(payload, self, AxleArtifact::Merge),
            Self::Theorem2Sorry => parse_variant(payload, self, AxleArtifact::Theorem2Sorry),
            Self::Theorem2Lemma => parse_variant(payload, self, AxleArtifact::Theorem2Lemma),
            Self::SimplifyTheorems => parse_variant(payload, self, AxleArtifact::SimplifyTheorems),
            Self::RepairProofs => parse_variant(payload, self, AxleArtifact::RepairProofs),
            Self::Have2Lemma => parse_variant(payload, self, AxleArtifact::Have2Lemma),
            Self::Have2Sorry => parse_variant(payload, self, AxleArtifact::Have2Sorry),
            Self::Sorry2Lemma => parse_variant(payload, self, AxleArtifact::Sorry2Lemma),
            Self::Disprove => parse_variant(payload, self, AxleArtifact::Disprove),
            Self::Normalize => parse_variant(payload, self, AxleArtifact::Normalize),
        }
    }
}

impl Serialize for AxleArtifactKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for AxleArtifactKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(serde::de::Error::custom)
    }
}

impl Display for AxleArtifactKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AxleArtifactKind {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "verify_proof" | "verify-proof" => Ok(Self::VerifyProof),
            "check" => Ok(Self::Check),
            "document" => Ok(Self::Document),
            "extract_decls" | "extract-decls" => Ok(Self::ExtractDecls),
            "extract_theorems" | "extract-theorems" => Ok(Self::ExtractTheorems),
            "rename" => Ok(Self::Rename),
            "merge" => Ok(Self::Merge),
            "theorem2sorry" | "theorem2-sorry" => Ok(Self::Theorem2Sorry),
            "theorem2lemma" | "theorem2-lemma" => Ok(Self::Theorem2Lemma),
            "simplify_theorems" | "simplify-theorems" => Ok(Self::SimplifyTheorems),
            "repair_proofs" | "repair-proofs" => Ok(Self::RepairProofs),
            "have2lemma" | "have2-lemma" => Ok(Self::Have2Lemma),
            "have2sorry" | "have2-sorry" => Ok(Self::Have2Sorry),
            "sorry2lemma" | "sorry2-lemma" => Ok(Self::Sorry2Lemma),
            "disprove" => Ok(Self::Disprove),
            "normalize" => Ok(Self::Normalize),
            _ => Err(CoreError::UnknownArtifactKind(value.to_string())),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum AxleArtifact {
    VerifyProof(VerifyProofResponse),
    Check(CheckResponse),
    Document(bil_axle::Document),
    ExtractDecls(ExtractDeclsResponse),
    ExtractTheorems(ExtractTheoremsResponse),
    Rename(RenameResponse),
    Merge(MergeResponse),
    Theorem2Sorry(Theorem2SorryResponse),
    Theorem2Lemma(Theorem2LemmaResponse),
    SimplifyTheorems(SimplifyTheoremsResponse),
    RepairProofs(RepairProofsResponse),
    Have2Lemma(Have2LemmaResponse),
    Have2Sorry(Have2SorryResponse),
    Sorry2Lemma(Sorry2LemmaResponse),
    Disprove(DisproveResponse),
    Normalize(NormalizeResponse),
}

impl AxleArtifact {
    pub fn kind(&self) -> AxleArtifactKind {
        match self {
            Self::VerifyProof(_) => AxleArtifactKind::VerifyProof,
            Self::Check(_) => AxleArtifactKind::Check,
            Self::Document(_) => AxleArtifactKind::Document,
            Self::ExtractDecls(_) => AxleArtifactKind::ExtractDecls,
            Self::ExtractTheorems(_) => AxleArtifactKind::ExtractTheorems,
            Self::Rename(_) => AxleArtifactKind::Rename,
            Self::Merge(_) => AxleArtifactKind::Merge,
            Self::Theorem2Sorry(_) => AxleArtifactKind::Theorem2Sorry,
            Self::Theorem2Lemma(_) => AxleArtifactKind::Theorem2Lemma,
            Self::SimplifyTheorems(_) => AxleArtifactKind::SimplifyTheorems,
            Self::RepairProofs(_) => AxleArtifactKind::RepairProofs,
            Self::Have2Lemma(_) => AxleArtifactKind::Have2Lemma,
            Self::Have2Sorry(_) => AxleArtifactKind::Have2Sorry,
            Self::Sorry2Lemma(_) => AxleArtifactKind::Sorry2Lemma,
            Self::Disprove(_) => AxleArtifactKind::Disprove,
            Self::Normalize(_) => AxleArtifactKind::Normalize,
        }
    }

    pub fn payload_json(&self) -> Result<Value, CoreError> {
        match self {
            Self::VerifyProof(value) => to_json_value(value),
            Self::Check(value) => to_json_value(value),
            Self::Document(value) => to_json_value(value),
            Self::ExtractDecls(value) => to_json_value(value),
            Self::ExtractTheorems(value) => to_json_value(value),
            Self::Rename(value) => to_json_value(value),
            Self::Merge(value) => to_json_value(value),
            Self::Theorem2Sorry(value) => to_json_value(value),
            Self::Theorem2Lemma(value) => to_json_value(value),
            Self::SimplifyTheorems(value) => to_json_value(value),
            Self::RepairProofs(value) => to_json_value(value),
            Self::Have2Lemma(value) => to_json_value(value),
            Self::Have2Sorry(value) => to_json_value(value),
            Self::Sorry2Lemma(value) => to_json_value(value),
            Self::Disprove(value) => to_json_value(value),
            Self::Normalize(value) => to_json_value(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
pub struct AxleEvidenceRecord {
    pub schema_version: String,
    pub artifact_kind: AxleArtifactKind,
    pub payload: Value,
}

impl AxleEvidenceRecord {
    pub fn new(artifact: AxleArtifact) -> Result<Self, CoreError> {
        let artifact_kind = artifact.kind();
        let payload = artifact.payload_json()?;
        Ok(Self {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            artifact_kind,
            payload,
        })
    }

    pub fn parse_artifact(&self) -> Result<AxleArtifact, CoreError> {
        self.artifact_kind.parse_payload(self.payload.clone())
    }
}

impl Serialize for AxleEvidenceRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AxleEvidenceRecord", 3)?;
        state.serialize_field("schema_version", &self.schema_version)?;
        state.serialize_field("artifact_kind", &self.artifact_kind)?;
        state.serialize_field("payload", &self.payload)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for AxleEvidenceRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawRecord {
            schema_version: String,
            artifact_kind: AxleArtifactKind,
            payload: Value,
        }

        let raw = RawRecord::deserialize(deserializer)?;
        raw.artifact_kind
            .parse_payload(raw.payload.clone())
            .map_err(serde::de::Error::custom)?;

        Ok(Self {
            schema_version: raw.schema_version,
            artifact_kind: raw.artifact_kind,
            payload: raw.payload,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BundleKind {
    AxleEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CanonicalizationMode {
    JsonCanonicalV0,
    RawBytesV0,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct DigestSet {
    pub sha256: String,
    pub blake3: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ManifestEntry {
    pub logical_path: String,
    pub media_type: String,
    pub canonicalization: CanonicalizationMode,
    pub byte_length: u64,
    pub digests: DigestSet,
}

impl ManifestEntry {
    pub fn normalized(mut self) -> Result<Self, CoreError> {
        self.logical_path = normalize_logical_path(&self.logical_path)?;
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct BundleManifest {
    pub schema_version: String,
    pub entries: Vec<ManifestEntry>,
}

impl BundleManifest {
    pub fn normalized(&self) -> Result<Self, CoreError> {
        let mut entries = self
            .entries
            .iter()
            .cloned()
            .map(ManifestEntry::normalized)
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));

        let mut dedup = entries
            .iter()
            .map(|entry| entry.logical_path.as_str())
            .collect::<Vec<_>>();
        dedup.sort_unstable();
        dedup.windows(2).try_for_each(|pair| {
            if pair[0] == pair[1] {
                Err(CoreError::DuplicateLogicalPath(pair[0].to_string()))
            } else {
                Ok(())
            }
        })?;

        Ok(Self {
            schema_version: self.schema_version.clone(),
            entries,
        })
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum InstitutionalProfileSection {
    Banking,
    Insurance,
    LegalGovernance,
    AiAssurance,
}

impl InstitutionalProfileSection {
    pub const ALL: [Self; 4] = [
        Self::Banking,
        Self::Insurance,
        Self::LegalGovernance,
        Self::AiAssurance,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Banking => "banking",
            Self::Insurance => "insurance",
            Self::LegalGovernance => "legal_governance",
            Self::AiAssurance => "ai_assurance",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RiskSummaryRef {
    pub risk_id: String,
    pub title: String,
    pub severity: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ControlSummaryRef {
    pub control_id: String,
    pub title: String,
    pub control_type: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct BankingProfile {
    pub exposure_id: String,
    pub decision_context: String,
    pub counterparty: String,
    pub product_type: String,
    pub currency: String,
    pub exposure_amount: String,
    pub decision_outcome: String,
    pub review_status: String,
    pub governing_policy_refs: Vec<String>,
    pub referenced_risk_ids: Vec<String>,
    pub referenced_control_ids: Vec<String>,
    pub risk_summaries: Vec<RiskSummaryRef>,
    pub control_summaries: Vec<ControlSummaryRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct InsuranceProfile {
    pub coverage_case_id: String,
    pub coverage_context: String,
    pub insured_party: String,
    pub coverage_type: String,
    pub insured_amount: String,
    pub decision_outcome: String,
    pub review_status: String,
    pub policy_refs: Vec<String>,
    pub referenced_risk_ids: Vec<String>,
    pub referenced_control_ids: Vec<String>,
    pub risk_summaries: Vec<RiskSummaryRef>,
    pub control_summaries: Vec<ControlSummaryRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct LegalGovernanceProfile {
    pub matter_id: String,
    pub rights_and_duties_summary: String,
    pub liability_posture: String,
    pub compliance_posture: String,
    pub governing_authority_refs: Vec<String>,
    pub linked_exposure_ids: Vec<String>,
    pub linked_coverage_case_ids: Vec<String>,
    pub linked_assurance_case_ids: Vec<String>,
    pub referenced_risk_ids: Vec<String>,
    pub referenced_control_ids: Vec<String>,
    pub risk_summaries: Vec<RiskSummaryRef>,
    pub control_summaries: Vec<ControlSummaryRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AiAssuranceProfile {
    pub assurance_case_id: String,
    pub system_identifier: String,
    pub model_identifier: String,
    pub decision_traceability: String,
    pub human_review_status: String,
    pub assurance_outcome: String,
    pub linked_axle_artifact_path: String,
    pub linked_exposure_ids: Vec<String>,
    pub linked_coverage_case_ids: Vec<String>,
    pub referenced_risk_ids: Vec<String>,
    pub referenced_control_ids: Vec<String>,
    pub risk_summaries: Vec<RiskSummaryRef>,
    pub control_summaries: Vec<ControlSummaryRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct InstitutionalProfilesDocument {
    pub schema_version: String,
    pub banking: BankingProfile,
    pub insurance: InsuranceProfile,
    pub legal_governance: LegalGovernanceProfile,
    pub ai_assurance: AiAssuranceProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RiskRecord {
    pub risk_id: String,
    pub title: String,
    pub category: String,
    pub severity: String,
    pub status: String,
    pub owner: String,
    pub description: String,
    pub linked_control_ids: Vec<String>,
    pub linked_profile_sections: Vec<InstitutionalProfileSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ControlRecord {
    pub control_id: String,
    pub title: String,
    pub control_type: String,
    pub status: String,
    pub owner: String,
    pub description: String,
    pub evidence_paths: Vec<String>,
    pub mitigated_risk_ids: Vec<String>,
    pub linked_profile_sections: Vec<InstitutionalProfileSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RiskRegistryDocument {
    pub schema_version: String,
    pub risks: Vec<RiskRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ControlRegistryDocument {
    pub schema_version: String,
    pub controls: Vec<ControlRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct BundlePayloadPaths {
    pub axle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub institutional: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controls: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct BundleDescriptor {
    pub schema_version: String,
    pub bundle_kind: BundleKind,
    pub bundle_id: String,
    pub profile_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub institutional_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub institutional_profile_version: Option<String>,
    pub manifest_path: String,
    pub merkle_path: String,
    pub payload_paths: BundlePayloadPaths,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DigestAlgorithm {
    Sha256,
    Blake3,
}

impl Display for DigestAlgorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sha256 => f.write_str("sha256"),
            Self::Blake3 => f.write_str("blake3"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MerkleLeaf {
    pub logical_path: String,
    pub digests: DigestSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MerkleLevel {
    pub level: usize,
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MerkleTreeDocument {
    pub algorithm: DigestAlgorithm,
    pub root: String,
    pub levels: Vec<MerkleLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MerkleTrees {
    pub sha256: MerkleTreeDocument,
    pub blake3: MerkleTreeDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MerkleDocument {
    pub schema_version: String,
    pub leaf_order: Vec<String>,
    pub leaves: Vec<MerkleLeaf>,
    pub trees: MerkleTrees,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReceiptMode {
    Embedded,
    Detached,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CoverageScope {
    PreReceiptBundleFilesV0,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SignatureAlgorithm {
    Ed25519,
    EcdsaP256Sha256,
    RsaPssSha256,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CoveredFile {
    pub logical_path: String,
    pub byte_length: u64,
    pub digests: DigestSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ReceiptClaims {
    pub schema_version: String,
    pub receipt_mode: ReceiptMode,
    pub coverage_scope: CoverageScope,
    pub bundle_id: String,
    pub bundle_kind: BundleKind,
    pub profile_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub institutional_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub institutional_profile_version: Option<String>,
    pub issued_at: String,
    pub covered_files: Vec<CoveredFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ReceiptSignature {
    pub algorithm: SignatureAlgorithm,
    pub key_id: String,
    pub public_key_der_b64: String,
    pub signature_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ReceiptDocument {
    pub claims: ReceiptClaims,
    pub signature: ReceiptSignature,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReceiptVerificationStatus {
    Missing,
    Verified,
    Untrusted,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct VerificationFinding {
    pub code: String,
    pub message: String,
    pub logical_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct VerificationReport {
    pub schema_version: String,
    pub bundle_path: String,
    pub bundle_id: Option<String>,
    pub bundle_kind: Option<BundleKind>,
    pub profile_version: Option<String>,
    pub institutional_kind: Option<String>,
    pub institutional_profile_version: Option<String>,
    pub payload_count: usize,
    pub verified_entries: Vec<ManifestEntry>,
    pub merkle_roots: Option<DigestSet>,
    pub receipt_present: bool,
    pub receipt_status: ReceiptVerificationStatus,
    pub receipt_mode: Option<ReceiptMode>,
    pub receipt_path: Option<String>,
    pub signature_algorithm: Option<SignatureAlgorithm>,
    pub key_id: Option<String>,
    pub covered_file_count: usize,
    pub bundle_verified: bool,
    pub institutional_layer_present: bool,
    pub banking_profile_verified: bool,
    pub insurance_profile_verified: bool,
    pub legal_governance_profile_verified: bool,
    pub ai_assurance_profile_verified: bool,
    pub risk_registry_verified: bool,
    pub controls_registry_verified: bool,
    pub cross_profile_consistency_verified: bool,
    pub signature_valid: bool,
    pub trust_verified: bool,
    pub overall_verified: bool,
    pub findings: Vec<VerificationFinding>,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("unknown AXLE artifact kind: {0}")]
    UnknownArtifactKind(String),
    #[error("failed to serialize AXLE artifact payload: {0}")]
    SerializePayload(#[source] serde_json::Error),
    #[error("failed to deserialize {kind} payload: {source}")]
    DeserializePayload {
        kind: AxleArtifactKind,
        #[source]
        source: serde_json::Error,
    },
    #[error("logical path is invalid: {0}")]
    InvalidLogicalPath(String),
    #[error("logical path is duplicated in the manifest: {0}")]
    DuplicateLogicalPath(String),
}

pub fn normalize_logical_path(path: &str) -> Result<String, CoreError> {
    let candidate = Path::new(path);
    if candidate.as_os_str().is_empty() {
        return Err(CoreError::InvalidLogicalPath(path.to_string()));
    }

    let mut normalized = Vec::new();
    for component in candidate.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(value) => normalized.push(value.to_string_lossy().to_string()),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(CoreError::InvalidLogicalPath(path.to_string()));
            }
        }
    }

    if normalized.is_empty() {
        return Err(CoreError::InvalidLogicalPath(path.to_string()));
    }

    Ok(normalized.join("/"))
}

fn parse_variant<T, F>(
    payload: Value,
    kind: AxleArtifactKind,
    constructor: F,
) -> Result<AxleArtifact, CoreError>
where
    T: DeserializeOwned,
    F: FnOnce(T) -> AxleArtifact,
{
    serde_json::from_value(payload)
        .map(constructor)
        .map_err(|source| CoreError::DeserializePayload { kind, source })
}

fn to_json_value<T>(value: &T) -> Result<Value, CoreError>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(CoreError::SerializePayload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bil_axle::{Messages, VerifyProofResponse};
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn axle_record_roundtrips_typed_payload() {
        let artifact = AxleArtifact::VerifyProof(VerifyProofResponse {
            okay: true,
            content: "theorem foo : 1 = 1 := rfl".to_string(),
            lean_messages: Messages::default(),
            tool_messages: Messages::default(),
            failed_declarations: vec![],
            timings: BTreeMap::from([("total".to_string(), 10)]),
            info: None,
        });

        let record = AxleEvidenceRecord::new(artifact).unwrap();
        let bytes = serde_json::to_vec(&record).unwrap();
        let decoded: AxleEvidenceRecord = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(decoded.schema_version, SCHEMA_VERSION_V0);
        assert_eq!(decoded.artifact_kind, AxleArtifactKind::VerifyProof);
        assert!(matches!(
            decoded.parse_artifact().unwrap(),
            AxleArtifact::VerifyProof(_)
        ));
    }

    #[test]
    fn logical_paths_are_normalized_to_forward_slashes() {
        assert_eq!(
            normalize_logical_path("./nested/axle.json").unwrap(),
            "nested/axle.json"
        );
        assert!(normalize_logical_path("../axle.json").is_err());
        assert!(normalize_logical_path("/axle.json").is_err());
    }

    #[test]
    fn manifest_normalization_sorts_entries() {
        let manifest = BundleManifest {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            entries: vec![
                ManifestEntry {
                    logical_path: "z.json".to_string(),
                    media_type: "application/json".to_string(),
                    canonicalization: CanonicalizationMode::JsonCanonicalV0,
                    byte_length: 1,
                    digests: DigestSet::default(),
                },
                ManifestEntry {
                    logical_path: "./a.json".to_string(),
                    media_type: "application/json".to_string(),
                    canonicalization: CanonicalizationMode::JsonCanonicalV0,
                    byte_length: 1,
                    digests: DigestSet::default(),
                },
            ],
        };

        let normalized = manifest.normalized().unwrap();
        assert_eq!(normalized.entries[0].logical_path, "a.json");
        assert_eq!(normalized.entries[1].logical_path, "z.json");
    }

    #[test]
    fn axle_kind_accepts_cli_aliases() {
        assert_eq!(
            AxleArtifactKind::from_str("extract-decls").unwrap(),
            AxleArtifactKind::ExtractDecls
        );
        assert_eq!(
            AxleArtifactKind::from_str("verify_proof").unwrap(),
            AxleArtifactKind::VerifyProof
        );
    }

    #[test]
    fn axle_kind_document_variant_is_supported() {
        let payload = json!({
            "name": "foo",
            "kind": "theorem",
            "declaration": "theorem foo",
            "content": "theorem foo : 1 = 1 := rfl",
            "tokens": [],
            "signature": "foo : 1 = 1",
            "type": "Prop",
            "type_hash": 0,
            "type_depth": 0,
            "term_depth": 0,
            "is_sorry": false,
            "index": 0,
            "line_pos": 1,
            "end_line_pos": 1,
            "proof_length": 1,
            "tactic_counts": {},
            "wall_ms": 0,
            "heartbeats": 0,
            "local_type_dependencies": [],
            "local_value_dependencies": [],
            "external_type_dependencies": [],
            "external_value_dependencies": [],
            "local_syntactic_dependencies": [],
            "external_syntactic_dependencies": [],
            "declaration_messages": {"errors": [], "warnings": [], "infos": []},
            "theorem_messages": {"errors": [], "warnings": [], "infos": []}
        });

        let artifact = AxleArtifactKind::Document.parse_payload(payload).unwrap();
        assert!(matches!(artifact, AxleArtifact::Document(_)));
    }

    #[test]
    fn payload_paths_omit_absent_institutional_fields() {
        let descriptor = BundleDescriptor {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            bundle_kind: BundleKind::AxleEvidence,
            bundle_id: "bil:v0:sha256:abc".to_string(),
            profile_version: AXLE_COMPAT_PROFILE_V0.to_string(),
            institutional_kind: None,
            institutional_profile_version: None,
            manifest_path: MANIFEST_JSON_PATH.to_string(),
            merkle_path: MERKLE_JSON_PATH.to_string(),
            payload_paths: BundlePayloadPaths {
                axle: AXLE_JSON_PATH.to_string(),
                institutional: None,
                risk: None,
                controls: None,
            },
        };

        let value = serde_json::to_value(&descriptor).unwrap();
        assert!(value.get("institutional_kind").is_none());
        assert!(value["payload_paths"].get("institutional").is_none());
    }
}
