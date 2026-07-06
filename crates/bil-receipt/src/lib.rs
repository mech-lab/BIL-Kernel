use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use bil_core::{
    CoreError, CoverageScope, CoveredFile, RECEIPT_JSON_PATH, ReceiptClaims, ReceiptDocument,
    ReceiptMode, ReceiptSignature, SCHEMA_VERSION_V0, SignatureAlgorithm, normalize_logical_path,
};
use bil_hash::{HashError, canonical_json_bytes, digest_bytes};
use chrono::{DateTime, SecondsFormat, Utc};
use rand::rngs::OsRng;
use signature::{RandomizedSigner, SignatureEncoding, Signer, Verifier};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ReceiptIssueOptions {
    pub mode: ReceiptMode,
    pub algorithm: SignatureAlgorithm,
    pub issued_at: Option<String>,
    pub out: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ReceiptMaterialization {
    pub receipt_path: PathBuf,
    pub document: ReceiptDocument,
    pub claims_bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct ReceiptIssuer;

impl ReceiptIssuer {
    pub fn new() -> Self {
        Self
    }

    pub fn issue<P>(
        &self,
        bundle_path: P,
        private_key_der: &[u8],
        options: ReceiptIssueOptions,
    ) -> Result<ReceiptMaterialization, ReceiptError>
    where
        P: AsRef<Path>,
    {
        let bundle_path = bundle_path.as_ref();
        validate_bundle_dir(bundle_path)?;

        let receipt_path = match options.mode {
            ReceiptMode::Embedded => {
                if options.out.is_some() {
                    return Err(ReceiptError::InvalidOutput(
                        "--out is not supported for embedded receipts".to_string(),
                    ));
                }
                let path = embedded_receipt_path(bundle_path);
                if path.exists() {
                    return Err(ReceiptError::OutputExists(path.display().to_string()));
                }
                path
            }
            ReceiptMode::Detached => {
                let path = options
                    .out
                    .clone()
                    .unwrap_or_else(|| default_detached_receipt_path(bundle_path));
                if path.exists() {
                    return Err(ReceiptError::OutputExists(path.display().to_string()));
                }
                path
            }
        };

        let descriptor_bytes = fs::read(bundle_path.join("bundle.json"))?;
        let descriptor: bil_core::BundleDescriptor = serde_json::from_slice(&descriptor_bytes)
            .map_err(|source| ReceiptError::Json {
                path: bundle_path.join("bundle.json").display().to_string(),
                source,
            })?;

        let covered_files = collect_covered_files(bundle_path, options.mode)?;
        let claims = ReceiptClaims {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            receipt_mode: options.mode,
            coverage_scope: CoverageScope::PreReceiptBundleFilesV0,
            bundle_id: descriptor.bundle_id,
            bundle_kind: descriptor.bundle_kind,
            profile_version: descriptor.profile_version,
            institutional_kind: descriptor.institutional_kind,
            institutional_profile_version: descriptor.institutional_profile_version,
            issued_at: normalize_issued_at(options.issued_at.as_deref())?,
            covered_files,
        };
        let claims_bytes = canonical_claims_bytes(&claims)?;
        let (public_key_der, signature_bytes) =
            sign_claims(&claims_bytes, options.algorithm, private_key_der)?;
        let key_id = key_id_from_public_key_der(&public_key_der);
        let document = ReceiptDocument {
            claims,
            signature: ReceiptSignature {
                algorithm: options.algorithm,
                key_id,
                public_key_der_b64: BASE64_STANDARD.encode(&public_key_der),
                signature_b64: BASE64_STANDARD.encode(&signature_bytes),
            },
        };
        let receipt_bytes = canonical_json_bytes(&document)?;
        fs::write(&receipt_path, receipt_bytes)?;

        Ok(ReceiptMaterialization {
            receipt_path,
            document,
            claims_bytes,
        })
    }
}

impl Default for ReceiptIssuer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn embedded_receipt_path(bundle_path: &Path) -> PathBuf {
    bundle_path.join(RECEIPT_JSON_PATH)
}

pub fn default_detached_receipt_path(bundle_path: &Path) -> PathBuf {
    let parent = bundle_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = bundle_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("bundle");
    parent.join(format!("{stem}.receipt.json"))
}

pub fn canonical_claims_bytes(claims: &ReceiptClaims) -> Result<Vec<u8>, ReceiptError> {
    canonical_json_bytes(claims).map_err(ReceiptError::Hash)
}

pub fn key_id_from_public_key_der(public_key_der: &[u8]) -> String {
    format!("sha256:{}", digest_bytes(public_key_der).sha256)
}

pub fn verify_receipt_signature(document: &ReceiptDocument) -> Result<(), ReceiptError> {
    let claims_bytes = canonical_claims_bytes(&document.claims)?;
    let public_key_der = decode_base64(
        &document.signature.public_key_der_b64,
        "receipt public key".to_string(),
    )?;
    let signature_bytes = decode_base64(
        &document.signature.signature_b64,
        "receipt signature".to_string(),
    )?;

    let expected_key_id = key_id_from_public_key_der(&public_key_der);
    if expected_key_id != document.signature.key_id {
        return Err(ReceiptError::KeyIdMismatch {
            expected: expected_key_id,
            actual: document.signature.key_id.clone(),
        });
    }

    verify_signature_bytes(
        &claims_bytes,
        document.signature.algorithm,
        &public_key_der,
        &signature_bytes,
    )
}

fn collect_covered_files(
    bundle_path: &Path,
    mode: ReceiptMode,
) -> Result<Vec<CoveredFile>, ReceiptError> {
    let mut files = WalkDir::new(bundle_path)
        .sort_by_file_name()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| ReceiptError::Walk {
            root: bundle_path.display().to_string(),
            source,
        })?
        .into_iter()
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let relative = entry.path().strip_prefix(bundle_path).ok()?;
            let logical_path = relative.to_string_lossy().replace('\\', "/");
            if mode == ReceiptMode::Embedded && logical_path == RECEIPT_JSON_PATH {
                None
            } else {
                Some((entry.path().to_path_buf(), logical_path))
            }
        })
        .map(|(path, logical_path)| {
            let logical_path = normalize_logical_path(&logical_path)?;
            let bytes = fs::read(&path)?;
            Ok(CoveredFile {
                logical_path,
                byte_length: bytes.len() as u64,
                digests: digest_bytes(&bytes),
            })
        })
        .collect::<Result<Vec<_>, ReceiptError>>()?;

    files.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    Ok(files)
}

