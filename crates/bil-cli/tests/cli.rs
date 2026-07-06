use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::{NamedTempFile, TempDir};

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
        .stdout(predicate::str::contains("\"verified\": true"))
        .stdout(predicate::str::contains("\"payload_count\": 1"));
}
