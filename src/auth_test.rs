// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for auth module

use crate::auth::{authenticate, AuthError};
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
#[cfg_attr(feature = "k8s-token-review", serial_test::serial)]
async fn test_authenticate_with_valid_token() {
    #[cfg(feature = "k8s-token-review")]
    {
        // When k8s-token-review is enabled, actual token validation happens
        // This test would need a real Kubernetes cluster, so we skip detailed testing
        // The feature-specific tests handle validation logic
        use std::env;
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(middleware::from_fn(authenticate));

    let request = Request::builder()
        .uri("/test")
        .header("authorization", "Bearer valid-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    #[cfg(not(feature = "k8s-token-review"))]
    assert_eq!(response.status(), StatusCode::OK);

    #[cfg(feature = "k8s-token-review")]
    {
        // With k8s-token-review, this will fail without a real cluster
        // which is expected behavior
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        use std::env;
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }
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
#[cfg_attr(feature = "k8s-token-review", serial_test::serial)]
async fn test_authenticate_token_with_special_characters() {
    #[cfg(feature = "k8s-token-review")]
    {
        use std::env;
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

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

    #[cfg(not(feature = "k8s-token-review"))]
    assert_eq!(response.status(), StatusCode::OK);

    #[cfg(feature = "k8s-token-review")]
    {
        // With k8s-token-review, validation will fail without a real cluster
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        use std::env;
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }
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
    use crate::auth::{validate_token_with_k8s, TokenReviewConfig};
    use serial_test::serial;
    use std::env;

    #[tokio::test]
    #[serial]
    async fn test_validate_token_with_k8s_requires_cluster() {
        // Clear environment to ensure consistent test
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");

        // This test validates the error handling when not in a cluster
        let result = validate_token_with_k8s("test-token").await;

        // Outside a cluster, this should fail with a clear error message
        assert!(
            result.is_err(),
            "Expected token validation to fail outside cluster"
        );

        // We just need to confirm it failed - the error message can vary

        // Cleanup
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
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

    #[test]
    #[serial_test::serial]
    fn test_token_review_config_default_audiences() {
        // Clear environment
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");

        let config = TokenReviewConfig::from_env();

        // Default audience should be "bindcar"
        assert_eq!(config.audiences, vec!["bindcar"]);
        assert!(config.allowed_namespaces.is_empty());
        assert!(config.allowed_service_accounts.is_empty());

        // Cleanup
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

    #[test]
    #[serial_test::serial]
    fn test_token_review_config_custom_audiences() {
        env::set_var("BIND_TOKEN_AUDIENCES", "api1,api2,api3");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");

        let config = TokenReviewConfig::from_env();

        assert_eq!(config.audiences, vec!["api1", "api2", "api3"]);
        assert!(config.allowed_namespaces.is_empty());
        assert!(config.allowed_service_accounts.is_empty());

        // Cleanup
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

    #[test]
    #[serial]
    fn test_token_review_config_with_whitespace() {
        env::set_var("BIND_TOKEN_AUDIENCES", " api1 , api2 , api3 ");
        env::set_var("BIND_ALLOWED_NAMESPACES", " dns-system , kube-system ");
        env::set_var(
            "BIND_ALLOWED_SERVICE_ACCOUNTS",
            " system:serviceaccount:ns1:sa1 , system:serviceaccount:ns2:sa2 ",
        );

        let config = TokenReviewConfig::from_env();

        // Whitespace should be trimmed
        assert_eq!(config.audiences, vec!["api1", "api2", "api3"]);
        assert_eq!(config.allowed_namespaces, vec!["dns-system", "kube-system"]);
        assert_eq!(
            config.allowed_service_accounts,
            vec![
                "system:serviceaccount:ns1:sa1",
                "system:serviceaccount:ns2:sa2"
            ]
        );

        // Cleanup
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

    #[test]
    #[serial]
    fn test_token_review_config_empty_values() {
        env::set_var("BIND_TOKEN_AUDIENCES", "");
        env::set_var("BIND_ALLOWED_NAMESPACES", "");
        env::set_var("BIND_ALLOWED_SERVICE_ACCOUNTS", "");

        let config = TokenReviewConfig::from_env();

        // Empty strings should result in default audience
        assert_eq!(config.audiences, vec!["bindcar"]);
        assert!(config.allowed_namespaces.is_empty());
        assert!(config.allowed_service_accounts.is_empty());

        // Cleanup
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

    #[test]
    #[serial]
    fn test_is_namespace_allowed_empty_list() {
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");

        let config = TokenReviewConfig::from_env();

        // Empty list means allow all
        assert!(config.is_namespace_allowed("any-namespace"));
        assert!(config.is_namespace_allowed("dns-system"));
        assert!(config.is_namespace_allowed("default"));

        // Cleanup
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

    #[test]
    #[serial]
    fn test_is_namespace_allowed_with_allowlist() {
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::set_var("BIND_ALLOWED_NAMESPACES", "dns-system,kube-system");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");

        let config = TokenReviewConfig::from_env();

        assert!(config.is_namespace_allowed("dns-system"));
        assert!(config.is_namespace_allowed("kube-system"));
        assert!(!config.is_namespace_allowed("default"));
        assert!(!config.is_namespace_allowed("other-namespace"));

        // Cleanup
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

    #[test]
    #[serial]
    fn test_is_service_account_allowed_empty_list() {
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");

        let config = TokenReviewConfig::from_env();

        // Empty list means allow all
        assert!(config.is_service_account_allowed("system:serviceaccount:ns:sa"));
        assert!(config.is_service_account_allowed("system:serviceaccount:default:test"));

        // Cleanup
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

    #[test]
    #[serial]
    fn test_is_service_account_allowed_with_allowlist() {
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::set_var(
            "BIND_ALLOWED_SERVICE_ACCOUNTS",
            "system:serviceaccount:dns-system:external-dns,system:serviceaccount:dns-system:cert-manager",
        );

        let config = TokenReviewConfig::from_env();

        assert!(config.is_service_account_allowed("system:serviceaccount:dns-system:external-dns"));
        assert!(config.is_service_account_allowed("system:serviceaccount:dns-system:cert-manager"));
        assert!(!config.is_service_account_allowed("system:serviceaccount:dns-system:other-app"));
        assert!(!config.is_service_account_allowed("system:serviceaccount:default:my-app"));

        // Cleanup
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

    #[test]
    fn test_extract_namespace_valid() {
        let username = "system:serviceaccount:dns-system:external-dns";
        let namespace = TokenReviewConfig::extract_namespace(username);

        assert_eq!(namespace, Some("dns-system".to_string()));
    }

    #[test]
    fn test_extract_namespace_different_namespace() {
        let username = "system:serviceaccount:kube-system:coredns";
        let namespace = TokenReviewConfig::extract_namespace(username);

        assert_eq!(namespace, Some("kube-system".to_string()));
    }

    #[test]
    fn test_extract_namespace_invalid_format() {
        // Not enough parts
        assert_eq!(TokenReviewConfig::extract_namespace("invalid"), None);

        // Wrong prefix
        assert_eq!(
            TokenReviewConfig::extract_namespace("user:serviceaccount:ns:sa"),
            None
        );

        // Missing serviceaccount
        assert_eq!(
            TokenReviewConfig::extract_namespace("system:namespace:ns:sa"),
            None
        );

        // Too few colons
        assert_eq!(
            TokenReviewConfig::extract_namespace("system:serviceaccount:ns"),
            None
        );
    }

    #[test]
    fn test_extract_namespace_with_special_characters() {
        let username = "system:serviceaccount:my-dns-system-123:app-name_v2";
        let namespace = TokenReviewConfig::extract_namespace(username);

        assert_eq!(namespace, Some("my-dns-system-123".to_string()));
    }

    #[test]
    #[serial]
    fn test_config_combination_strict_production() {
        env::set_var("BIND_TOKEN_AUDIENCES", "bindcar,https://bindcar.svc");
        env::set_var("BIND_ALLOWED_NAMESPACES", "dns-system");
        env::set_var(
            "BIND_ALLOWED_SERVICE_ACCOUNTS",
            "system:serviceaccount:dns-system:external-dns",
        );

        let config = TokenReviewConfig::from_env();

        assert_eq!(config.audiences, vec!["bindcar", "https://bindcar.svc"]);
        assert_eq!(config.allowed_namespaces, vec!["dns-system"]);
        assert_eq!(
            config.allowed_service_accounts,
            vec!["system:serviceaccount:dns-system:external-dns"]
        );

        // Validate combinations
        assert!(config.is_namespace_allowed("dns-system"));
        assert!(!config.is_namespace_allowed("default"));
        assert!(config.is_service_account_allowed("system:serviceaccount:dns-system:external-dns"));
        assert!(!config.is_service_account_allowed("system:serviceaccount:dns-system:other"));

        // Cleanup
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }
}