fn normalize_issued_at(issued_at: Option<&str>) -> Result<String, ReceiptError> {
    match issued_at {
        Some(value) => {
            let parsed = DateTime::parse_from_rfc3339(value)
                .map_err(|_| ReceiptError::InvalidIssuedAt(value.to_string()))?;
            Ok(parsed
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Secs, true))
        }
        None => Ok(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)),
    }
}

fn sign_claims(
    claims_bytes: &[u8],
    algorithm: SignatureAlgorithm,
    private_key_der: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), ReceiptError> {
    match algorithm {
        SignatureAlgorithm::Ed25519 => {
            use ed25519_dalek::pkcs8::{DecodePrivateKey as _, EncodePublicKey as _};

            let signing_key =
                ed25519_dalek::SigningKey::from_pkcs8_der(private_key_der).map_err(|error| {
                    ReceiptError::InvalidPrivateKey {
                        algorithm,
                        message: error.to_string(),
                    }
                })?;
            let signature = signing_key.sign(claims_bytes).to_bytes().to_vec();
            let public_key_der = signing_key
                .verifying_key()
                .to_public_key_der()
                .map_err(|error| ReceiptError::InvalidPublicKey {
                    algorithm,
                    message: error.to_string(),
                })?
                .as_ref()
                .to_vec();
            Ok((public_key_der, signature))
        }
        SignatureAlgorithm::EcdsaP256Sha256 => {
            use p256::ecdsa::SigningKey;
            use p256::pkcs8::{DecodePrivateKey as _, EncodePublicKey as _};

            let signing_key = SigningKey::from_pkcs8_der(private_key_der).map_err(|error| {
                ReceiptError::InvalidPrivateKey {
                    algorithm,
                    message: error.to_string(),
                }
            })?;
            let signature: p256::ecdsa::Signature = signing_key.sign(claims_bytes);
            let public_key_der = signing_key
                .verifying_key()
                .to_public_key_der()
                .map_err(|error| ReceiptError::InvalidPublicKey {
                    algorithm,
                    message: error.to_string(),
                })?
                .as_ref()
                .to_vec();
            Ok((public_key_der, signature.to_der().as_bytes().to_vec()))
        }
        SignatureAlgorithm::RsaPssSha256 => {
            use rsa::pkcs8::{DecodePrivateKey as _, EncodePublicKey as _};

            let private_key =
                rsa::RsaPrivateKey::from_pkcs8_der(private_key_der).map_err(|error| {
                    ReceiptError::InvalidPrivateKey {
                        algorithm,
                        message: error.to_string(),
                    }
                })?;
            let signing_key = rsa::pss::BlindedSigningKey::<sha2::Sha256>::new(private_key.clone());
            let signature = signing_key.sign_with_rng(&mut OsRng, claims_bytes).to_vec();
            let public_key_der = rsa::RsaPublicKey::from(&private_key)
                .to_public_key_der()
                .map_err(|error| ReceiptError::InvalidPublicKey {
                    algorithm,
                    message: error.to_string(),
                })?
                .as_ref()
                .to_vec();
            Ok((public_key_der, signature))
        }
    }
}

