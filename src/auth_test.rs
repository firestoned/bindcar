// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for auth module

use super::auth::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use tower::ServiceExt;

async fn test_handler() -> &'static str {
    "success"
}

#[tokio::test]
async fn test_authenticate_with_valid_token() {
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(authenticate));

    let request = Request::builder()
        .uri("/test")
        .header("authorization", "Bearer valid-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_authenticate_missing_header() {
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(authenticate));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_authenticate_invalid_format() {
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(authenticate));

    let request = Request::builder()
        .uri("/test")
        .header("authorization", "InvalidFormat token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_authenticate_empty_token() {
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(authenticate));

    let request = Request::builder()
        .uri("/test")
        .header("authorization", "Bearer ")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
