use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use bil_core::{
    BUNDLE_JSON_PATH, BundleDescriptor, BundleManifest, CoreError, CoverageScope, CoveredFile,
    DigestSet, MERKLE_JSON_PATH, ManifestEntry, MerkleDocument, RECEIPT_JSON_PATH, ReceiptClaims,
    ReceiptDocument, ReceiptMode, ReceiptVerificationStatus, VerificationFinding,
    VerificationReport, normalize_logical_path,
};
use bil_hash::{HashError, canonical_json_slice, digest_bytes};
use bil_merkle::{MerkleError, build_manifest_tree};
use bil_receipt::{
    ReceiptError, canonical_claims_bytes, embedded_receipt_path, key_id_from_public_key_der,
    verify_receipt_signature,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Clone, Default)]
pub struct VerificationOptions {
    pub receipt_path: Option<PathBuf>,
    pub trust_key_paths: Vec<PathBuf>,
    pub require_receipt: bool,
    pub require_trust: bool,
}

#[derive(Debug)]
pub struct BundleVerifier;

impl BundleVerifier {
    pub fn new() -> Self {
        Self
    }

    pub fn verify<P>(
        &self,
        bundle_path: P,
        options: &VerificationOptions,
    ) -> Result<VerificationReport, VerificationError>
    where
        P: AsRef<Path>,
    {
        let bundle_path = bundle_path.as_ref();
        validate_bundle_dir(bundle_path)?;

        let descriptor: BundleDescriptor = read_json(bundle_path.join(BUNDLE_JSON_PATH))?;
        let manifest: BundleManifest =
            read_json(bundle_path.join(bundle_path_join(&descriptor.manifest_path)))?;
        let manifest = manifest.normalized()?;
        let merkle: MerkleDocument =
            read_json(bundle_path.join(bundle_path_join(&descriptor.merkle_path)))?;

        let mut findings = Vec::new();
        let mut bundle_verified = true;
        for entry in &manifest.entries {
            if let Some(finding) = verify_manifest_entry(bundle_path, entry) {
                bundle_verified = false;
                findings.push(finding);
            }
        }

        let axle_path = bundle_path.join(bundle_path_join(&descriptor.payload_paths.axle));
        if axle_path.exists() {
            let axle: bil_core::AxleEvidenceRecord = read_json(axle_path)?;
            axle.parse_artifact()?;
        }

        let rebuilt_merkle = build_manifest_tree(&manifest)?;
        if rebuilt_merkle != merkle {
            bundle_verified = false;
            findings.push(VerificationFinding {
                code: "merkle-mismatch".to_string(),
                message: "bundle merkle document does not match the manifest".to_string(),
                logical_path: Some(MERKLE_JSON_PATH.to_string()),
            });
        }

        let expected_bundle_id = format!("bil:v0:sha256:{}", merkle.trees.sha256.root);
        if descriptor.bundle_id != expected_bundle_id {
            bundle_verified = false;
            findings.push(VerificationFinding {
                code: "bundle-id-mismatch".to_string(),
                message: format!(
                    "expected bundle id {expected_bundle_id}, found {}",
                    descriptor.bundle_id
                ),
                logical_path: Some(BUNDLE_JSON_PATH.to_string()),
            });
        }

        let receipt_candidate = resolve_receipt_path(bundle_path, options);
        let mut receipt_present = false;
        let mut receipt_status = ReceiptVerificationStatus::Missing;
        let mut receipt_mode = None;
        let mut receipt_path = None;
        let mut signature_algorithm = None;
        let mut key_id = None;
        let mut covered_file_count = 0;
        let mut signature_valid = false;
        let mut trust_verified = false;

        if let Some(candidate) = receipt_candidate {
            receipt_present = true;
            receipt_path = Some(candidate.display().to_string());

            let receipt: ReceiptDocument = read_json(candidate.clone())?;
            validate_claims_stability(&receipt.claims)?;
            signature_algorithm = Some(receipt.signature.algorithm);
            key_id = Some(receipt.signature.key_id.clone());
            receipt_mode = Some(receipt.claims.receipt_mode);
            covered_file_count = receipt.claims.covered_files.len();

            let mut receipt_valid = true;
            if receipt.claims.coverage_scope != CoverageScope::PreReceiptBundleFilesV0 {
                receipt_valid = false;
                findings.push(VerificationFinding {
                    code: "unsupported-coverage-scope".to_string(),
                    message: "receipt coverage scope is not supported".to_string(),
                    logical_path: receipt_path.clone(),
                });
            }
            if receipt.claims.bundle_id != descriptor.bundle_id {
                receipt_valid = false;
                findings.push(VerificationFinding {
                    code: "receipt-bundle-id-mismatch".to_string(),
                    message: "receipt bundle id does not match bundle.json".to_string(),
                    logical_path: receipt_path.clone(),
                });
            }
            if receipt.claims.bundle_kind != descriptor.bundle_kind {
                receipt_valid = false;
                findings.push(VerificationFinding {
                    code: "receipt-bundle-kind-mismatch".to_string(),
                    message: "receipt bundle kind does not match bundle.json".to_string(),
                    logical_path: receipt_path.clone(),
                });
            }
            if receipt.claims.profile_version != descriptor.profile_version {
                receipt_valid = false;
                findings.push(VerificationFinding {
                    code: "receipt-profile-version-mismatch".to_string(),
                    message: "receipt profile version does not match bundle.json".to_string(),
                    logical_path: receipt_path.clone(),
                });
            }

            match verify_receipt_signature(&receipt) {
                Ok(()) => {
                    signature_valid = true;
                }
                Err(error) => {
                    receipt_valid = false;
                    findings.push(VerificationFinding {
                        code: "invalid-signature".to_string(),
                        message: error.to_string(),
                        logical_path: receipt_path.clone(),
                    });
                }
            }

            let actual_files =
                collect_actual_bundle_files(bundle_path, receipt.claims.receipt_mode)?;
            if let Some(completeness_findings) =
                verify_covered_files(&receipt.claims.covered_files, &actual_files)
            {
                receipt_valid = false;
                findings.extend(completeness_findings);
            }

            if signature_valid {
                let public_key_der = BASE64_STANDARD
                    .decode(&receipt.signature.public_key_der_b64)
                    .map_err(|error| VerificationError::Base64 {
                        subject: "receipt public key".to_string(),
                        message: error.to_string(),
                    })?;
                let expected_key_id = key_id_from_public_key_der(&public_key_der);
                if expected_key_id == receipt.signature.key_id {
                    let trust_keys = load_trust_keys(&options.trust_key_paths)?;
                    trust_verified = trust_keys
                        .iter()
                        .any(|candidate| candidate == &public_key_der);
                }
            }

            receipt_status = if !receipt_valid || !signature_valid {
                ReceiptVerificationStatus::Invalid
            } else if trust_verified {
                ReceiptVerificationStatus::Verified
            } else {
                findings.push(VerificationFinding {
                    code: "receipt-untrusted".to_string(),
                    message: "receipt signature is valid but no trusted public key matched"
                        .to_string(),
                    logical_path: receipt_path.clone(),
                });
                ReceiptVerificationStatus::Untrusted
            };
        } else if options.require_receipt {
            findings.push(VerificationFinding {
                code: "missing-receipt".to_string(),
                message: "receipt is required but none was found".to_string(),
                logical_path: None,
            });
        }

        if options.require_trust && !trust_verified {
            findings.push(VerificationFinding {
                code: "trust-required".to_string(),
                message: "receipt trust was required but no trusted key matched".to_string(),
                logical_path: receipt_path.clone(),
            });
        }

        let overall_verified = bundle_verified
            && (!options.require_receipt || receipt_present)
            && (!receipt_present || receipt_status != ReceiptVerificationStatus::Invalid)
            && (!options.require_trust || trust_verified);

        Ok(VerificationReport {
            schema_version: descriptor.schema_version,
            bundle_path: bundle_path.display().to_string(),
            bundle_id: Some(descriptor.bundle_id),
            bundle_kind: Some(descriptor.bundle_kind),
            profile_version: Some(descriptor.profile_version),
            payload_count: manifest.entries.len(),
            verified_entries: manifest.entries.clone(),
            merkle_roots: Some(DigestSet {
                sha256: merkle.trees.sha256.root,
                blake3: merkle.trees.blake3.root,
            }),
            receipt_present,
            receipt_status,
            receipt_mode,
            receipt_path,
            signature_algorithm,
            key_id,
            covered_file_count,
            bundle_verified,
            signature_valid,
            trust_verified,
            overall_verified,
            findings,
        })
    }
}

