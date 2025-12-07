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

#[tokio::test]
async fn test_authenticate_token_with_special_characters() {
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(authenticate));

    // JWT-like token with dots and hyphens
    let request = Request::builder()
        .uri("/test")
        .header("authorization", "Bearer eyJhbGciOiJSUzI1NiIsImtpZCI6Ik1qSTBNVEV5TXpRMU5qYzRPVEF4TWpNME5UWTNPRGt3TVRJek5EVTJOVGM0T1RBeCJ9.test")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should pass basic validation (token presence check)
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_authenticate_case_sensitive_bearer() {
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(authenticate));

    // Test lowercase "bearer" - should fail
    let request = Request::builder()
        .uri("/test")
        .header("authorization", "bearer token123")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_error_serialization() {
    let error = AuthError {
        error: "Test error message".to_string(),
    };

    let json = serde_json::to_string(&error).unwrap();
    assert!(json.contains("Test error message"));
    assert!(json.contains("error"));
}

// Kubernetes TokenReview tests (only when feature is enabled)
#[cfg(feature = "k8s-token-review")]
mod k8s_token_review_tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_token_with_k8s_requires_cluster() {
        // This test validates the error handling when not in a cluster
        let result = validate_token_with_k8s("test-token").await;

        // Outside a cluster, this should fail with a clear error message
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error.contains("Failed to create Kubernetes client")
                || error.contains("Failed to validate token")
        );
    }

    #[tokio::test]
    async fn test_validate_token_empty_string() {
        let result = validate_token_with_k8s("").await;

        // Even an empty token should attempt validation and fail
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_malformed() {
        // Test with a clearly malformed token
        let result = validate_token_with_k8s("not-a-valid-jwt-token").await;

        // Should fail validation
        assert!(result.is_err());
    }
}

// Documentation tests - these appear in rustdoc
#[doc = r#"
# Authentication Examples

## Basic Token Validation (Default)

```rust,no_run
use axum::{Router, routing::get, middleware};
use bindcar::auth::authenticate;

async fn handler() -> &'static str {
    "Protected resource"
}

let app = Router::new()
    .route("/api/protected", get(handler))
    .layer(middleware::from_fn(authenticate));
```

## With Kubernetes TokenReview (feature: k8s-token-review)

When the `k8s-token-review` feature is enabled, tokens are validated
against the Kubernetes API server:

```rust,ignore
// Cargo.toml:
// bindcar = { version = "0.1", features = ["k8s-token-review"] }

// Tokens will be validated using Kubernetes TokenReview API
// Requires in-cluster configuration or valid kubeconfig
```

## Testing with Mock Tokens

```rust,no_run
use axum::{body::Body, http::Request};

let request = Request::builder()
    .uri("/api/test")
    .header("authorization", "Bearer test-token")
    .body(Body::empty())
    .unwrap();
```
"#]
#[allow(dead_code)]
fn auth_documentation_examples() {}
