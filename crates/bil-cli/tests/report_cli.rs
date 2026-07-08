use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{NamedTempFile, TempDir};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn verify_proof_payload() -> &'static str {
    r#"{"okay":true,"content":"theorem foo : 1 = 1 := rfl","lean_messages":{"errors":[],"warnings":[],"infos":[]},"tool_messages":{"errors":[],"warnings":[],"infos":[]},"failed_declarations":[],"timings":{"total":10},"info":null}"#
}

fn create_phase1_bundle(temp: &TempDir) -> PathBuf {
    let payload = NamedTempFile::new_in(temp.path()).unwrap();
    fs::write(payload.path(), verify_proof_payload()).unwrap();
    let bundle_path = temp.path().join("proof.bil");

    let mut command = Command::cargo_bin("bil").unwrap();
    command
        .args([
            "bundle",
            "create",
            "--axle",
            payload.path().to_str().unwrap(),
            "--axle-kind",
            "verify-proof",
            "--out",
            bundle_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    bundle_path
}

#[test]
fn verification_report_markdown_renders_for_committed_example() {
    let root = repo_root();
    let bundle = root.join("examples/axle-proof-artifact/axle-proof-artifact.bil");
    let trust_key = root.join("examples/axle-proof-artifact/trust-key.der");

    let mut command = Command::cargo_bin("bil").unwrap();
    command
        .args([
            "report",
            bundle.to_str().unwrap(),
            "--format",
            "markdown",
            "--trust-key",
            trust_key.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# BIL Verification Report"))
        .stdout(predicate::str::contains("| Overall verified | `true` |"));
}

#[test]
fn verification_report_sarif_warns_for_untrusted_receipt() {
    let root = repo_root();
    let bundle = root.join("examples/axle-proof-artifact/axle-proof-artifact.bil");

    let mut command = Command::cargo_bin("bil").unwrap();
    command
        .args(["report", bundle.to_str().unwrap(), "--format", "sarif"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"receipt-untrusted\"",
        ))
        .stdout(predicate::str::contains("\"level\": \"warning\""));
}

#[test]
fn verification_report_sarif_errors_for_bundle_mismatch() {
    let temp = TempDir::new().unwrap();
    let bundle_path = create_phase1_bundle(&temp);
    let bundle_json_path = bundle_path.join("bundle.json");
    let mut bundle_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&bundle_json_path).unwrap()).unwrap();
    bundle_json["bundle_id"] = serde_json::Value::String("bil:v0:sha256:tampered".to_string());
    fs::write(&bundle_json_path, serde_json::to_vec(&bundle_json).unwrap()).unwrap();

    let output = Command::cargo_bin("bil")
        .unwrap()
        .args(["report", bundle_path.to_str().unwrap(), "--format", "sarif"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"ruleId\": \"bundle-id-mismatch\""));
    assert!(stdout.contains("\"level\": \"error\""));
}

#[test]
fn audit_report_markdown_matches_committed_example() {
    let root = repo_root();
    let expected = fs::read_to_string(
        root.join("examples/ai-decision-bundle/reports/audit-review-example.md"),
    )
    .unwrap();

    let output = Command::cargo_bin("bil")
        .unwrap()
        .current_dir(&root)
        .args([
            "report",
            "./examples/ai-decision-bundle/ai-decision-bundle.bil",
            "--kind",
            "audit",
            "--format",
            "markdown",
            "--trust-key",
            "./examples/ai-decision-bundle/trust-key.der",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn regulatory_report_markdown_matches_committed_example() {
    let root = repo_root();
    let expected = fs::read_to_string(
        root.join("examples/ai-decision-bundle/reports/regulatory-review-example.md"),
    )
    .unwrap();

    let output = Command::cargo_bin("bil")
        .unwrap()
        .current_dir(&root)
        .args([
            "report",
            "./examples/ai-decision-bundle/ai-decision-bundle.bil",
            "--kind",
            "regulatory",
            "--format",
            "markdown",
            "--trust-key",
            "./examples/ai-decision-bundle/trust-key.der",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn audit_report_json_renders_without_institutional_layer_and_exits_2() {
    let temp = TempDir::new().unwrap();
    let bundle_path = create_phase1_bundle(&temp);

    let output = Command::cargo_bin("bil")
        .unwrap()
        .args([
            "report",
            bundle_path.to_str().unwrap(),
            "--kind",
            "audit",
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"institutional_layer_present\": false"));
    assert!(stdout.contains("\"decision_context\": \"_not available_\""));
}

#[test]
fn invalid_sarif_combinations_exit_1() {
    let root = repo_root();
    let bundle = root.join("examples/ai-decision-bundle/ai-decision-bundle.bil");

    for kind in ["audit", "regulatory"] {
        let mut command = Command::cargo_bin("bil").unwrap();
        command
            .args([
                "report",
                bundle.to_str().unwrap(),
                "--kind",
                kind,
                "--format",
                "sarif",
            ])
            .assert()
            .failure()
            .code(1)
            .stderr(predicate::str::contains(
                "SARIF output is only supported for verification reports",
            ));
    }
}

#[test]
fn require_trust_failure_exits_2() {
    let root = repo_root();
    let bundle = root.join("examples/axle-proof-artifact/axle-proof-artifact.bil");

    let mut command = Command::cargo_bin("bil").unwrap();
    command
        .args([
            "report",
            bundle.to_str().unwrap(),
            "--require-trust",
            "--format",
            "json",
        ])
        .assert()
        .failure()
        .code(2)
        .stdout(predicate::str::contains("\"overall_verified\": false"));
}
