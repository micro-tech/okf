use crate::bundle::resolve_bundle_dir;
use crate::error::OkfError;
use crate::model::ServerConfig;
use std::fs;
use tracing::info;

pub fn load_server_config(path: &str) -> Result<ServerConfig, OkfError> {
    if path.trim().is_empty() {
        return Err(OkfError::Config("config path is empty".to_string()));
    }

    info!(config_path = %path, "Loading server configuration");

    let contents = fs::read_to_string(path)
        .map_err(|e| OkfError::Config(format!("Failed to read config file '{path}': {e}")))?;

    let cfg: ServerConfig = serde_yaml::from_str(&contents)
        .map_err(|e| OkfError::Config(format!("Failed to parse config YAML '{path}': {e}")))?;

    // Basic validation of loaded config
    if cfg.server.host.trim().is_empty() {
        return Err(OkfError::Config("server.host is empty".to_string()));
    }
    if cfg.server.port == 0 {
        return Err(OkfError::Config("server.port must be a valid non-zero port".to_string()));
    }
    if cfg.paths.bundles_dir.trim().is_empty() {
        return Err(OkfError::Config("paths.bundles_dir is empty".to_string()));
    }

    Ok(cfg)
}

/// Fail-fast wiring for `paths.default_bundle`: it must name a bundle that
/// actually exists, otherwise the server refuses to start. (Phase 2's
/// `GET /v1/bundles?default` will resolve it at request time.)
pub fn validate_config(cfg: &ServerConfig) -> Result<(), OkfError> {
    let default = cfg.paths.default_bundle.trim();
    if default.is_empty() {
        return Err(OkfError::Config(
            "paths.default_bundle is empty".to_string(),
        ));
    }
    resolve_bundle_dir(&cfg.paths.bundles_dir, default).map_err(|e| {
        OkfError::Config(format!(
            "paths.default_bundle '{default}' is not a loadable bundle: {e}"
        ))
    })?;
    info!(default_bundle = %default, "Default bundle validated");
    Ok(())
}
