use bil_core::{
    AXLE_COMPAT_PROFILE_V0, AXLE_JSON_PATH, AxleArtifactKind, AxleEvidenceRecord, BUNDLE_JSON_PATH,
    BundleDescriptor, BundleKind, BundleManifest, BundlePayloadPaths, CanonicalizationMode,
    CoreError, MANIFEST_JSON_PATH, MERKLE_JSON_PATH, ManifestEntry, MerkleDocument,
    SCHEMA_VERSION_V0, VerificationReport,
};
use bil_hash::{HashError, canonical_json_bytes, digest_bytes};
use bil_merkle::{MerkleError, build_manifest_tree};
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
        let axle_digests = digest_bytes(&axle_bytes);
        let manifest = BundleManifest {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            entries: vec![ManifestEntry {
                logical_path: AXLE_JSON_PATH.to_string(),
                media_type: "application/json".to_string(),
                canonicalization: CanonicalizationMode::JsonCanonicalV0,
                byte_length: axle_bytes.len() as u64,
                digests: axle_digests.clone(),
            }],
        }
        .normalized()?;
        let merkle = build_manifest_tree(&manifest)?;
        let descriptor = BundleDescriptor {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            bundle_kind: BundleKind::AxleEvidence,
            bundle_id: format!("bil:v0:sha256:{}", merkle.trees.sha256.root),
            profile_version: AXLE_COMPAT_PROFILE_V0.to_string(),
            manifest_path: MANIFEST_JSON_PATH.to_string(),
            merkle_path: MERKLE_JSON_PATH.to_string(),
            payload_paths: BundlePayloadPaths {
                axle: AXLE_JSON_PATH.to_string(),
            },
        };

        fs::create_dir_all(out_dir)?;
        write_canonical_json(out_dir.join(AXLE_JSON_PATH), &record)?;
        write_canonical_json(out_dir.join(MANIFEST_JSON_PATH), &manifest)?;
        write_canonical_json(out_dir.join(MERKLE_JSON_PATH), &merkle)?;
        write_canonical_json(out_dir.join(BUNDLE_JSON_PATH), &descriptor)?;

        Ok(BundleMaterialization {
            descriptor,
            manifest,
            merkle,
            axle: record,
            output_dir: out_dir.to_path_buf(),
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
}

#[derive(Debug, Clone)]
pub struct BundleMaterialization {
    pub descriptor: BundleDescriptor,
    pub manifest: BundleManifest,
    pub merkle: MerkleDocument,
    pub axle: AxleEvidenceRecord,
    pub output_dir: PathBuf,
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

fn write_canonical_json<T>(path: PathBuf, value: &T) -> Result<(), BundleError>
where
    T: Serialize,
{
    let bytes = canonical_json_bytes(value)?;
    fs::write(path, bytes)?;
    Ok(())
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
}