fn verify_signature_bytes(
    claims_bytes: &[u8],
    algorithm: SignatureAlgorithm,
    public_key_der: &[u8],
    signature_bytes: &[u8],
) -> Result<(), ReceiptError> {
    match algorithm {
        SignatureAlgorithm::Ed25519 => {
            use ed25519_dalek::pkcs8::DecodePublicKey as _;

            let verifying_key = ed25519_dalek::VerifyingKey::from_public_key_der(public_key_der)
                .map_err(|error| ReceiptError::InvalidPublicKey {
                    algorithm,
                    message: error.to_string(),
                })?;
            let signature =
                ed25519_dalek::Signature::from_slice(signature_bytes).map_err(|error| {
                    ReceiptError::SignatureValidation {
                        algorithm,
                        message: error.to_string(),
                    }
                })?;
            verifying_key
                .verify(claims_bytes, &signature)
                .map_err(|error| ReceiptError::SignatureValidation {
                    algorithm,
                    message: error.to_string(),
                })
        }
        SignatureAlgorithm::EcdsaP256Sha256 => {
            use p256::pkcs8::DecodePublicKey as _;

            let verifying_key = p256::ecdsa::VerifyingKey::from_public_key_der(public_key_der)
                .map_err(|error| ReceiptError::InvalidPublicKey {
                    algorithm,
                    message: error.to_string(),
                })?;
            let signature = p256::ecdsa::Signature::from_der(signature_bytes).map_err(|error| {
                ReceiptError::SignatureValidation {
                    algorithm,
                    message: error.to_string(),
                }
            })?;
            verifying_key
                .verify(claims_bytes, &signature)
                .map_err(|error| ReceiptError::SignatureValidation {
                    algorithm,
                    message: error.to_string(),
                })
        }
        SignatureAlgorithm::RsaPssSha256 => {
            use rsa::pkcs8::DecodePublicKey as _;

            let public_key =
                rsa::RsaPublicKey::from_public_key_der(public_key_der).map_err(|error| {
                    ReceiptError::InvalidPublicKey {
                        algorithm,
                        message: error.to_string(),
                    }
                })?;
            let verifying_key = rsa::pss::VerifyingKey::<sha2::Sha256>::new(public_key);
            let signature = rsa::pss::Signature::try_from(signature_bytes).map_err(|error| {
                ReceiptError::SignatureValidation {
                    algorithm,
                    message: error.to_string(),
                }
            })?;
            verifying_key
                .verify(claims_bytes, &signature)
                .map_err(|error| ReceiptError::SignatureValidation {
                    algorithm,
                    message: error.to_string(),
                })
        }
    }
}

