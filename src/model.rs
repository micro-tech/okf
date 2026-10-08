use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub server: ServerSection,
    pub paths: PathsSection,
    /// Auth is optional and defaults to absent: without a token the server
    /// is fail-open (John's established Helix posture). Present-but-empty
    /// is treated the same as absent.
    #[serde(default)]
    pub auth: AuthSection,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct AuthSection {
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerSection {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PathsSection {
    pub bundles_dir: String,
    pub default_bundle: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BundleMeta {
    pub id: String,
    pub description: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BundleToolRef {
    pub id: String,
    pub file: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolDef {
    pub id: String,
    pub description: Option<String>,
    /// Optional: a tool without a declared schema is valid (schema-less tool).
    /// When present it must be a JSON object — enforced at load time.
    #[serde(default)]
    pub schema: Option<serde_json::Value>,
}

/// Wrapper for okf.yaml files which have a top-level `bundle:` key
/// and a sibling `tools:` list at the root.
#[derive(Debug, Deserialize)]
pub(crate) struct BundleFile {
    pub bundle: BundleMeta,
    pub tools: Vec<BundleToolRef>,
}

/// Wrapper for tool YAML files which have a top-level `tool:` key.
#[derive(Debug, Deserialize)]
pub(crate) struct ToolFile {
    pub tool: ToolDef,
}

#[derive(Debug, Clone, Serialize)]
pub struct Bundle {
    pub meta: BundleMeta,
    pub tools: HashMap<String, ToolDef>,
}