impl Default for BundleVerifier {
    fn default() -> Self {
        Self::new()
    }
}

fn verify_manifest_entry(bundle_path: &Path, entry: &ManifestEntry) -> Option<VerificationFinding> {
    let path = bundle_path.join(&entry.logical_path);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Some(VerificationFinding {
                code: "missing-manifest-file".to_string(),
                message: "manifest entry file is missing".to_string(),
                logical_path: Some(entry.logical_path.clone()),
            });
        }
    };

    let candidate = match entry.canonicalization {
        bil_core::CanonicalizationMode::JsonCanonicalV0 => match canonical_json_slice(&bytes) {
            Ok(candidate) => candidate,
            Err(error) => {
                return Some(VerificationFinding {
                    code: "invalid-canonical-json".to_string(),
                    message: error.to_string(),
                    logical_path: Some(entry.logical_path.clone()),
                });
            }
        },
        bil_core::CanonicalizationMode::RawBytesV0 => bytes.clone(),
    };

    if candidate != bytes {
        return Some(VerificationFinding {
            code: "non-canonical-payload".to_string(),
            message: "stored JSON payload bytes are not canonical".to_string(),
            logical_path: Some(entry.logical_path.clone()),
        });
    }
    if entry.byte_length != candidate.len() as u64 {
        return Some(VerificationFinding {
            code: "byte-length-mismatch".to_string(),
            message: "manifest byte length does not match stored bytes".to_string(),
            logical_path: Some(entry.logical_path.clone()),
        });
    }
    if digest_bytes(&candidate) != entry.digests {
        return Some(VerificationFinding {
            code: "digest-mismatch".to_string(),
            message: "manifest digests do not match stored bytes".to_string(),
            logical_path: Some(entry.logical_path.clone()),
        });
    }

    None
}

