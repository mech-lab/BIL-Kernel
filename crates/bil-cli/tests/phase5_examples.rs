use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn committed_example_bundles_inspect_successfully() {
    let root = repo_root();
    for bundle in [
        root.join("examples/axle-proof-artifact/axle-proof-artifact.bil"),
        root.join("examples/lean-proof-bundle/lean-proof-bundle.bil"),
        root.join("examples/ai-decision-bundle/ai-decision-bundle.bil"),
    ] {
        let mut command = Command::cargo_bin("bil").unwrap();
        command
            .args(["bundle", "inspect", bundle.to_str().unwrap()])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"overall_verified\": true"));
    }
}

#[test]
fn lean_example_detached_receipt_verifies_with_trust_key() {
    let root = repo_root();
    let bundle = root.join("examples/lean-proof-bundle/lean-proof-bundle.bil");
    let receipt = root.join("examples/lean-proof-bundle/lean-proof-bundle.receipt.json");
    let trust_key = root.join("examples/lean-proof-bundle/trust-key.der");

    let mut command = Command::cargo_bin("bil").unwrap();
    command
        .args([
            "bundle",
            "inspect",
            bundle.to_str().unwrap(),
            "--receipt",
            receipt.to_str().unwrap(),
            "--trust-key",
            trust_key.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"receipt_status\": \"verified\""))
        .stdout(predicate::str::contains("\"trust_verified\": true"));
}

#[test]
fn ai_decision_example_reports_full_institutional_status() {
    let root = repo_root();
    let bundle = root.join("examples/ai-decision-bundle/ai-decision-bundle.bil");
    let trust_key = root.join("examples/ai-decision-bundle/trust-key.der");

    let mut command = Command::cargo_bin("bil").unwrap();
    command
        .args([
            "bundle",
            "inspect",
            bundle.to_str().unwrap(),
            "--trust-key",
            trust_key.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"institutional_layer_present\": true",
        ))
        .stdout(predicate::str::contains(
            "\"banking_profile_verified\": true",
        ))
        .stdout(predicate::str::contains(
            "\"insurance_profile_verified\": true",
        ))
        .stdout(predicate::str::contains(
            "\"legal_governance_profile_verified\": true",
        ))
        .stdout(predicate::str::contains(
            "\"ai_assurance_profile_verified\": true",
        ))
        .stdout(predicate::str::contains("\"risk_registry_verified\": true"))
        .stdout(predicate::str::contains(
            "\"controls_registry_verified\": true",
        ))
        .stdout(predicate::str::contains(
            "\"cross_profile_consistency_verified\": true",
        ));
}

#[test]
fn phase5_templates_and_example_docs_exist() {
    let root = repo_root();
    for path in [
        "specs/assurance-interop-v0.md",
        "specs/bil-report-v0.md",
        "templates/reports/audit-review-v0.md",
        "templates/reports/regulatory-review-v0.md",
        "examples/axle-proof-artifact/README.md",
        "examples/lean-proof-bundle/README.md",
        "examples/ai-decision-bundle/README.md",
        "examples/ai-decision-bundle/reports/audit-review-example.md",
        "examples/ai-decision-bundle/reports/regulatory-review-example.md",
        "examples/keys/phase5-ed25519-public.der",
        "examples/axle-proof-artifact/trust-key.der",
        "examples/lean-proof-bundle/trust-key.der",
        "examples/ai-decision-bundle/trust-key.der",
    ] {
        assert!(root.join(path).exists(), "missing expected path: {path}");
    }
}
