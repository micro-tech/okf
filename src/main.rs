mod bundle;
mod config;
mod error;
mod model;
mod server;

use crate::config::load_server_config;
use crate::server::{build_router, AppState};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging with tracing.
    // Supports RUST_LOG env var, e.g. RUST_LOG=info,debug,okf=trace
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting OKF server...");

    let cfg = match load_server_config("config/server.yaml") {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load config: {}", e);
            return Err(anyhow::anyhow!("Failed to load config: {e}"));
        }
    };

    info!(
        host = %cfg.server.host,
        port = cfg.server.port,
        bundles_dir = %cfg.paths.bundles_dir,
        default_bundle = %cfg.paths.default_bundle,
        "Loaded server configuration"
    );

    let state = AppState { config: cfg.clone() };

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