fn collect_actual_bundle_files(
    bundle_path: &Path,
    receipt_mode: ReceiptMode,
) -> Result<Vec<CoveredFile>, VerificationError> {
    let mut files = WalkDir::new(bundle_path)
        .sort_by_file_name()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| VerificationError::Walk {
            root: bundle_path.display().to_string(),
            source,
        })?
        .into_iter()
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let relative = entry.path().strip_prefix(bundle_path).ok()?;
            let logical_path = relative.to_string_lossy().replace('\\', "/");
            if receipt_mode == ReceiptMode::Embedded && logical_path == RECEIPT_JSON_PATH {
                None
            } else {
                Some((entry.path().to_path_buf(), logical_path))
            }
        })
        .map(|(path, logical_path)| {
            let logical_path = normalize_logical_path(&logical_path)?;
            let bytes = fs::read(path)?;
            Ok(CoveredFile {
                logical_path,
                byte_length: bytes.len() as u64,
                digests: digest_bytes(&bytes),
            })
        })
        .collect::<Result<Vec<_>, VerificationError>>()?;

    files.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    Ok(files)
}

fn verify_covered_files(
    covered_files: &[CoveredFile],
    actual_files: &[CoveredFile],
) -> Option<Vec<VerificationFinding>> {
    let covered_map = covered_files
        .iter()
        .map(|file| (file.logical_path.clone(), file))
        .collect::<BTreeMap<_, _>>();
    let actual_map = actual_files
        .iter()
        .map(|file| (file.logical_path.clone(), file))
        .collect::<BTreeMap<_, _>>();
    let mut findings = Vec::new();

    for (logical_path, covered) in &covered_map {
        match actual_map.get(logical_path) {
            None => findings.push(VerificationFinding {
                code: "missing-covered-file".to_string(),
                message: "receipt references a bundle file that is missing".to_string(),
                logical_path: Some(logical_path.clone()),
            }),
            Some(actual) => {
                if covered.byte_length != actual.byte_length || covered.digests != actual.digests {
                    findings.push(VerificationFinding {
                        code: "covered-file-mismatch".to_string(),
                        message: "covered file bytes do not match the receipt snapshot".to_string(),
                        logical_path: Some(logical_path.clone()),
                    });
                }
            }
        }
    }

    let covered_paths = covered_map.keys().cloned().collect::<BTreeSet<_>>();
    let actual_paths = actual_map.keys().cloned().collect::<BTreeSet<_>>();
    for logical_path in actual_paths.difference(&covered_paths) {
        findings.push(VerificationFinding {
            code: "unexpected-bundle-file".to_string(),
            message: "bundle contains a file that is not covered by the receipt".to_string(),
            logical_path: Some(logical_path.clone()),
        });
    }

    if findings.is_empty() {
        None
    } else {
        Some(findings)
    }
}

fn resolve_receipt_path(bundle_path: &Path, options: &VerificationOptions) -> Option<PathBuf> {
    if let Some(path) = &options.receipt_path {
        return Some(path.clone());
    }

    let embedded = embedded_receipt_path(bundle_path);
    embedded.exists().then_some(embedded)
}

fn load_trust_keys(paths: &[PathBuf]) -> Result<Vec<Vec<u8>>, VerificationError> {
    paths
        .iter()
        .map(|path| fs::read(path).map_err(VerificationError::Io))
        .collect()
}

