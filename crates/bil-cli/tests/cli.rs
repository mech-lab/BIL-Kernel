use assert_cmd::Command;
use ed25519_dalek::pkcs8::EncodePrivateKey as _;
use predicates::prelude::*;
use tempfile::{NamedTempFile, TempDir};

fn ed25519_private_key_der() -> Vec<u8> {
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[6_u8; 32]);
    signing_key.to_pkcs8_der().unwrap().as_bytes().to_vec()
}

fn institutional_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "v0",
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
            "linked_axle_artifact_path": "axle.json",
            "linked_exposure_ids": ["exp-1"],
            "linked_coverage_case_ids": ["cov-1"],
            "referenced_risk_ids": ["risk-1"],
            "referenced_control_ids": ["control-1"],
            "risk_summaries": [{"risk_id":"risk-1","title":"Model drift","severity":"high","status":"open"}],
            "control_summaries": [{"control_id":"control-1","title":"Human review","control_type":"review","status":"active"}]
        }
    })
}

fn risk_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "v0",
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
    })
}

fn controls_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "v0",
        "controls": [{
            "control_id":"control-1",
            "title":"Human review",
            "control_type":"review",
            "status":"active",
            "owner":"ops",
            "description":"desc",
            "evidence_paths":["axle.json"],
            "mitigated_risk_ids":["risk-1"],
            "linked_profile_sections":["banking","insurance","legal_governance","ai_assurance"]
        }]
    })
}

#[test]
fn status_prints_health_payload() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("GET", "/v1/status")
        .with_status(200)
        .with_body(r#"{"status":"healthy"}"#)
        .create();

    let mut command = Command::cargo_bin("bil").unwrap();
    command
        .env("AXLE_API_URL", server.url())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"healthy\""));
}

#[test]
fn environments_prints_environment_list() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("GET", "/v1/environments")
        .with_status(200)
        .with_body(r#"[{"name":"lean-4.28.0"}]"#)
        .create();

    let mut command = Command::cargo_bin("bil").unwrap();
    command
        .env("AXLE_API_URL", server.url())
        .arg("environments")
        .assert()
        .success()
        .stdout(predicate::str::contains("lean-4.28.0"));
}

#[test]
fn check_posts_content_and_prints_json() {
    let source = NamedTempFile::new().unwrap();
    std::fs::write(source.path(), "theorem foo : 1 = 1 := rfl").unwrap();

    let mut server = mockito::Server::new();
    let _mock = server
        .mock("POST", "/api/v1/check")
        .match_body(mockito::Matcher::PartialJson(serde_json::json!({
            "content": "theorem foo : 1 = 1 := rfl",
            "environment": "lean-4.28.0"
        })))
        .with_status(200)
        .with_body(
            r#"{"okay":true,"content":"theorem foo : 1 = 1 := rfl","lean_messages":{"errors":[],"warnings":[],"infos":[]},"tool_messages":{"errors":[],"warnings":[],"infos":[]},"failed_declarations":[],"timings":{"total":12},"info":null}"#,
        )
        .create();

    let mut command = Command::cargo_bin("bil").unwrap();
    command
        .env("AXLE_API_URL", server.url())
        .args([
            "axle",
            "check",
            source.path().to_str().unwrap(),
            "--environment",
            "lean-4.28.0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"okay\": true"))
        .stdout(predicate::str::contains("\"total\": 12"));
}

#[test]
fn hash_supports_canonical_json() {
    let source = NamedTempFile::new().unwrap();
    std::fs::write(source.path(), "{\n  \"b\": 2,\n  \"a\": 1\n}\n").unwrap();

    let mut command = Command::cargo_bin("bil").unwrap();
    command
        .args(["hash", source.path().to_str().unwrap(), "--canonical-json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"canonical_json\": true"))
        .stdout(predicate::str::contains("\"byte_length\": 13"))
        .stdout(predicate::str::contains("\"sha256\""));
}

#[test]
fn bundle_create_and_inspect_roundtrip() {
    let payload = NamedTempFile::new().unwrap();
    std::fs::write(
        payload.path(),
        r#"{"okay":true,"content":"theorem foo : 1 = 1 := rfl","lean_messages":{"errors":[],"warnings":[],"infos":[]},"tool_messages":{"errors":[],"warnings":[],"infos":[]},"failed_declarations":[],"timings":{"total":10},"info":null}"#,
    )
    .unwrap();
    let temp = TempDir::new().unwrap();
    let bundle_path = temp.path().join("proof.bil");

    let mut create = Command::cargo_bin("bil").unwrap();
    create
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
        .success()
        .stdout(predicate::str::contains(
            "\"bundle_kind\": \"axle-evidence\"",
        ))
        .stdout(predicate::str::contains("\"payload_count\": 1"));

    let mut inspect = Command::cargo_bin("bil").unwrap();
    inspect
        .args(["bundle", "inspect", bundle_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"bundle_verified\": true"))
        .stdout(predicate::str::contains("\"overall_verified\": true"))
        .stdout(predicate::str::contains("\"payload_count\": 1"));
}

