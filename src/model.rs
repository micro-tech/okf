use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub server: ServerSection,
    pub paths: PathsSection,
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
    pub schema: serde_json::Value,
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
