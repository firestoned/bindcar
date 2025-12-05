// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for middleware module

use super::middleware::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    response::IntoResponse,
    routing::get,
    Router,
};
use tower::ServiceExt;

async fn test_handler() -> impl IntoResponse {
    (StatusCode::OK, "success")
}

async fn test_handler_error() -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "error")
}

#[tokio::test]
async fn test_track_metrics_success() {
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(track_metrics));

    let request = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify metrics were recorded
    let metrics_output = crate::metrics::gather_metrics().unwrap();
    assert!(metrics_output.contains("bindcar_http_requests_total"));
}

#[tokio::test]
async fn test_track_metrics_error_response() {
    let app = Router::new()
        .route("/error", get(test_handler_error))
        .layer(middleware::from_fn(track_metrics));

    let request = Request::builder()
        .uri("/error")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // Verify metrics were recorded
    let metrics_output = crate::metrics::gather_metrics().unwrap();
    assert!(metrics_output.contains("bindcar_http_requests_total"));
}

#[tokio::test]
async fn test_track_metrics_different_paths() {
    let app = Router::new()
        .route("/path1", get(test_handler))
        .route("/path2", get(test_handler))
        .layer(middleware::from_fn(track_metrics));

    let request1 = Request::builder()
        .uri("/path1")
        .body(Body::empty())
        .unwrap();

    let request2 = Request::builder()
        .uri("/path2")
        .body(Body::empty())
        .unwrap();

    let _ = app.clone().oneshot(request1).await.unwrap();
    let _ = app.oneshot(request2).await.unwrap();

    let metrics_output = crate::metrics::gather_metrics().unwrap();
    assert!(metrics_output.contains("bindcar_http_requests_total"));
}

#[tokio::test]
async fn test_track_metrics_duration() {
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(track_metrics));

    let request = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let _ = app.oneshot(request).await.unwrap();

    let metrics_output = crate::metrics::gather_metrics().unwrap();
    assert!(metrics_output.contains("bindcar_http_request_duration_seconds"));
}
