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

// Kubernetes client builder tests (only when feature is enabled)
#[cfg(feature = "k8s-token-review")]
mod kube_client_builder_tests {
    use crate::auth::{build_explicit_kube_client, detect_kube_auth_mode, KubeAuthMode};
    use serial_test::serial;
    use std::env;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // --- detect_kube_auth_mode: explicit mode ---

    #[test]
    #[serial]
    fn test_detect_kube_auth_mode_explicit_when_all_vars_set() {
        env::set_var("KUBE_API_SERVER", "https://api.example.com:6443");
        env::set_var("KUBE_TOKEN_PATH", "/var/run/secrets/token");
        env::set_var("KUBE_CA_CERT_PATH", "/var/run/secrets/ca.crt");

        let mode = detect_kube_auth_mode();

        match mode {
            KubeAuthMode::Explicit {
                server,
                token_path,
                ca_cert_path,
            } => {
                assert_eq!(server, "https://api.example.com:6443");
                assert_eq!(token_path, "/var/run/secrets/token");
                assert_eq!(ca_cert_path, "/var/run/secrets/ca.crt");
            }
            KubeAuthMode::Default => panic!("Expected Explicit mode, got Default"),
        }

        env::remove_var("KUBE_API_SERVER");
        env::remove_var("KUBE_TOKEN_PATH");
        env::remove_var("KUBE_CA_CERT_PATH");
    }

    // --- detect_kube_auth_mode: fallback to Default when no vars set ---

    #[test]
    #[serial]
    fn test_detect_kube_auth_mode_default_when_no_vars_set() {
        env::remove_var("KUBE_API_SERVER");
        env::remove_var("KUBE_TOKEN_PATH");
        env::remove_var("KUBE_CA_CERT_PATH");

        let mode = detect_kube_auth_mode();

        assert!(
            matches!(mode, KubeAuthMode::Default),
            "Expected Default mode when no env vars are set"
        );

        env::remove_var("KUBE_API_SERVER");
        env::remove_var("KUBE_TOKEN_PATH");
        env::remove_var("KUBE_CA_CERT_PATH");
    }

    // --- detect_kube_auth_mode: partial vars always fall back to Default ---

    #[test]
    #[serial]
    fn test_detect_kube_auth_mode_default_when_only_api_server_set() {
        env::set_var("KUBE_API_SERVER", "https://api.example.com:6443");
        env::remove_var("KUBE_TOKEN_PATH");
        env::remove_var("KUBE_CA_CERT_PATH");

        let mode = detect_kube_auth_mode();

        assert!(
            matches!(mode, KubeAuthMode::Default),
            "Expected Default mode when only KUBE_API_SERVER is set"
        );

        env::remove_var("KUBE_API_SERVER");
        env::remove_var("KUBE_TOKEN_PATH");
        env::remove_var("KUBE_CA_CERT_PATH");
    }

    #[test]
    #[serial]
    fn test_detect_kube_auth_mode_default_when_only_token_path_set() {
        env::remove_var("KUBE_API_SERVER");
        env::set_var("KUBE_TOKEN_PATH", "/var/run/secrets/token");
        env::remove_var("KUBE_CA_CERT_PATH");

        let mode = detect_kube_auth_mode();

        assert!(
            matches!(mode, KubeAuthMode::Default),
            "Expected Default mode when only KUBE_TOKEN_PATH is set"
        );

        env::remove_var("KUBE_API_SERVER");
        env::remove_var("KUBE_TOKEN_PATH");
        env::remove_var("KUBE_CA_CERT_PATH");
    }

    #[test]
    #[serial]
    fn test_detect_kube_auth_mode_default_when_only_ca_cert_path_set() {
        env::remove_var("KUBE_API_SERVER");
        env::remove_var("KUBE_TOKEN_PATH");
        env::set_var("KUBE_CA_CERT_PATH", "/var/run/secrets/ca.crt");

        let mode = detect_kube_auth_mode();

        assert!(
            matches!(mode, KubeAuthMode::Default),
            "Expected Default mode when only KUBE_CA_CERT_PATH is set"
        );

        env::remove_var("KUBE_API_SERVER");
        env::remove_var("KUBE_TOKEN_PATH");
        env::remove_var("KUBE_CA_CERT_PATH");
    }

