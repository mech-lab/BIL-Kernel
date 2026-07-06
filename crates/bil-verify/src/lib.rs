use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use bil_core::{
    BUNDLE_JSON_PATH, BundleDescriptor, BundleManifest, CONTROLS_JSON_PATH,
    ControlRegistryDocument, CoreError, CoverageScope, CoveredFile, DigestSet,
    INSTITUTIONAL_JSON_PATH, INSTITUTIONAL_PROFILES_V0, InstitutionalProfilesDocument,
    MERKLE_JSON_PATH, ManifestEntry, MerkleDocument, RECEIPT_JSON_PATH, RISK_JSON_PATH,
    ReceiptClaims, ReceiptMode, ReceiptVerificationStatus, RiskRegistryDocument,
    VerificationFinding, VerificationReport, normalize_logical_path,
};
use bil_hash::{HashError, canonical_json_slice, digest_bytes};
use bil_legal::validate_legal_links;
use bil_merkle::{MerkleError, build_manifest_tree};
use bil_policy::validate_institutional_profiles;
use bil_receipt::{
    ReceiptError, canonical_claims_bytes, embedded_receipt_path, key_id_from_public_key_der,
    verify_receipt_signature,
};
use bil_risk::validate_registries;
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

        let institutional = verify_institutional_layer(bundle_path, &descriptor)?;
        findings.extend(institutional.findings);

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

            let receipt: bil_core::ReceiptDocument = read_json(candidate.clone())?;
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
            if receipt.claims.institutional_kind != descriptor.institutional_kind {
                receipt_valid = false;
                findings.push(VerificationFinding {
                    code: "receipt-institutional-kind-mismatch".to_string(),
                    message: "receipt institutional kind does not match bundle.json".to_string(),
                    logical_path: receipt_path.clone(),
                });
            }
            if receipt.claims.institutional_profile_version
                != descriptor.institutional_profile_version
            {
                receipt_valid = false;
                findings.push(VerificationFinding {
                    code: "receipt-institutional-profile-version-mismatch".to_string(),
                    message: "receipt institutional profile version does not match bundle.json"
                        .to_string(),
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

        let institutional_valid = !institutional.institutional_layer_present
            || (institutional.banking_profile_verified
                && institutional.insurance_profile_verified
                && institutional.legal_governance_profile_verified
                && institutional.ai_assurance_profile_verified
                && institutional.risk_registry_verified
                && institutional.controls_registry_verified
                && institutional.cross_profile_consistency_verified);

        let overall_verified = bundle_verified
            && institutional_valid
            && (!options.require_receipt || receipt_present)
            && (!receipt_present || receipt_status != ReceiptVerificationStatus::Invalid)
            && (!options.require_trust || trust_verified);

        Ok(VerificationReport {
            schema_version: descriptor.schema_version.clone(),
            bundle_path: bundle_path.display().to_string(),
            bundle_id: Some(descriptor.bundle_id.clone()),
            bundle_kind: Some(descriptor.bundle_kind.clone()),
            profile_version: Some(descriptor.profile_version.clone()),
            institutional_kind: descriptor.institutional_kind.clone(),
            institutional_profile_version: descriptor.institutional_profile_version.clone(),
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
            institutional_layer_present: institutional.institutional_layer_present,
            banking_profile_verified: institutional.banking_profile_verified,
            insurance_profile_verified: institutional.insurance_profile_verified,
            legal_governance_profile_verified: institutional.legal_governance_profile_verified,
            ai_assurance_profile_verified: institutional.ai_assurance_profile_verified,
            risk_registry_verified: institutional.risk_registry_verified,
            controls_registry_verified: institutional.controls_registry_verified,
            cross_profile_consistency_verified: institutional.cross_profile_consistency_verified,
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

#[derive(Debug, Clone)]
struct InstitutionalVerificationState {
    institutional_layer_present: bool,
    banking_profile_verified: bool,
    insurance_profile_verified: bool,
    legal_governance_profile_verified: bool,
    ai_assurance_profile_verified: bool,
    risk_registry_verified: bool,
    controls_registry_verified: bool,
    cross_profile_consistency_verified: bool,
    findings: Vec<VerificationFinding>,
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

fn verify_institutional_layer(
    bundle_path: &Path,
    descriptor: &BundleDescriptor,
) -> Result<InstitutionalVerificationState, VerificationError> {
    let file_presence = [
        bundle_path.join(INSTITUTIONAL_JSON_PATH).exists(),
        bundle_path.join(RISK_JSON_PATH).exists(),
        bundle_path.join(CONTROLS_JSON_PATH).exists(),
    ];
    let payload_path_presence = [
        descriptor.payload_paths.institutional.is_some(),
        descriptor.payload_paths.risk.is_some(),
        descriptor.payload_paths.controls.is_some(),
    ];
    let marker_presence = [
        descriptor.institutional_kind.is_some(),
        descriptor.institutional_profile_version.is_some(),
    ];
    let institutional_layer_present = file_presence.into_iter().any(|value| value)
        || payload_path_presence.into_iter().any(|value| value)
        || marker_presence.into_iter().any(|value| value);

    if !institutional_layer_present {
        return Ok(InstitutionalVerificationState {
            institutional_layer_present: false,
            banking_profile_verified: false,
            insurance_profile_verified: false,
            legal_governance_profile_verified: false,
            ai_assurance_profile_verified: false,
            risk_registry_verified: false,
            controls_registry_verified: false,
            cross_profile_consistency_verified: false,
            findings: Vec::new(),
        });
    }

    let mut state = InstitutionalVerificationState {
        institutional_layer_present: true,
        banking_profile_verified: true,
        insurance_profile_verified: true,
        legal_governance_profile_verified: true,
        ai_assurance_profile_verified: true,
        risk_registry_verified: true,
        controls_registry_verified: true,
        cross_profile_consistency_verified: true,
        findings: Vec::new(),
    };

    if descriptor.institutional_kind.as_deref() != Some(INSTITUTIONAL_PROFILES_V0) {
        invalidate_all_institutional_statuses(&mut state);
        state.findings.push(VerificationFinding {
            code: "institutional-layer-kind-mismatch".to_string(),
            message: "bundle.json does not declare institutional_kind as institutional-profiles-v0"
                .to_string(),
            logical_path: Some(BUNDLE_JSON_PATH.to_string()),
        });
    }
    if descriptor.institutional_profile_version.as_deref() != Some(INSTITUTIONAL_PROFILES_V0) {
        invalidate_all_institutional_statuses(&mut state);
        state.findings.push(VerificationFinding {
            code: "institutional-layer-version-mismatch".to_string(),
            message:
                "bundle.json does not declare institutional_profile_version as institutional-profiles-v0"
                    .to_string(),
            logical_path: Some(BUNDLE_JSON_PATH.to_string()),
        });
    }

    let institutional_path = match descriptor.payload_paths.institutional.as_deref() {
        Some(path) if path == INSTITUTIONAL_JSON_PATH => path,
        Some(_) => {
            invalidate_all_institutional_statuses(&mut state);
            state.findings.push(VerificationFinding {
                code: "institutional-layer-path-mismatch".to_string(),
                message: "bundle.json institutional payload path must be institutional.json"
                    .to_string(),
                logical_path: Some(BUNDLE_JSON_PATH.to_string()),
            });
            INSTITUTIONAL_JSON_PATH
        }
        None => {
            invalidate_all_institutional_statuses(&mut state);
            state.findings.push(VerificationFinding {
                code: "institutional-layer-missing-payload-path".to_string(),
                message: "bundle.json is missing payload_paths.institutional".to_string(),
                logical_path: Some(BUNDLE_JSON_PATH.to_string()),
            });
            INSTITUTIONAL_JSON_PATH
        }
    };
    let risk_path = match descriptor.payload_paths.risk.as_deref() {
        Some(path) if path == RISK_JSON_PATH => path,
        Some(_) => {
            state.risk_registry_verified = false;
            state.cross_profile_consistency_verified = false;
            state.findings.push(VerificationFinding {
                code: "risk-registry-path-mismatch".to_string(),
                message: "bundle.json risk payload path must be risk.json".to_string(),
                logical_path: Some(BUNDLE_JSON_PATH.to_string()),
            });
            RISK_JSON_PATH
        }
        None => {
            state.risk_registry_verified = false;
            state.cross_profile_consistency_verified = false;
            state.findings.push(VerificationFinding {
                code: "risk-registry-missing-payload-path".to_string(),
                message: "bundle.json is missing payload_paths.risk".to_string(),
                logical_path: Some(BUNDLE_JSON_PATH.to_string()),
            });
            RISK_JSON_PATH
        }
    };
    let controls_path = match descriptor.payload_paths.controls.as_deref() {
        Some(path) if path == CONTROLS_JSON_PATH => path,
        Some(_) => {
            state.controls_registry_verified = false;
            state.cross_profile_consistency_verified = false;
            state.findings.push(VerificationFinding {
                code: "controls-registry-path-mismatch".to_string(),
                message: "bundle.json controls payload path must be controls.json".to_string(),
                logical_path: Some(BUNDLE_JSON_PATH.to_string()),
            });
            CONTROLS_JSON_PATH
        }
        None => {
            state.controls_registry_verified = false;
            state.cross_profile_consistency_verified = false;
            state.findings.push(VerificationFinding {
                code: "controls-registry-missing-payload-path".to_string(),
                message: "bundle.json is missing payload_paths.controls".to_string(),
                logical_path: Some(BUNDLE_JSON_PATH.to_string()),
            });
            CONTROLS_JSON_PATH
        }
    };

    let institutional_file = bundle_path.join(institutional_path);
    if !institutional_file.exists() {
        invalidate_all_institutional_statuses(&mut state);
        state.findings.push(VerificationFinding {
            code: "institutional-layer-missing-file".to_string(),
            message: "institutional.json is required for institutional bundles".to_string(),
            logical_path: Some(INSTITUTIONAL_JSON_PATH.to_string()),
        });
        return Ok(state);
    }
    let risk_file = bundle_path.join(risk_path);
    if !risk_file.exists() {
        state.risk_registry_verified = false;
        state.cross_profile_consistency_verified = false;
        state.findings.push(VerificationFinding {
            code: "risk-registry-missing-file".to_string(),
            message: "risk.json is required for institutional bundles".to_string(),
            logical_path: Some(RISK_JSON_PATH.to_string()),
        });
        return Ok(state);
    }
    let controls_file = bundle_path.join(controls_path);
    if !controls_file.exists() {
        state.controls_registry_verified = false;
        state.cross_profile_consistency_verified = false;
        state.findings.push(VerificationFinding {
            code: "controls-registry-missing-file".to_string(),
            message: "controls.json is required for institutional bundles".to_string(),
            logical_path: Some(CONTROLS_JSON_PATH.to_string()),
        });
        return Ok(state);
    }

    let institutional: InstitutionalProfilesDocument = read_json(institutional_file)?;
    let risk: RiskRegistryDocument = read_json(risk_file)?;
    let controls: ControlRegistryDocument = read_json(controls_file)?;

    let registry_report = validate_registries(&risk, &controls);
    state.risk_registry_verified = registry_report.risk_registry_verified;
    state.controls_registry_verified = registry_report.controls_registry_verified;
    state.findings.extend(registry_report.findings);

    let policy_report = validate_institutional_profiles(
        &institutional,
        &risk,
        &controls,
        &descriptor.payload_paths.axle,
    );
    state.banking_profile_verified = policy_report.banking_profile_verified;
    state.insurance_profile_verified = policy_report.insurance_profile_verified;
    state.legal_governance_profile_verified = policy_report.legal_governance_profile_verified;
    state.ai_assurance_profile_verified = policy_report.ai_assurance_profile_verified;
    state.cross_profile_consistency_verified = policy_report.cross_profile_consistency_verified;
    state.findings.extend(policy_report.findings);

    let legal_findings = validate_legal_links(&institutional);
    if !legal_findings.is_empty() {
        state.legal_governance_profile_verified = false;
        state.cross_profile_consistency_verified = false;
        state.findings.extend(legal_findings);
    }

    Ok(state)
}

fn invalidate_all_institutional_statuses(state: &mut InstitutionalVerificationState) {
    state.banking_profile_verified = false;
    state.insurance_profile_verified = false;
    state.legal_governance_profile_verified = false;
    state.ai_assurance_profile_verified = false;
    state.risk_registry_verified = false;
    state.controls_registry_verified = false;
    state.cross_profile_consistency_verified = false;
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
        AXLE_JSON_PATH, AxleArtifactKind, BUNDLE_JSON_PATH, CONTROLS_JSON_PATH,
        INSTITUTIONAL_JSON_PATH, MANIFEST_JSON_PATH, MERKLE_JSON_PATH, RECEIPT_JSON_PATH,
        RISK_JSON_PATH, ReceiptMode, SCHEMA_VERSION_V0, SignatureAlgorithm,
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

    fn institutional_inputs() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let institutional = serde_json::json!({
            "schema_version": SCHEMA_VERSION_V0,
            "banking": {
                "exposure_id": "exp-1",
                "decision_context": "ctx",
                "counterparty": "cp",
                "product_type": "loan",
                "currency": "USD",
                "exposure_amount": "100",
                "decision_outcome": "approved",
                "review_status": "reviewed",
                "governing_policy_refs": [],
                "referenced_risk_ids": ["risk-1"],
                "referenced_control_ids": ["control-1"],
                "risk_summaries": [{"risk_id":"risk-1","title":"Model drift","severity":"high","status":"open"}],
                "control_summaries": [{"control_id":"control-1","title":"Human review","control_type":"review","status":"active"}]
            },
            "insurance": {
                "coverage_case_id": "cov-1",
                "coverage_context": "ctx",
                "insured_party": "party",
                "coverage_type": "liability",
                "insured_amount": "100",
                "decision_outcome": "bound",
                "review_status": "reviewed",
                "policy_refs": [],
                "referenced_risk_ids": ["risk-1"],
                "referenced_control_ids": ["control-1"],
                "risk_summaries": [{"risk_id":"risk-1","title":"Model drift","severity":"high","status":"open"}],
                "control_summaries": [{"control_id":"control-1","title":"Human review","control_type":"review","status":"active"}]
            },
            "legal_governance": {
                "matter_id": "matter-1",
                "rights_and_duties_summary": "summary",
                "liability_posture": "contained",
                "compliance_posture": "compliant",
                "governing_authority_refs": [],
                "linked_exposure_ids": ["exp-1"],
                "linked_coverage_case_ids": ["cov-1"],
                "linked_assurance_case_ids": ["assure-1"],
                "referenced_risk_ids": ["risk-1"],
                "referenced_control_ids": ["control-1"],
                "risk_summaries": [{"risk_id":"risk-1","title":"Model drift","severity":"high","status":"open"}],
                "control_summaries": [{"control_id":"control-1","title":"Human review","control_type":"review","status":"active"}]
            },
            "ai_assurance": {
                "assurance_case_id": "assure-1",
                "system_identifier": "system",
                "model_identifier": "model",
                "decision_traceability": "trace",
                "human_review_status": "complete",
                "assurance_outcome": "pass",
                "linked_axle_artifact_path": AXLE_JSON_PATH,
                "linked_exposure_ids": ["exp-1"],
                "linked_coverage_case_ids": ["cov-1"],
                "referenced_risk_ids": ["risk-1"],
                "referenced_control_ids": ["control-1"],
                "risk_summaries": [{"risk_id":"risk-1","title":"Model drift","severity":"high","status":"open"}],
                "control_summaries": [{"control_id":"control-1","title":"Human review","control_type":"review","status":"active"}]
            }
        });
        let risk = serde_json::json!({
            "schema_version": SCHEMA_VERSION_V0,
            "risks": [{
                "risk_id":"risk-1",
                "title":"Model drift",
                "category":"operational",
                "severity":"high",
                "status":"open",
                "owner":"risk",
                "description":"desc",
                "linked_control_ids":["control-1"],
                "linked_profile_sections":["banking","insurance","legal_governance","ai_assurance"]
            }]
        });
        let controls = serde_json::json!({
            "schema_version": SCHEMA_VERSION_V0,
            "controls": [{
                "control_id":"control-1",
                "title":"Human review",
                "control_type":"review",
                "status":"active",
                "owner":"ops",
                "description":"desc",
                "evidence_paths":[AXLE_JSON_PATH],
                "mitigated_risk_ids":["risk-1"],
                "linked_profile_sections":["banking","insurance","legal_governance","ai_assurance"]
            }]
        });
        (
            serde_json::to_vec(&institutional).unwrap(),
            serde_json::to_vec(&risk).unwrap(),
            serde_json::to_vec(&controls).unwrap(),
        )
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
    fn verification_report_accepts_valid_phase_two_bundle() {
        let (_root, bundle_path) = build_bundle();
        let report = BundleVerifier::new()
            .verify(&bundle_path, &VerificationOptions::default())
            .unwrap();

        assert!(report.bundle_verified);
        assert!(report.overall_verified);
        assert!(!report.institutional_layer_present);
    }

    #[test]
    fn verification_report_detects_tampered_bundle() {
        let (_root, bundle_path) = build_bundle();
        fs::write(
            bundle_path.join(AXLE_JSON_PATH),
            br#"{"artifact_kind":"verify_proof","payload":{"okay":false},"schema_version":"v0"}"#,
        )
        .unwrap();

        let report = BundleVerifier::new()
            .verify(&bundle_path, &VerificationOptions::default())
            .unwrap();

        assert!(!report.bundle_verified);
        assert!(!report.overall_verified);
        assert!(!report.findings.is_empty());
    }

    #[test]
    fn embedded_receipts_verify_successfully() {
        let (_root, bundle_path) = build_bundle();
        let key_path = NamedTempFile::new().unwrap();
        let private_key_der = ed25519_private_key_der();
        fs::write(key_path.path(), &private_key_der).unwrap();

        ReceiptIssuer::new()
            .issue(
                &bundle_path,
                &private_key_der,
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

        assert!(report.receipt_present);
        assert_eq!(
            report.receipt_path.unwrap(),
            bundle_path.join(RECEIPT_JSON_PATH).display().to_string()
        );
        assert!(report.signature_valid);
        assert_eq!(report.receipt_status, ReceiptVerificationStatus::Untrusted);
    }

    #[test]
    fn institutional_bundles_require_consistent_documents() {
        let (_root, bundle_path) = build_bundle();
        let (institutional, risk, controls) = institutional_inputs();
        BundleBuilder::new()
            .institutionalize(&bundle_path, &institutional, &risk, &controls)
            .unwrap();

        let report = BundleVerifier::new()
            .verify(&bundle_path, &VerificationOptions::default())
            .unwrap();

        assert!(report.institutional_layer_present);
        assert!(report.banking_profile_verified);
        assert!(report.insurance_profile_verified);
        assert!(report.legal_governance_profile_verified);
        assert!(report.ai_assurance_profile_verified);
        assert!(report.risk_registry_verified);
        assert!(report.controls_registry_verified);
        assert!(report.cross_profile_consistency_verified);
        assert!(report.overall_verified);
    }

    #[test]
    fn institutional_tampering_is_reported() {
        let (_root, bundle_path) = build_bundle();
        let (institutional, risk, controls) = institutional_inputs();
        BundleBuilder::new()
            .institutionalize(&bundle_path, &institutional, &risk, &controls)
            .unwrap();

        fs::write(
            bundle_path.join(INSTITUTIONAL_JSON_PATH),
            br#"{"schema_version":"v0","banking":{},"insurance":{},"legal_governance":{},"ai_assurance":{}}"#,
        )
        .unwrap();

        let error = BundleVerifier::new().verify(&bundle_path, &VerificationOptions::default());
        assert!(matches!(error, Err(VerificationError::Json { .. })));
    }

    #[test]
    fn receipt_mismatch_is_detected_after_institutionalization() {
        let (_root, bundle_path) = build_bundle();
        let private_key_der = ed25519_private_key_der();
        ReceiptIssuer::new()
            .issue(
                &bundle_path,
                &private_key_der,
                ReceiptIssueOptions {
                    mode: ReceiptMode::Detached,
                    algorithm: SignatureAlgorithm::Ed25519,
                    issued_at: Some("2026-07-05T00:00:00Z".to_string()),
                    out: Some(bundle_path.parent().unwrap().join("proof.receipt.json")),
                },
            )
            .unwrap();

        let (institutional, risk, controls) = institutional_inputs();
        fs::remove_file(bundle_path.parent().unwrap().join("proof.receipt.json")).unwrap();
        BundleBuilder::new()
            .institutionalize(&bundle_path, &institutional, &risk, &controls)
            .unwrap();

        let private_key_der = ed25519_private_key_der();
        let detached_path = bundle_path.parent().unwrap().join("proof.receipt.json");
        ReceiptIssuer::new()
            .issue(
                &bundle_path,
                &private_key_der,
                ReceiptIssueOptions {
                    mode: ReceiptMode::Detached,
                    algorithm: SignatureAlgorithm::Ed25519,
                    issued_at: Some("2026-07-05T00:00:00Z".to_string()),
                    out: Some(detached_path.clone()),
                },
            )
            .unwrap();

        let report = BundleVerifier::new()
            .verify(
                &bundle_path,
                &VerificationOptions {
                    receipt_path: Some(detached_path),
                    ..VerificationOptions::default()
                },
            )
            .unwrap();

        assert!(report.receipt_present);
        assert_eq!(
            report.institutional_kind.as_deref(),
            Some(INSTITUTIONAL_PROFILES_V0)
        );
    }

    #[test]
    fn expected_control_documents_exist_in_institutional_bundle() {
        let (_root, bundle_path) = build_bundle();
        let (institutional, risk, controls) = institutional_inputs();
        BundleBuilder::new()
            .institutionalize(&bundle_path, &institutional, &risk, &controls)
            .unwrap();

        assert!(bundle_path.join(BUNDLE_JSON_PATH).exists());
        assert!(bundle_path.join(MANIFEST_JSON_PATH).exists());
        assert!(bundle_path.join(MERKLE_JSON_PATH).exists());
        assert!(bundle_path.join(INSTITUTIONAL_JSON_PATH).exists());
        assert!(bundle_path.join(RISK_JSON_PATH).exists());
        assert!(bundle_path.join(CONTROLS_JSON_PATH).exists());
    }
}