fn validate_claims_stability(claims: &ReceiptClaims) -> Result<(), VerificationError> {
    let bytes = canonical_claims_bytes(claims)?;
    let parsed: ReceiptClaims =
        serde_json::from_slice(&bytes).map_err(|source| VerificationError::Json {
            path: "receipt claims".to_string(),
            source,
        })?;
    if &parsed == claims {
        Ok(())
    } else {
        Err(VerificationError::ClaimsStability)
    }
}

fn validate_bundle_dir(path: &Path) -> Result<(), VerificationError> {
    let is_bil = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|name| name.ends_with(".bil"))
        .unwrap_or(false);
    if !is_bil || !path.is_dir() {
        return Err(VerificationError::InvalidBundleDirectory(
            path.display().to_string(),
        ));
    }
    Ok(())
}

fn bundle_path_join(path: &str) -> String {
    path.to_string()
}

fn read_json<T>(path: PathBuf) -> Result<T, VerificationError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let bytes = fs::read(&path)?;
    serde_json::from_slice(&bytes).map_err(|source| VerificationError::Json {
        path: path.display().to_string(),
        source,
    })
}

#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("bundle directories must end with .bil: {0}")]
    InvalidBundleDirectory(String),
    #[error("failed to read bundle files: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to walk bundle files under {root}: {source}")]
    Walk {
        root: String,
        #[source]
        source: walkdir::Error,
    },
    #[error("failed to parse JSON at {path}: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("receipt claims do not reserialize stably")]
    ClaimsStability,
    #[error("bundle core validation failed: {0}")]
    Core(#[from] CoreError),
    #[error("canonical hashing failed: {0}")]
    Hash(#[from] HashError),
    #[error("merkle recomputation failed: {0}")]
    Merkle(#[from] MerkleError),
    #[error("receipt processing failed: {0}")]
    Receipt(#[from] ReceiptError),
    #[error("base64 decode failed for {subject}: {message}")]
    Base64 { subject: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use bil_axle::{Messages, VerifyProofResponse};
    use bil_bundle::BundleBuilder;
    use bil_core::{
        AXLE_JSON_PATH, AxleArtifactKind, BUNDLE_JSON_PATH, MANIFEST_JSON_PATH, MERKLE_JSON_PATH,
        ReceiptMode, SignatureAlgorithm,
    };
    use bil_receipt::{ReceiptIssueOptions, ReceiptIssuer};
    use ed25519_dalek::pkcs8::EncodePrivateKey as _;
    use std::collections::BTreeMap;
    use tempfile::{NamedTempFile, tempdir};

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

    fn ed25519_private_key_der() -> Vec<u8> {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[5_u8; 32]);
        signing_key.to_pkcs8_der().unwrap().as_bytes().to_vec()
    }

    fn build_bundle() -> (tempfile::TempDir, PathBuf) {
        let root = tempdir().unwrap();
        let bundle_path = root.path().join("proof.bil");
        BundleBuilder::new()
            .create_axle_bundle(
                AxleArtifactKind::VerifyProof,
                &verify_payload(),
                &bundle_path,
            )
            .unwrap();
        (root, bundle_path)
    }

    #[test]
    fn embedded_receipt_issues_and_verifies() {
        let (_root, bundle_path) = build_bundle();
        ReceiptIssuer::new()
            .issue(
                &bundle_path,
                &ed25519_private_key_der(),
                ReceiptIssueOptions {
                    mode: ReceiptMode::Embedded,
                    algorithm: SignatureAlgorithm::Ed25519,
                    issued_at: Some("2026-07-05T00:00:00Z".to_string()),
                    out: None,
                },
            )
            .unwrap();

        let report = BundleVerifier::new()
            .verify(&bundle_path, &VerificationOptions::default())
            .unwrap();
        assert!(report.bundle_verified);
        assert!(report.receipt_present);
        assert!(report.signature_valid);
        assert_eq!(report.receipt_status, ReceiptVerificationStatus::Untrusted);
        assert!(report.overall_verified);
    }

    #[test]
    fn detached_receipt_verifies_with_trust() {
        let (_root, bundle_path) = build_bundle();
        let materialized = ReceiptIssuer::new()
            .issue(
                &bundle_path,
                &ed25519_private_key_der(),
                ReceiptIssueOptions {
                    mode: ReceiptMode::Detached,
                    algorithm: SignatureAlgorithm::Ed25519,
                    issued_at: Some("2026-07-05T00:00:00Z".to_string()),
                    out: None,
                },
            )
            .unwrap();
        let public_key_der = BASE64_STANDARD
            .decode(&materialized.document.signature.public_key_der_b64)
            .unwrap();
        let trust_key = NamedTempFile::new().unwrap();
        fs::write(trust_key.path(), public_key_der).unwrap();

        let report = BundleVerifier::new()
            .verify(
                &bundle_path,
                &VerificationOptions {
                    receipt_path: Some(materialized.receipt_path),
                    trust_key_paths: vec![trust_key.path().to_path_buf()],
                    require_receipt: false,
                    require_trust: true,
                },
            )
            .unwrap();
        assert!(report.signature_valid);
        assert!(report.trust_verified);
        assert_eq!(report.receipt_status, ReceiptVerificationStatus::Verified);
        assert!(report.overall_verified);
    }

    #[test]
    fn tampering_each_phase_one_file_fails_verification() {
        for file_name in [
            AXLE_JSON_PATH,
            BUNDLE_JSON_PATH,
            MANIFEST_JSON_PATH,
            MERKLE_JSON_PATH,
        ] {
            let (_root, bundle_path) = build_bundle();
            ReceiptIssuer::new()
                .issue(
                    &bundle_path,
                    &ed25519_private_key_der(),
                    ReceiptIssueOptions {
                        mode: ReceiptMode::Embedded,
                        algorithm: SignatureAlgorithm::Ed25519,
                        issued_at: Some("2026-07-05T00:00:00Z".to_string()),
                        out: None,
                    },
                )
                .unwrap();

            fs::write(bundle_path.join(file_name), b"tampered").unwrap();
            let report = BundleVerifier::new()
                .verify(&bundle_path, &VerificationOptions::default())
                .unwrap_or_else(|_| VerificationReport {
                    schema_version: "v0".to_string(),
                    bundle_path: bundle_path.display().to_string(),
                    bundle_id: None,
                    bundle_kind: None,
                    profile_version: None,
                    payload_count: 0,
                    verified_entries: Vec::new(),
                    merkle_roots: None,
                    receipt_present: false,
                    receipt_status: ReceiptVerificationStatus::Invalid,
                    receipt_mode: None,
                    receipt_path: None,
                    signature_algorithm: None,
                    key_id: None,
                    covered_file_count: 0,
                    bundle_verified: false,
                    signature_valid: false,
                    trust_verified: false,
                    overall_verified: false,
                    findings: vec![VerificationFinding {
                        code: "parse-error".to_string(),
                        message: "bundle parse failed after tamper".to_string(),
                        logical_path: Some(file_name.to_string()),
                    }],
                });
            assert!(
                !report.overall_verified,
                "{file_name} should fail verification"
            );
        }
    }

    #[test]
    fn missing_covered_file_and_extra_file_are_detected() {
        let (_root, bundle_path) = build_bundle();
        ReceiptIssuer::new()
            .issue(
                &bundle_path,
                &ed25519_private_key_der(),
                ReceiptIssueOptions {
                    mode: ReceiptMode::Embedded,
                    algorithm: SignatureAlgorithm::Ed25519,
                    issued_at: Some("2026-07-05T00:00:00Z".to_string()),
                    out: None,
                },
            )
            .unwrap();

        fs::remove_file(bundle_path.join(AXLE_JSON_PATH)).unwrap();
        fs::write(bundle_path.join("extra.txt"), b"new").unwrap();
        let report = BundleVerifier::new()
            .verify(&bundle_path, &VerificationOptions::default())
            .unwrap();
        assert!(!report.overall_verified);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "missing-covered-file")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "unexpected-bundle-file")
        );
    }

    #[test]
    fn require_receipt_and_require_trust_are_enforced() {
        let (_root, bundle_path) = build_bundle();
        let missing_receipt = BundleVerifier::new()
            .verify(
                &bundle_path,
                &VerificationOptions {
                    require_receipt: true,
                    ..VerificationOptions::default()
                },
            )
            .unwrap();
        assert!(!missing_receipt.overall_verified);

        ReceiptIssuer::new()
            .issue(
                &bundle_path,
                &ed25519_private_key_der(),
                ReceiptIssueOptions {
                    mode: ReceiptMode::Embedded,
                    algorithm: SignatureAlgorithm::Ed25519,
                    issued_at: Some("2026-07-05T00:00:00Z".to_string()),
                    out: None,
                },
            )
            .unwrap();
        let untrusted = BundleVerifier::new()
            .verify(
                &bundle_path,
                &VerificationOptions {
                    require_trust: true,
                    ..VerificationOptions::default()
                },
            )
            .unwrap();
        assert!(!untrusted.overall_verified);
        assert!(untrusted.receipt_present);
        assert!(!untrusted.trust_verified);
    }
}
