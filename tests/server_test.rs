use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use okf::bundle::BundleCache;
use okf::model::ServerConfig;
use okf::server::{build_router, AppState};
use std::sync::Arc;
use tower::ServiceExt; // for `oneshot`

fn test_state() -> AppState {
    test_state_with_token(None)
}

fn test_state_with_token(token: Option<&str>) -> AppState {
    // Minimal valid config for testing
    AppState {
        config: ServerConfig {
            server: okf::model::ServerSection {
                host: "127.0.0.1".to_string(),
                port: 0,
            },
            paths: okf::model::PathsSection {
                bundles_dir: "bundles".to_string(),
                default_bundle: "helix".to_string(),
            },
            auth: okf::model::AuthSection { token: None },
        },
        cache: Arc::new(BundleCache::new()),
        auth_token: token.map(Arc::from),
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = build_router(test_state());

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_endpoint_returns_json_status_ok() {
    // GIVEN the router:
    let app = build_router(test_state());

    // WHEN we hit /health,
    // THEN the body is JSON with {"status":"ok"} (not plain text):
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn test_get_existing_bundle() {
    let app = build_router(test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/bundles/helix")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_missing_bundle() {
    let app = build_router(test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/bundles/does_not_exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_existing_tool() {
    let app = build_router(test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/bundles/helix/tools/fs_read")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_missing_tool() {
    let app = build_router(test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/bundles/helix/tools/no_such_tool")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// --- Auth matrix (fail-open reads when no token is configured) ---

#[tokio::test]
async fn test_no_token_configured_allows_anonymous() {
    // GIVEN no token configured (fail-open):
    let app = build_router(test_state_with_token(None));

    // WHEN we request without any Authorization header,
    // THEN it passes:
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_token_configured_missing_token_still_passes_on_reads() {
    // GIVEN a token is configured:
    let app = build_router(test_state_with_token(Some("s3cret")));

    // WHEN a read arrives with no token (fail-open on reads),
    // THEN it passes:
    let response = app
        .oneshot(
            Request::builder()
                .uri("/bundles/helix")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_token_configured_valid_token_passes() {
    let app = build_router(test_state_with_token(Some("s3cret")));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/bundles/helix")
                .header("Authorization", "Bearer s3cret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_token_configured_wrong_token_rejected() {
    // GIVEN a token is configured:
    let app = build_router(test_state_with_token(Some("s3cret")));

    // WHEN a request presents the wrong token,
    // THEN it is rejected even on a public read (never silently accepted):
    let response = app
        .oneshot(
            Request::builder()
                .uri("/bundles/helix")
                .header("Authorization", "Bearer wrong")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("error").is_some());
}
