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
