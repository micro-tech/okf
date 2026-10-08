use axum::http::StatusCode;
use axum::response::IntoResponse;
use okf::error::OkfError;

async fn body_json(response: axum::response::Response) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("error body must be JSON");
    (status, json)
}

#[tokio::test]
async fn test_io_error_is_trimmed_500_json() {
    // GIVEN an IO error carrying a filesystem path and OS text:
    let io_err = std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "permission denied on /opt/okf/bundles/secret/okf.yaml",
    );
    let err: OkfError = io_err.into();

    // WHEN converted to a response,
    // THEN it is a 500 with a generic JSON body — no paths, no OS text:
    let (status, json) = body_json(err.into_response()).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let msg = json["error"].as_str().unwrap();
    assert!(!msg.contains("/opt/okf"), "path leaked into 500: {msg}");
    assert!(!msg.contains("permission denied"), "OS text leaked into 500: {msg}");
}

#[tokio::test]
async fn test_config_error_is_trimmed_500_json() {
    let err = OkfError::Config("failed to parse /etc/okf/server.yaml: boom".to_string());
    let (status, json) = body_json(err.into_response()).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let msg = json["error"].as_str().unwrap();
    assert!(!msg.contains("/etc/okf"), "path leaked into 500: {msg}");
}

#[tokio::test]
async fn test_4xx_errors_are_json_with_error_field() {
    // 4xx bodies echo the caller-supplied id (their own input) as JSON.
    for err in [
        OkfError::BundleNotFound("helix".to_string()),
        OkfError::ToolNotFound("fs_read".to_string()),
        OkfError::InvalidBundleId("../x".to_string()),
        OkfError::Unauthorized,
    ] {
        let (status, json) = body_json(err.into_response()).await;
        assert!(status.is_client_error(), "expected 4xx, got {status}");
        assert!(json.get("error").is_some(), "missing error field: {json}");
    }
}

#[tokio::test]
async fn test_invalid_bundle_id_maps_to_400() {
    let (status, _) = body_json(OkfError::InvalidBundleId("..".to_string()).into_response()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_unauthorized_maps_to_401() {
    let (status, _) = body_json(OkfError::Unauthorized.into_response()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
