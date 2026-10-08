use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
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
    #[allow(dead_code)]
    Config(String),
}

impl IntoResponse for OkfError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            OkfError::BundleNotFound(id) => (StatusCode::NOT_FOUND, format!("Bundle not found: {id}")),
            OkfError::ToolNotFound(id) => (StatusCode::NOT_FOUND, format!("Tool not found: {id}")),
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
            OkfError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("IO error: {e}")),
            OkfError::Yaml(e) => (StatusCode::BAD_REQUEST, format!("YAML parse error: {e}")),
            OkfError::Json(e) => (StatusCode::BAD_REQUEST, format!("JSON parse error: {e}")),
            OkfError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Config error: {msg}")),
        };
        (status, message).into_response()
    }
}