#[test]
fn receipt_issue_supports_embedded_and_detached_modes() {
    let payload = NamedTempFile::new().unwrap();
    std::fs::write(
        payload.path(),
        r#"{"okay":true,"content":"theorem foo : 1 = 1 := rfl","lean_messages":{"errors":[],"warnings":[],"infos":[]},"tool_messages":{"errors":[],"warnings":[],"infos":[]},"failed_declarations":[],"timings":{"total":10},"info":null}"#,
    )
    .unwrap();
    let key = NamedTempFile::new().unwrap();
    std::fs::write(key.path(), ed25519_private_key_der()).unwrap();

    let temp = TempDir::new().unwrap();
    let embedded_bundle = temp.path().join("embedded.bil");
    let detached_bundle = temp.path().join("detached.bil");

    for bundle_path in [&embedded_bundle, &detached_bundle] {
        let mut create = Command::cargo_bin("bil").unwrap();
        create
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
    }

    let mut embedded_issue = Command::cargo_bin("bil").unwrap();
    embedded_issue
        .args([
            "receipt",
            "issue",
            embedded_bundle.to_str().unwrap(),
            "--mode",
            "embedded",
            "--algorithm",
            "ed25519",
            "--private-key",
            key.path().to_str().unwrap(),
            "--issued-at",
            "2026-07-05T00:00:00Z",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"receipt_mode\": \"embedded\""));

    let detached_receipt = temp.path().join("detached.custom.receipt.json");
    let mut detached_issue = Command::cargo_bin("bil").unwrap();
    detached_issue
        .args([
            "receipt",
            "issue",
            detached_bundle.to_str().unwrap(),
            "--mode",
            "detached",
            "--algorithm",
            "ed25519",
            "--private-key",
            key.path().to_str().unwrap(),
            "--issued-at",
            "2026-07-05T00:00:00Z",
            "--out",
            detached_receipt.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"receipt_mode\": \"detached\""));
}

#[test]
fn bundle_inspect_supports_markdown_and_verification_exit_code_two() {
    let payload = NamedTempFile::new().unwrap();
    std::fs::write(
        payload.path(),
        r#"{"okay":true,"content":"theorem foo : 1 = 1 := rfl","lean_messages":{"errors":[],"warnings":[],"infos":[]},"tool_messages":{"errors":[],"warnings":[],"infos":[]},"failed_declarations":[],"timings":{"total":10},"info":null}"#,
    )
    .unwrap();
    let key = NamedTempFile::new().unwrap();
    std::fs::write(key.path(), ed25519_private_key_der()).unwrap();

    let temp = TempDir::new().unwrap();
    let bundle_path = temp.path().join("proof.bil");
    let mut create = Command::cargo_bin("bil").unwrap();
    create
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

    let mut issue = Command::cargo_bin("bil").unwrap();
    issue
        .args([
            "receipt",
            "issue",
            bundle_path.to_str().unwrap(),
            "--mode",
            "embedded",
            "--algorithm",
            "ed25519",
            "--private-key",
            key.path().to_str().unwrap(),
            "--issued-at",
            "2026-07-05T00:00:00Z",
        ])
        .assert()
        .success();

    let mut markdown = Command::cargo_bin("bil").unwrap();
    markdown
        .args([
            "bundle",
            "inspect",
            bundle_path.to_str().unwrap(),
            "--format",
            "markdown",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# BIL Verification Report"));

    std::fs::write(
        bundle_path.join("axle.json"),
        br#"{"artifact_kind":"verify_proof","payload":{"okay":false},"schema_version":"v0"}"#,
    )
    .unwrap();
    let mut failed_inspect = Command::cargo_bin("bil").unwrap();
    failed_inspect
        .args(["bundle", "inspect", bundle_path.to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn bundle_institutionalize_updates_bundle_and_inspect_reports_statuses() {
    let payload = NamedTempFile::new().unwrap();
    std::fs::write(
        payload.path(),
        r#"{"okay":true,"content":"theorem foo : 1 = 1 := rfl","lean_messages":{"errors":[],"warnings":[],"infos":[]},"tool_messages":{"errors":[],"warnings":[],"infos":[]},"failed_declarations":[],"timings":{"total":10},"info":null}"#,
    )
    .unwrap();
    let institutional = NamedTempFile::new().unwrap();
    let risk = NamedTempFile::new().unwrap();
    let controls = NamedTempFile::new().unwrap();
    std::fs::write(
        institutional.path(),
        serde_json::to_vec(&institutional_json()).unwrap(),
    )
    .unwrap();
    std::fs::write(risk.path(), serde_json::to_vec(&risk_json()).unwrap()).unwrap();
    std::fs::write(
        controls.path(),
        serde_json::to_vec(&controls_json()).unwrap(),
    )
    .unwrap();

    let temp = TempDir::new().unwrap();
    let bundle_path = temp.path().join("proof.bil");
    let mut create = Command::cargo_bin("bil").unwrap();
    create
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

    let mut institutionalize = Command::cargo_bin("bil").unwrap();
    institutionalize
        .args([
            "bundle",
            "institutionalize",
            bundle_path.to_str().unwrap(),
            "--institutional",
            institutional.path().to_str().unwrap(),
            "--risk",
            risk.path().to_str().unwrap(),
            "--controls",
            controls.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"institutional_kind\": \"institutional-profiles-v0\"",
        ));

    let mut inspect = Command::cargo_bin("bil").unwrap();
    inspect
        .args(["bundle", "inspect", bundle_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"institutional_layer_present\": true",
        ))
        .stdout(predicate::str::contains(
            "\"cross_profile_consistency_verified\": true",
        ));

    let mut markdown = Command::cargo_bin("bil").unwrap();
    markdown
        .args([
            "bundle",
            "inspect",
            bundle_path.to_str().unwrap(),
            "--format",
            "markdown",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## Institutional Status"));
}
