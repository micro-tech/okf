use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use okf::model::ServerConfig;
use okf::server::{build_router, AppState};
use tower::ServiceExt; // for `oneshot`

fn test_state() -> AppState {
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
        },
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