    #[test]
    #[serial]
    fn test_detect_kube_auth_mode_default_when_api_server_and_token_set_missing_ca() {
        env::set_var("KUBE_API_SERVER", "https://api.example.com:6443");
        env::set_var("KUBE_TOKEN_PATH", "/var/run/secrets/token");
        env::remove_var("KUBE_CA_CERT_PATH");

        let mode = detect_kube_auth_mode();

        assert!(
            matches!(mode, KubeAuthMode::Default),
            "Expected Default mode when KUBE_CA_CERT_PATH is missing"
        );

        env::remove_var("KUBE_API_SERVER");
        env::remove_var("KUBE_TOKEN_PATH");
        env::remove_var("KUBE_CA_CERT_PATH");
    }

    #[test]
    #[serial]
    fn test_detect_kube_auth_mode_default_when_api_server_and_ca_set_missing_token() {
        env::set_var("KUBE_API_SERVER", "https://api.example.com:6443");
        env::remove_var("KUBE_TOKEN_PATH");
        env::set_var("KUBE_CA_CERT_PATH", "/var/run/secrets/ca.crt");

        let mode = detect_kube_auth_mode();

        assert!(
            matches!(mode, KubeAuthMode::Default),
            "Expected Default mode when KUBE_TOKEN_PATH is missing"
        );

        env::remove_var("KUBE_API_SERVER");
        env::remove_var("KUBE_TOKEN_PATH");
        env::remove_var("KUBE_CA_CERT_PATH");
    }

    #[test]
    #[serial]
    fn test_detect_kube_auth_mode_default_when_token_and_ca_set_missing_api_server() {
        env::remove_var("KUBE_API_SERVER");
        env::set_var("KUBE_TOKEN_PATH", "/var/run/secrets/token");
        env::set_var("KUBE_CA_CERT_PATH", "/var/run/secrets/ca.crt");

        let mode = detect_kube_auth_mode();

        assert!(
            matches!(mode, KubeAuthMode::Default),
            "Expected Default mode when KUBE_API_SERVER is missing"
        );

        env::remove_var("KUBE_API_SERVER");
        env::remove_var("KUBE_TOKEN_PATH");
        env::remove_var("KUBE_CA_CERT_PATH");
    }

    // --- build_explicit_kube_client: file error handling ---

    #[tokio::test]
    async fn test_build_explicit_kube_client_fails_with_missing_token_file() {
        let result = build_explicit_kube_client(
            "https://api.example.com:6443".to_string(),
            "/nonexistent/bindcar-test/token".to_string(),
            "/nonexistent/bindcar-test/ca.crt".to_string(),
        )
        .await;

        assert!(result.is_err(), "Expected error for missing token file");
        // Client doesn't implement Debug, so extract the error string via if let.
        if let Err(err) = result {
            assert!(
                err.contains("token")
                    || err.contains("No such file")
                    || err.contains("failed to read"),
                "Error should describe the token file problem, got: {}",
                err
            );
        }
    }

    #[tokio::test]
    async fn test_build_explicit_kube_client_fails_with_missing_ca_file() {
        let mut token_file = NamedTempFile::new().unwrap();
        write!(token_file, "fake-sa-token").unwrap();
        let token_path = token_file.path().to_str().unwrap().to_string();

        let result = build_explicit_kube_client(
            "https://api.example.com:6443".to_string(),
            token_path,
            "/nonexistent/bindcar-test/ca.crt".to_string(),
        )
        .await;

        assert!(result.is_err(), "Expected error for missing CA cert file");
        if let Err(err) = result {
            assert!(
                err.contains("certificate")
                    || err.contains("ca")
                    || err.contains("No such file")
                    || err.contains("failed to read"),
                "Error should describe the CA certificate problem, got: {}",
                err
            );
        }
    }

    #[tokio::test]
    async fn test_build_explicit_kube_client_fails_with_invalid_ca_cert() {
        let mut token_file = NamedTempFile::new().unwrap();
        write!(token_file, "fake-sa-token").unwrap();
        let token_path = token_file.path().to_str().unwrap().to_string();

        let mut ca_file = NamedTempFile::new().unwrap();
        write!(ca_file, "this is not a valid PEM certificate").unwrap();
        let ca_path = ca_file.path().to_str().unwrap().to_string();

        let result = build_explicit_kube_client(
            "https://api.example.com:6443".to_string(),
            token_path,
            ca_path,
        )
        .await;

        assert!(
            result.is_err(),
            "Expected error for invalid CA certificate content"
        );
    }
}

