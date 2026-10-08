use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OkfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Bundle not found: {0}")]
    BundleNotFound(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Invalid bundle id '{0}': must be non-empty and contain only [A-Za-z0-9_-]")]
    InvalidBundleId(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Duplicate tool ID '{id}' in bundle '{bundle_id}'")]
    DuplicateToolId { bundle_id: String, id: String },

    #[error("Invalid bundle '{bundle_id}': {reason}")]
    InvalidBundle { bundle_id: String, reason: String },

    #[error("Missing tool file for '{tool_id}' in bundle '{bundle_id}': {file}")]
    MissingToolFile {
        bundle_id: String,
        tool_id: String,
        file: String,
    },

    #[error("Invalid tool definition '{tool_id}' in bundle '{bundle_id}': {reason}")]
    InvalidToolDefinition {
        bundle_id: String,
        tool_id: String,
        reason: String,
    },

    #[error("Config error: {0}")]
    Config(String),
}

impl IntoResponse for OkfError {
    fn into_response(self) -> Response {
        // 4xx responses may echo back the client-supplied id — that's the
        // caller's own input, not a leak. 5xx responses are deliberately
        // generic: the full error (paths, OS text) goes to the server log
        // via the tracing calls at each call site, never to the client.
        let (status, message) = match &self {
            OkfError::BundleNotFound(id) => (StatusCode::NOT_FOUND, format!("Bundle not found: {id}")),
            OkfError::ToolNotFound(id) => (StatusCode::NOT_FOUND, format!("Tool not found: {id}")),
            OkfError::InvalidBundleId(id) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid bundle id: '{id}'"),
            ),
            OkfError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            OkfError::DuplicateToolId { bundle_id, id } => (
                StatusCode::BAD_REQUEST,
                format!("Duplicate tool ID '{id}' in bundle '{bundle_id}'"),
            ),
            OkfError::InvalidBundle { bundle_id, reason } => (
                StatusCode::BAD_REQUEST,
                format!("Invalid bundle '{bundle_id}': {reason}"),
            ),
            OkfError::MissingToolFile { bundle_id, tool_id, .. } => (
                StatusCode::BAD_REQUEST,
                format!("Missing tool file for tool '{tool_id}' in bundle '{bundle_id}'"),
            ),
            OkfError::InvalidToolDefinition { bundle_id, tool_id, reason } => (
                StatusCode::BAD_REQUEST,
                format!("Invalid tool definition '{tool_id}' in bundle '{bundle_id}': {reason}"),
            ),
            // serde_yaml errors carry line/column info, not filesystem paths —
            // safe and genuinely useful for operators fixing their YAML.
            OkfError::Yaml(e) => (StatusCode::BAD_REQUEST, format!("YAML parse error: {e}")),
            OkfError::Json(e) => (StatusCode::BAD_REQUEST, format!("JSON parse error: {e}")),
            // Trimmed on purpose: no OS error text, no paths (Reviewer item 9).
            OkfError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
            OkfError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
