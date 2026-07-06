use bil_axle::{CheckResponse, VerifyProofResponse};
use bil_core::{AxleArtifact, AxleEvidenceRecord};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn verify_proof_example_roundtrips_through_typed_models() {
    let root = repo_root();
    let bytes =
        fs::read(root.join("examples/axle-proof-artifact/verify-proof-response.json")).unwrap();
    let response: VerifyProofResponse = serde_json::from_slice(&bytes).unwrap();
    let record = AxleEvidenceRecord::new(AxleArtifact::VerifyProof(response.clone())).unwrap();
    let decoded: AxleEvidenceRecord =
        serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();

    assert_eq!(decoded.artifact_kind.as_str(), "verify_proof");
    assert!(
        matches!(decoded.parse_artifact().unwrap(), AxleArtifact::VerifyProof(parsed) if parsed == response)
    );
}

#[test]
fn check_examples_roundtrip_through_typed_models() {
    let root = repo_root();
    for path in [
        root.join("examples/lean-proof-bundle/check-response.json"),
        root.join("examples/ai-decision-bundle/check-response.json"),
    ] {
        let bytes = fs::read(path).unwrap();
        let response: CheckResponse = serde_json::from_slice(&bytes).unwrap();
        let record = AxleEvidenceRecord::new(AxleArtifact::Check(response.clone())).unwrap();
        let decoded: AxleEvidenceRecord =
            serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();

        assert_eq!(decoded.artifact_kind.as_str(), "check");
        assert!(
            matches!(decoded.parse_artifact().unwrap(), AxleArtifact::Check(parsed) if parsed == response)
        );
    }
}