// Kubernetes TokenReview tests (only when feature is enabled)
#[cfg(feature = "k8s-token-review")]
mod k8s_token_review_tests {
    use crate::auth::{
        audiences_compatible, check_authorization_posture, validate_token_with_k8s,
        TokenReviewConfig,
    };
    use serial_test::serial;
    use std::env;

    // ---- A1: audience binding (status.audiences) ----

    #[test]
    fn test_audiences_compatible_exact_match() {
        let requested = vec!["bindcar".to_string()];
        let returned = vec!["bindcar".to_string()];
        assert!(audiences_compatible(&requested, &returned));
    }

    #[test]
    fn test_audiences_compatible_intersection() {
        let requested = vec!["bindcar".to_string(), "https://bindcar.svc".to_string()];
        let returned = vec!["https://bindcar.svc".to_string()];
        assert!(audiences_compatible(&requested, &returned));
    }

    #[test]
    fn test_audiences_incompatible_empty_returned() {
        // The A1 exploit case: apiserver authenticates a valid token but echoes
        // no compatible audience. Must be rejected, not accepted.
        let requested = vec!["bindcar".to_string()];
        let returned: Vec<String> = vec![];
        assert!(!audiences_compatible(&requested, &returned));
    }

    #[test]
    fn test_audiences_incompatible_apiserver_audience() {
        // A pod's default SA token carries the apiserver audience, never "bindcar".
        let requested = vec!["bindcar".to_string()];
        let returned = vec!["https://kubernetes.default.svc".to_string()];
        assert!(!audiences_compatible(&requested, &returned));
    }

    // ---- A2: fail-closed authorization posture ----

    #[test]
    fn test_authorization_posture_rejects_allow_all_default() {
        // No allowlist + no override = allow-all: must refuse to start.
        assert!(check_authorization_posture(false, false).is_err());
    }

    #[test]
    fn test_authorization_posture_allows_restricted() {
        assert!(check_authorization_posture(true, false).is_ok());
    }

    #[test]
    fn test_authorization_posture_allows_explicit_override() {
        assert!(check_authorization_posture(false, true).is_ok());
    }

    #[test]
    #[serial]
    fn test_is_authorization_restricted_empty_is_unrestricted() {
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");

        let config = TokenReviewConfig::from_env();
        assert!(!config.is_authorization_restricted());
    }

    #[test]
    #[serial]
    fn test_is_authorization_restricted_with_namespace() {
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::set_var("BIND_ALLOWED_NAMESPACES", "dns-system");
        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");

        let config = TokenReviewConfig::from_env();
        assert!(config.is_authorization_restricted());

        env::remove_var("BIND_ALLOWED_NAMESPACES");
    }

    #[test]
    #[serial]
    fn test_is_authorization_restricted_with_service_account() {
        env::remove_var("BIND_TOKEN_AUDIENCES");
        env::remove_var("BIND_ALLOWED_NAMESPACES");
        env::set_var(
            "BIND_ALLOWED_SERVICE_ACCOUNTS",
            "system:serviceaccount:dns-system:external-dns",
        );

        let config = TokenReviewConfig::from_env();
        assert!(config.is_authorization_restricted());

        env::remove_var("BIND_ALLOWED_SERVICE_ACCOUNTS");
    }

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

// ---------------------------------------------------------------------------
// B-4: shared-secret auth + startup posture guard (pure, env-free tests)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod b4_auth_posture_tests {
    use crate::auth::{check_startup_auth_posture, compare_shared_secret, is_loopback_host};

    #[test]
    fn test_is_loopback_host() {
        assert!(is_loopback_host("127.0.0.1"));
        assert!(is_loopback_host("127.0.0.5"));
        assert!(is_loopback_host("::1"));
        assert!(is_loopback_host("localhost"));
        assert!(is_loopback_host("LocalHost"));
        assert!(!is_loopback_host("0.0.0.0"));
        assert!(!is_loopback_host("10.0.0.5"));
        assert!(!is_loopback_host("example.com"));
    }

