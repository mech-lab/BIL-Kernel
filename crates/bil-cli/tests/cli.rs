use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

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
