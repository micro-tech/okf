use crate::error::OkfError;
use crate::model::{Bundle, BundleFile, ToolFile};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn, debug, error};

pub fn load_bundle(bundles_dir: &str, bundle_id: &str) -> Result<Bundle, OkfError> {
    debug!(bundles_dir = %bundles_dir, bundle_id = %bundle_id, "Loading bundle");

    let bundle_dir = Path::new(bundles_dir).join(bundle_id);
    let meta_path = bundle_dir.join("okf.yaml");

    if !meta_path.exists() {
        warn!(bundle_id = %bundle_id, "Bundle metadata file not found");
        return Err(OkfError::BundleNotFound(bundle_id.to_string()));
    }

    let meta_str = fs::read_to_string(&meta_path)?;
    let bundle_file: BundleFile = serde_yaml::from_str(&meta_str)?;

    // Validate bundle metadata
    if bundle_file.bundle.id.trim().is_empty() {
        return Err(OkfError::InvalidBundle {
            bundle_id: bundle_id.to_string(),
            reason: "bundle id is empty or missing".to_string(),
        });
    }

    // Detect duplicate tool IDs in the bundle definition
    let mut seen_tool_ids = std::collections::HashSet::new();
    for tool_ref in &bundle_file.tools {
        if !seen_tool_ids.insert(&tool_ref.id) {
            error!(bundle_id = %bundle_id, tool_id = %tool_ref.id, "Duplicate tool ID detected in bundle");
            return Err(OkfError::DuplicateToolId {
                bundle_id: bundle_id.to_string(),
                id: tool_ref.id.clone(),
            });
        }
    }

    let mut tools = HashMap::new();

    for tool_ref in &bundle_file.tools {
        let tool_path: PathBuf = bundle_dir.join(&tool_ref.file);

        if !tool_path.exists() {
            error!(
                bundle_id = %bundle_id,
                tool_id = %tool_ref.id,
                file = %tool_ref.file,
                "Tool file missing"
            );
            return Err(OkfError::MissingToolFile {
                bundle_id: bundle_id.to_string(),
                tool_id: tool_ref.id.clone(),
                file: tool_ref.file.clone(),
            });
        }

        let tool_str = fs::read_to_string(&tool_path)?;
        let tool_file: ToolFile = serde_yaml::from_str(&tool_str)?;
        let tool_def = tool_file.tool;

        // Validate tool definition
        if tool_def.id.trim().is_empty() {
            return Err(OkfError::InvalidToolDefinition {
                bundle_id: bundle_id.to_string(),
                tool_id: tool_ref.id.clone(),
                reason: "tool id is empty or missing".to_string(),
            });
        }

        // Optional: ensure the tool id matches the reference id in the bundle manifest
        if tool_def.id != tool_ref.id {
            return Err(OkfError::InvalidToolDefinition {
                bundle_id: bundle_id.to_string(),
                tool_id: tool_ref.id.clone(),
                reason: format!(
                    "tool id '{}' in file does not match declared id '{}' in bundle manifest",
                    tool_def.id, tool_ref.id
                ),
            });
        }

        // Basic schema validation: schema should be an object
        if !tool_def.schema.is_object() && !tool_def.schema.is_null() {
            return Err(OkfError::InvalidToolDefinition {
                bundle_id: bundle_id.to_string(),
                tool_id: tool_def.id.clone(),
                reason: "schema must be a JSON object (or null)".to_string(),
            });
        }

        tools.insert(tool_def.id.clone(), tool_def);
    }

    info!(
        bundle_id = %bundle_id,
        tool_count = tools.len(),
        "Successfully loaded bundle"
    );

    Ok(Bundle {
        meta: bundle_file.bundle,
        tools,
    })
}