    #[test]
    fn test_compare_shared_secret_noop_when_unconfigured() {
        // Not configured (None) => any token passes this layer.
        assert!(compare_shared_secret("anything", None).is_ok());
        assert!(compare_shared_secret("", None).is_ok());
    }

    #[test]
    fn test_compare_shared_secret_matches_and_rejects() {
        assert!(compare_shared_secret("s3cret-token", Some("s3cret-token")).is_ok());
        // Presence-only style token (B-4) must now be rejected when a real secret is set.
        assert!(compare_shared_secret("valid-token", Some("s3cret-token")).is_err());
        assert!(compare_shared_secret("", Some("s3cret-token")).is_err());
        assert!(compare_shared_secret("s3cret-token-extra", Some("s3cret-token")).is_err());
    }

    #[test]
    fn test_compare_shared_secret_length_independent() {
        // A15: comparison is over fixed-size SHA-256 digests, so tokens of very
        // different lengths still compare correctly (exact match passes, any
        // mismatch fails) without a length-based short-circuit.
        let secret = "a-reasonably-long-shared-secret-value";
        assert!(compare_shared_secret(secret, Some(secret)).is_ok());
        assert!(compare_shared_secret("x", Some(secret)).is_err());
        assert!(compare_shared_secret(&"z".repeat(4096), Some(secret)).is_err());
    }

    #[test]
    fn test_startup_posture_allows_loopback_without_auth() {
        // Disabled or presence-only auth on loopback is acceptable for local dev.
        assert!(check_startup_auth_posture(false, false, "127.0.0.1", false).is_ok());
        assert!(check_startup_auth_posture(false, false, "localhost", false).is_ok());
        assert!(check_startup_auth_posture(true, false, "::1", false).is_ok());
    }

    #[test]
    fn test_startup_posture_refuses_nonloopback_without_real_auth() {
        // Presence-only (auth enabled, no real auth) on a non-loopback bind must refuse.
        assert!(check_startup_auth_posture(true, false, "0.0.0.0", false).is_err());
        // Disabled auth on a non-loopback bind must refuse.
        assert!(check_startup_auth_posture(false, false, "0.0.0.0", false).is_err());
        assert!(check_startup_auth_posture(false, false, "10.0.0.5", false).is_err());
    }

    #[test]
    fn test_startup_posture_allows_nonloopback_with_real_auth() {
        assert!(check_startup_auth_posture(true, true, "0.0.0.0", false).is_ok());
    }

    #[test]
    fn test_startup_posture_allows_explicit_override() {
        // --i-know-this-is-insecure lets an operator opt into the risk.
        assert!(check_startup_auth_posture(false, false, "0.0.0.0", true).is_ok());
        assert!(check_startup_auth_posture(true, false, "0.0.0.0", true).is_ok());
    }
}

/// Auth-mode selection (Option 1): shared-secret and TokenReview are mutually
/// exclusive. A configured `BIND_API_TOKEN` selects shared-secret mode, in which
/// TokenReview is not consulted and the A2 fail-closed guard is not enforced.
#[cfg(test)]
mod auth_mode_selection_tests {
    use crate::auth::{has_real_auth, shared_secret_configured, BIND_API_TOKEN_ENV};
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_shared_secret_configured_true_when_token_set() {
        env::set_var(BIND_API_TOKEN_ENV, "s3cret-token");
        assert!(shared_secret_configured());
        env::remove_var(BIND_API_TOKEN_ENV);
    }

    #[test]
    #[serial]
    fn test_shared_secret_configured_false_when_unset() {
        env::remove_var(BIND_API_TOKEN_ENV);
        assert!(!shared_secret_configured());
    }

    #[test]
    #[serial]
    fn test_shared_secret_configured_false_when_empty() {
        // An empty value is not a usable secret and must not select the mode.
        env::set_var(BIND_API_TOKEN_ENV, "");
        assert!(!shared_secret_configured());
        env::remove_var(BIND_API_TOKEN_ENV);
    }

    #[test]
    #[serial]
    fn test_has_real_auth_true_with_shared_secret() {
        env::remove_var(BIND_API_TOKEN_ENV);
        env::set_var(BIND_API_TOKEN_ENV, "tok");
        assert!(has_real_auth());
        env::remove_var(BIND_API_TOKEN_ENV);
    }
}