fn decode_base64(value: &str, subject: String) -> Result<Vec<u8>, ReceiptError> {
    BASE64_STANDARD
        .decode(value)
        .map_err(|error| ReceiptError::Base64 {
            subject,
            message: error.to_string(),
        })
}

fn validate_bundle_dir(path: &Path) -> Result<(), ReceiptError> {
    let is_bil = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|name| name.ends_with(".bil"))
        .unwrap_or(false);
    if !is_bil || !path.is_dir() {
        return Err(ReceiptError::InvalidBundleDirectory(
            path.display().to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ReceiptError {
    #[error("bundle directories must end with .bil: {0}")]
    InvalidBundleDirectory(String),
    #[error("receipt output already exists: {0}")]
    OutputExists(String),
    #[error("invalid receipt output configuration: {0}")]
    InvalidOutput(String),
    #[error("invalid RFC3339 issued_at timestamp: {0}")]
    InvalidIssuedAt(String),
    #[error("failed to read or write receipt files: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to walk bundle files under {root}: {source}")]
    Walk {
        root: String,
        #[source]
        source: walkdir::Error,
    },
    #[error("receipt JSON failed for {path}: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("receipt canonicalization failed: {0}")]
    Hash(#[from] HashError),
    #[error("invalid private key for {algorithm:?}: {message}")]
    InvalidPrivateKey {
        algorithm: SignatureAlgorithm,
        message: String,
    },
    #[error("invalid public key for {algorithm:?}: {message}")]
    InvalidPublicKey {
        algorithm: SignatureAlgorithm,
        message: String,
    },
    #[error("signature validation failed for {algorithm:?}: {message}")]
    SignatureValidation {
        algorithm: SignatureAlgorithm,
        message: String,
    },
    #[error("receipt key id mismatch: expected {expected}, found {actual}")]
    KeyIdMismatch { expected: String, actual: String },
    #[error("base64 decode failed for {subject}: {message}")]
    Base64 { subject: String, message: String },
    #[error("logical path normalization failed: {0}")]
    Core(#[from] CoreError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use bil_core::{BundleKind, DigestSet};
    use rsa::pkcs8::EncodePrivateKey as _;

    fn sample_claims() -> ReceiptClaims {
        ReceiptClaims {
            schema_version: SCHEMA_VERSION_V0.to_string(),
            receipt_mode: ReceiptMode::Detached,
            coverage_scope: CoverageScope::PreReceiptBundleFilesV0,
            bundle_id: "bil:v0:sha256:abc".to_string(),
            bundle_kind: BundleKind::AxleEvidence,
            profile_version: "axle-compat-v0".to_string(),
            institutional_kind: None,
            institutional_profile_version: None,
            issued_at: "2026-07-05T00:00:00Z".to_string(),
            covered_files: vec![CoveredFile {
                logical_path: "axle.json".to_string(),
                byte_length: 4,
                digests: DigestSet {
                    sha256: "a".repeat(64),
                    blake3: "b".repeat(64),
                },
            }],
        }
    }

    fn ed25519_private_key_der() -> Vec<u8> {
        use ed25519_dalek::pkcs8::EncodePrivateKey as _;

        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[7_u8; 32]);
        signing_key.to_pkcs8_der().unwrap().as_bytes().to_vec()
    }

    fn p256_private_key_der() -> Vec<u8> {
        use p256::pkcs8::EncodePrivateKey as _;

        let signing_key = p256::ecdsa::SigningKey::from_bytes((&[9_u8; 32]).into()).unwrap();
        signing_key.to_pkcs8_der().unwrap().as_bytes().to_vec()
    }

    fn rsa_private_key_der() -> Vec<u8> {
        let private_key = rsa::RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
        private_key.to_pkcs8_der().unwrap().as_bytes().to_vec()
    }

    #[test]
    fn claims_canonicalization_is_stable() {
        let claims = sample_claims();
        let bytes = canonical_claims_bytes(&claims).unwrap();
        assert_eq!(bytes, canonical_claims_bytes(&claims).unwrap());
    }

    #[test]
    fn key_id_and_der_round_trip_across_algorithms() {
        for (algorithm, private_key_der) in [
            (SignatureAlgorithm::Ed25519, ed25519_private_key_der()),
            (SignatureAlgorithm::EcdsaP256Sha256, p256_private_key_der()),
            (SignatureAlgorithm::RsaPssSha256, rsa_private_key_der()),
        ] {
            let claims_bytes = canonical_claims_bytes(&sample_claims()).unwrap();
            let (public_key_der, _) =
                sign_claims(&claims_bytes, algorithm, &private_key_der).unwrap();
            let key_id = key_id_from_public_key_der(&public_key_der);
            assert!(key_id.starts_with("sha256:"));
            assert_eq!(key_id.len(), "sha256:".len() + 64);
        }
    }

    #[test]
    fn each_algorithm_signs_and_verifies() {
        for (algorithm, private_key_der) in [
            (SignatureAlgorithm::Ed25519, ed25519_private_key_der()),
            (SignatureAlgorithm::EcdsaP256Sha256, p256_private_key_der()),
            (SignatureAlgorithm::RsaPssSha256, rsa_private_key_der()),
        ] {
            let claims = sample_claims();
            let claims_bytes = canonical_claims_bytes(&claims).unwrap();
            let (public_key_der, signature_bytes) =
                sign_claims(&claims_bytes, algorithm, &private_key_der).unwrap();
            let document = ReceiptDocument {
                claims: claims.clone(),
                signature: ReceiptSignature {
                    algorithm,
                    key_id: key_id_from_public_key_der(&public_key_der),
                    public_key_der_b64: BASE64_STANDARD.encode(&public_key_der),
                    signature_b64: BASE64_STANDARD.encode(&signature_bytes),
                },
            };
            verify_receipt_signature(&document).unwrap();
        }
    }

    #[test]
    fn wrong_key_and_corrupted_signature_fail() {
        let claims = sample_claims();
        let claims_bytes = canonical_claims_bytes(&claims).unwrap();
        let (public_key_der, signature_bytes) = sign_claims(
            &claims_bytes,
            SignatureAlgorithm::Ed25519,
            &ed25519_private_key_der(),
        )
        .unwrap();

        let wrong_public_key = sign_claims(&claims_bytes, SignatureAlgorithm::Ed25519, &{
            use ed25519_dalek::pkcs8::EncodePrivateKey as _;

            let signing_key = ed25519_dalek::SigningKey::from_bytes(&[8_u8; 32]);
            signing_key.to_pkcs8_der().unwrap().as_bytes().to_vec()
        })
        .unwrap()
        .0;

        let wrong_key_document = ReceiptDocument {
            claims: claims.clone(),
            signature: ReceiptSignature {
                algorithm: SignatureAlgorithm::Ed25519,
                key_id: key_id_from_public_key_der(&wrong_public_key),
                public_key_der_b64: BASE64_STANDARD.encode(&wrong_public_key),
                signature_b64: BASE64_STANDARD.encode(&signature_bytes),
            },
        };
        assert!(verify_receipt_signature(&wrong_key_document).is_err());

        let mut corrupted_signature = signature_bytes.clone();
        corrupted_signature[0] ^= 0x01;
        let corrupted_document = ReceiptDocument {
            claims,
            signature: ReceiptSignature {
                algorithm: SignatureAlgorithm::Ed25519,
                key_id: key_id_from_public_key_der(&public_key_der),
                public_key_der_b64: BASE64_STANDARD.encode(&public_key_der),
                signature_b64: BASE64_STANDARD.encode(&corrupted_signature),
            },
        };
        assert!(verify_receipt_signature(&corrupted_document).is_err());
    }
}
