use axum::{
    extract::{Path, State},
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::bundle::BundleCache;
use crate::error::OkfError;
use crate::model::{Bundle, ServerConfig};
use tower_http::trace::TraceLayer;
use tracing::{info, debug, warn};

#[derive(Clone)]
pub struct AppState {
    pub config: ServerConfig,
    pub cache: Arc<BundleCache>,
    /// Resolved bearer token (config file, `OKF_AUTH_TOKEN` wins). None =
    /// fail-open: every request passes, matching the Helix posture.
    pub auth_token: Option<Arc<str>>,
}

/// Per-route auth requirement. Today every route is a read (`required: false`);
/// Phase 2 write endpoints will be constructed with `required: true`.
#[derive(Clone)]
struct AuthConfig {
    token: Option<Arc<str>>,
    required: bool,
}

/// Compare in constant time so token bytes don't leak through timing.
/// (Hand-rolled to avoid a new dependency for ten lines.)
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn bearer_token(req: &Request<axum::body::Body>) -> Option<&str> {
    req.headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

async fn auth_middleware(
    State(auth): State<Arc<AuthConfig>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // No token configured → fail-open, everything passes.
    let Some(expected) = auth.token.as_deref() else {
        return next.run(req).await;
    };

    match bearer_token(&req) {
        Some(presented) if constant_time_eq(presented.as_bytes(), expected.as_bytes()) => {
            next.run(req).await
        }
        Some(_) => {
            // A wrong token is never silently accepted — fail closed here
            // even on public reads, so a misconfigured client can't think
            // it's authenticated when it isn't.
            warn!("Rejected request with invalid bearer token");
            OkfError::Unauthorized.into_response()
        }
        None if auth.required => OkfError::Unauthorized.into_response(),
        // Token configured but this is a public read and the client didn't
        // present one: fail-open, per the unification plan.
        None => next.run(req).await,
    }
}

pub fn build_router(state: AppState) -> Router {
    let auth_state = Arc::new(AuthConfig {
        token: state.auth_token.clone(),
        // All current routes are reads: public, fail-open.
        // Phase 2 write endpoints will require the token when configured.
        required: false,
    });
    Router::new()
        .route("/health", get(health))
        .route("/bundles/{bundle_id}", get(get_bundle))
        .route("/bundles/{bundle_id}/tools/{tool_id}", get(get_tool))
        .layer(middleware::from_fn_with_state(auth_state, auth_middleware))
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

async fn health() -> impl IntoResponse {
    debug!("Health check requested");
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "okf-server",
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

async fn get_bundle(
    Path(bundle_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!(bundle_id = %bundle_id, "GET /bundles/{}", bundle_id);

    match state.cache.get(&state.config.paths.bundles_dir, &bundle_id) {
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

    match state.cache.get(&state.config.paths.bundles_dir, &bundle_id) {
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
