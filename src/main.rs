// The binary is a thin wrapper: all logic lives in the library crate so
// integration tests exercise the exact same code the server runs.
use okf::bundle::BundleCache;
use okf::config::{load_server_config, validate_config};
use okf::model::ServerConfig;
use okf::server::{build_router, AppState};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const DEFAULT_CONFIG_PATH: &str = "config/server.yaml";

/// Config path precedence: `--config`/`-c` flag > `OKF_CONFIG` env >
/// `config/server.yaml`. Hand-rolled for one flag — no clap dependency.
fn resolve_config_path() -> String {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if (arg == "--config" || arg == "-c")
            && let Some(path) = args.next()
        {
            return path;
        }
        if let Some(path) = arg.strip_prefix("--config=") {
            return path.to_string();
        }
    }
    std::env::var("OKF_CONFIG")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CONFIG_PATH.to_string())
}

/// Bearer token precedence: `OKF_AUTH_TOKEN` env > `auth.token` in the
/// config file. Blank values are treated as "not configured" (fail-open).
fn resolve_auth_token(cfg: &ServerConfig) -> Option<Arc<str>> {
    std::env::var("OKF_AUTH_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| cfg.auth.token.clone().filter(|s| !s.trim().is_empty()))
        .map(|s| Arc::from(s.trim()))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging with tracing.
    // Supports RUST_LOG env var, e.g. RUST_LOG=info,debug,okf=trace
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting OKF server...");

    let config_path = resolve_config_path();
    info!(config_path = %config_path, "Using configuration file");

    let cfg = match load_server_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load config: {}", e);
            return Err(anyhow::anyhow!("Failed to load config: {e}"));
        }
    };

    if let Err(e) = validate_config(&cfg) {
        error!("Invalid configuration: {}", e);
        return Err(anyhow::anyhow!("Invalid configuration: {e}"));
    }

    let auth_token = resolve_auth_token(&cfg);
    if auth_token.is_some() {
        info!("Bearer token auth enabled (fail-open on reads, required on writes)");
    } else {
        warn!("No bearer token configured — server is fail-open. Set OKF_AUTH_TOKEN or auth.token before exposing beyond localhost.");
    }

    info!(
        host = %cfg.server.host,
        port = cfg.server.port,
        bundles_dir = %cfg.paths.bundles_dir,
        default_bundle = %cfg.paths.default_bundle,
        auth_enabled = auth_token.is_some(),
        "Loaded server configuration"
    );

    let state = AppState {
        config: cfg.clone(),
        cache: Arc::new(BundleCache::new()),
        auth_token,
    };

    let router = build_router(state);

    let addr = SocketAddr::from((
        cfg.server.host.parse::<std::net::IpAddr>()?,
        cfg.server.port,
    ));

    info!("OKF server listening on http://{addr}");

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
