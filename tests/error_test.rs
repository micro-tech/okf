use axum::http::StatusCode;
use axum::response::IntoResponse;
use okf::error::OkfError;

#[test]
fn test_bundle_not_found_error() {
    let err = OkfError::BundleNotFound("missing".to_string());
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_tool_not_found_error() {
    let err = OkfError::ToolNotFound("nope".to_string());
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_duplicate_tool_id_error() {
    let err = OkfError::DuplicateToolId {
        bundle_id: "test-bundle".to_string(),
        id: "duplicate_tool".to_string(),
    };
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    // Verify message contains key info (body not directly asserted here)
}

#[test]
fn test_invalid_bundle_error() {
    let err = OkfError::InvalidBundle {
        bundle_id: "bad".to_string(),
        reason: "missing id".to_string(),
    };
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_missing_tool_file_error() {
    let err = OkfError::MissingToolFile {
        bundle_id: "helix".to_string(),
        tool_id: "fs_read".to_string(),
        file: "tools/missing.yaml".to_string(),
    };
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_invalid_tool_definition_error() {
    let err = OkfError::InvalidToolDefinition {
        bundle_id: "helix".to_string(),
        tool_id: "fs_read".to_string(),
        reason: "schema is not an object".to_string(),
    };
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let err: OkfError = io_err.into();
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_yaml_error() {
    // Create an invalid YAML error
    let yaml_err: Result<serde_yaml::Value, _> = serde_yaml::from_str("invalid: [yaml");
    let err: OkfError = yaml_err.unwrap_err().into();
    let response = err.into_response();

    // YAML parse errors from bundles now return BAD_REQUEST
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_config_error() {
    let err = OkfError::Config("invalid host".to_string());
    let response = err.into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
