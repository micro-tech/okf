use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use crate::bundle::load_bundle;
use crate::error::OkfError;
use crate::model::{Bundle, ServerConfig};
use tower_http::trace::TraceLayer;
use tracing::{info, debug, warn};

#[derive(Clone)]
pub struct AppState {
    pub config: ServerConfig,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/bundles/{bundle_id}", get(get_bundle))
        .route("/bundles/{bundle_id}/tools/{tool_id}", get(get_tool))
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &axum::http::Request<_>, _span: &tracing::Span| {
                    debug!(
                        method = %request.method(),
                        uri = %request.uri(),
                        "incoming request"
                    );
                })
                .on_response(
                    |response: &axum::http::Response<_>, latency: std::time::Duration, _span: &tracing::Span| {
                        debug!(
                            status = response.status().as_u16(),
                            latency_ms = latency.as_millis() as u64,
                            "response sent"
                        );
                    },
                ),
        )
        .with_state(state)
}

async fn health() -> &'static str {
    debug!("Health check requested");
    "okf-server: healthy"
}

async fn get_bundle(
    Path(bundle_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!(bundle_id = %bundle_id, "GET /bundles/{}", bundle_id);

    match load_bundle(&state.config.paths.bundles_dir, &bundle_id) {
        Ok(bundle) => {
            debug!(bundle_id = %bundle_id, tool_count = bundle.tools.len(), "Bundle served successfully");
            Json(bundle).into_response()
        }
        Err(e) => {
            warn!(bundle_id = %bundle_id, error = %e, "Failed to load bundle");
            e.into_response()
        }
    }
}

async fn get_tool(
    Path((bundle_id, tool_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!(bundle_id = %bundle_id, tool_id = %tool_id, "GET /bundles/{}/tools/{}", bundle_id, tool_id);

    match load_bundle(&state.config.paths.bundles_dir, &bundle_id) {
        Ok(Bundle { tools, .. }) => match tools.get(&tool_id) {
            Some(tool) => {
                debug!(bundle_id = %bundle_id, tool_id = %tool_id, "Tool served successfully");
                Json(tool.clone()).into_response()
            }
            None => {
                warn!(bundle_id = %bundle_id, tool_id = %tool_id, "Tool not found");
                OkfError::ToolNotFound(tool_id).into_response()
            }
        },
        Err(e) => {
            warn!(bundle_id = %bundle_id, error = %e, "Failed to load bundle for tool request");
            e.into_response()
        }
    }
}
