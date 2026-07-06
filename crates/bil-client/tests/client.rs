use bil_client::{AxleClient, AxleError};
use mockito::Server;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn check_status_returns_health_payload() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/v1/status")
        .match_header("x-request-source", "sdk")
        .with_status(200)
        .with_body(r#"{"status":"healthy"}"#)
        .create_async()
        .await;

    let client = AxleClient::new(Some(server.url()), Some(4), Some(5.0), None).unwrap();
    let status = client.check_status(5.0).await.unwrap();

    assert_eq!(status["status"], "healthy");
    mock.assert_async().await;
}

#[tokio::test]
async fn check_status_maps_google_oidc_redirect_to_browser_login_error() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/v1/status")
        .with_status(302)
        .with_header("location", "https://accounts.google.com/o/oauth2/v2/auth")
        .with_body("redirect")
        .create_async()
        .await;

    let client = AxleClient::new(Some(server.url()), None, Some(5.0), None).unwrap();
    let error = client.check_status(5.0).await.unwrap_err();

    assert!(matches!(error, AxleError::BrowserLoginRequired { .. }));
    mock.assert_async().await;
}

#[tokio::test]
async fn check_status_times_out_when_server_hangs() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buffer = vec![0_u8; 1024];
            let _ = stream.read(&mut buffer).await;
            sleep(Duration::from_millis(200)).await;
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\n{\"status\":\"healthy\"}")
                .await;
        }
    });

    let client =
        AxleClient::new(Some(format!("http://{address}")), None, Some(0.05), None).unwrap();
    let error = client.check_status(0.0).await.unwrap_err();

    assert!(matches!(error, AxleError::IsUnavailable { .. }));
    assert!(error.to_string().contains("client timeout"));
}

#[tokio::test]
async fn check_maps_not_found_errors() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/api/v1/check")
        .with_status(404)
        .with_body("missing")
        .create_async()
        .await;

    let client = AxleClient::new(Some(server.url()), None, Some(5.0), None).unwrap();
    let error = client
        .check(
            "theorem foo : 1 = 1 := rfl".to_string(),
            "lean-4.28.0".to_string(),
            None,
            None,
            None,
        )
        .await
        .unwrap_err();

    assert!(matches!(error, AxleError::NotFound(_)));
    mock.assert_async().await;
}

#[tokio::test]
async fn check_deserializes_typed_responses() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/api/v1/check")
        .match_body(mockito::Matcher::PartialJson(json!({
            "content": "theorem foo : 1 = 1 := rfl",
            "environment": "lean-4.28.0"
        })))
        .with_status(200)
        .with_body(
            r#"{"okay":true,"content":"theorem foo : 1 = 1 := rfl","lean_messages":{"errors":[],"warnings":[],"infos":[]},"tool_messages":{"errors":[],"warnings":[],"infos":[]},"failed_declarations":[],"timings":{"total":10},"info":null}"#,
        )
        .create_async()
        .await;

    let client = AxleClient::new(Some(server.url()), None, Some(5.0), None).unwrap();
    let response = client
        .check(
            "theorem foo : 1 = 1 := rfl".to_string(),
            "lean-4.28.0".to_string(),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert!(response.okay);
    assert_eq!(response.timings["total"], 10);
    mock.assert_async().await;
}
