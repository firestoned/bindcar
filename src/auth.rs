// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Authentication middleware for Kubernetes ServiceAccount tokens
//!
//! This module validates that incoming requests include a valid ServiceAccount token
//! in the Authorization header.
//!
//! ## Token Validation Modes
//!
//! ### Basic Mode (default)
//! - Checks for token presence and format
//! - Suitable for trusted environments or when using external auth (API gateway, service mesh)
//!
//! ### Kubernetes TokenReview Mode (feature: `k8s-token-review`)
//! - Validates tokens against Kubernetes TokenReview API
//! - Verifies token authenticity and expiration
//! - Validates token audience
//! - Restricts to allowed namespaces and service accounts
//! - Requires in-cluster configuration or kubeconfig
//!
//! ## Security Configuration
//!
//! Environment variables for TokenReview mode:
//! - `BIND_TOKEN_AUDIENCES` - Comma-separated list of expected audiences (default: "bindcar")
//! - `BIND_ALLOWED_NAMESPACES` - Comma-separated list of allowed namespaces (empty = allow all)
//! - `BIND_ALLOWED_SERVICE_ACCOUNTS` - Comma-separated list of allowed SA names (empty = allow all)

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde::Serialize;
use tracing::{debug, warn};

#[cfg(feature = "k8s-token-review")]
use k8s_openapi::api::authentication::v1::TokenReview;
#[cfg(feature = "k8s-token-review")]
use kube::{
    config::{
        AuthInfo, Cluster, KubeConfigOptions, Kubeconfig, NamedAuthInfo, NamedCluster, NamedContext,
    },
    Api, Client, Config,
};
#[cfg(feature = "k8s-token-review")]
use std::env;
#[cfg(feature = "k8s-token-review")]
use tracing::error;

/// Error response for authentication failures
#[derive(Serialize)]
pub struct AuthError {
    pub error: String,
}

/// Configuration for TokenReview security policies
#[cfg(feature = "k8s-token-review")]
#[derive(Debug, Clone)]
pub struct TokenReviewConfig {
    /// Expected audiences for token validation
    pub audiences: Vec<String>,
    /// Allowed namespaces (empty = allow all)
    pub allowed_namespaces: Vec<String>,
    /// Allowed service accounts in format "system:serviceaccount:namespace:name" (empty = allow all)
    pub allowed_service_accounts: Vec<String>,
}

#[cfg(feature = "k8s-token-review")]
impl TokenReviewConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let audiences = match env::var("BIND_TOKEN_AUDIENCES") {
            Ok(val) if !val.trim().is_empty() => val
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            _ => vec!["bindcar".to_string()],
        };

        let allowed_namespaces = env::var("BIND_ALLOWED_NAMESPACES")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let allowed_service_accounts = env::var("BIND_ALLOWED_SERVICE_ACCOUNTS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let config = Self {
            audiences,
            allowed_namespaces,
            allowed_service_accounts,
        };

        debug!("TokenReview config loaded: audiences={:?}, allowed_namespaces={:?}, allowed_service_accounts={:?}",
            config.audiences, config.allowed_namespaces, config.allowed_service_accounts);

        config
    }

    /// Check if a namespace is allowed
    pub(crate) fn is_namespace_allowed(&self, namespace: &str) -> bool {
        // Empty list means allow all
        if self.allowed_namespaces.is_empty() {
            return true;
        }
        self.allowed_namespaces.contains(&namespace.to_string())
    }

    /// Check if a service account is allowed
    pub(crate) fn is_service_account_allowed(&self, username: &str) -> bool {
        // Empty list means allow all
        if self.allowed_service_accounts.is_empty() {
            return true;
        }
        self.allowed_service_accounts
            .contains(&username.to_string())
    }

    /// Extract namespace from service account username
    /// Format: "system:serviceaccount:namespace:name"
    pub(crate) fn extract_namespace(username: &str) -> Option<String> {
        let parts: Vec<&str> = username.split(':').collect();
        if parts.len() != 4 || parts[0] != "system" || parts[1] != "serviceaccount" {
            return None;
        }
        Some(parts[2].to_string())
    }
}

/// Describes how the Kubernetes client will be authenticated when performing TokenReview calls.
///
/// The mode is determined by environment variables at startup. When all three explicit
/// vars (`KUBE_API_SERVER`, `KUBE_TOKEN_PATH`, `KUBE_CA_CERT_PATH`) are present,
/// bindcar builds the client directly from those files. Otherwise it falls back to
/// `kube::Client::try_default()`, which checks `KUBECONFIG`, `~/.kube/config`, and then
/// the in-cluster ServiceAccount mount in order.
#[cfg(feature = "k8s-token-review")]
#[derive(Debug)]
pub enum KubeAuthMode {
    /// All three explicit env vars are set; use them to build the client.
    Explicit {
        server: String,
        token_path: String,
        ca_cert_path: String,
    },
    /// Fall back to `kube::Client::try_default()`.
    Default,
}

/// Inspect environment variables and return which Kubernetes auth mode should be used.
///
/// Returns `KubeAuthMode::Explicit` only when **all three** of `KUBE_API_SERVER`,
/// `KUBE_TOKEN_PATH`, and `KUBE_CA_CERT_PATH` are present. If only some are set a
/// warning is logged and `KubeAuthMode::Default` is returned so that existing
/// in-cluster and kubeconfig deployments are unaffected.
#[cfg(feature = "k8s-token-review")]
pub fn detect_kube_auth_mode() -> KubeAuthMode {
    let api_server = env::var("KUBE_API_SERVER").ok();
    let token_path = env::var("KUBE_TOKEN_PATH").ok();
    let ca_cert_path = env::var("KUBE_CA_CERT_PATH").ok();

    // Warn on partial configuration so operators notice misconfiguration early.
    let set_count = [&api_server, &token_path, &ca_cert_path]
        .iter()
        .filter(|v| v.is_some())
        .count();

    if set_count > 0 && set_count < 3 {
        warn!(
            "Partial KUBE_* env vars set ({}/3 present): \
             KUBE_API_SERVER={}, KUBE_TOKEN_PATH={}, KUBE_CA_CERT_PATH={}. \
             All three must be set for explicit auth. Falling back to try_default().",
            set_count,
            api_server.as_deref().unwrap_or("(not set)"),
            token_path.as_deref().unwrap_or("(not set)"),
            ca_cert_path.as_deref().unwrap_or("(not set)"),
        );
    }

    match (api_server, token_path, ca_cert_path) {
        (Some(server), Some(token_path), Some(ca_cert_path)) => KubeAuthMode::Explicit {
            server,
            token_path,
            ca_cert_path,
        },
        _ => KubeAuthMode::Default,
    }
}

/// Build a `kube::Client` from explicit file-based credentials.
///
/// Reads the token from `token_path` and the CA certificate from `ca_cert_path`,
/// then constructs an in-memory kubeconfig pointing at `server`.
///
/// # Errors
/// Returns an error string if either file cannot be read, the CA certificate is not
/// valid PEM, or the kube client cannot be initialized.
#[cfg(feature = "k8s-token-review")]
pub(crate) async fn build_explicit_kube_client(
    server: String,
    token_path: String,
    ca_cert_path: String,
) -> Result<Client, String> {
    // Validate token file is readable first so callers get a clear error message.
    // The content is not used here — kube will re-read the file on each API call via
    // token_file in the kubeconfig, which is correct for rotating SA tokens.
    tokio::fs::read_to_string(&token_path)
        .await
        .map_err(|e| format!("failed to read token file '{}': {}", token_path, e))?;

    // Validate CA certificate file is readable and contains a PEM certificate block.
    // kube defers TLS setup to the first API call, so we validate eagerly here to
    // surface misconfiguration at startup rather than on the first request.
    let ca_bytes = tokio::fs::read(&ca_cert_path).await.map_err(|e| {
        format!(
            "failed to read CA certificate file '{}': {}",
            ca_cert_path, e
        )
    })?;
    let ca_pem = String::from_utf8_lossy(&ca_bytes);
    if !ca_pem.contains("-----BEGIN CERTIFICATE-----") {
        return Err(format!(
            "CA certificate file '{}' does not contain a valid PEM certificate block",
            ca_cert_path
        ));
    }

    // Build an in-memory kubeconfig using file paths. kube will re-read the token
    // file on each API call (correct for short-lived, rotating SA tokens) and will
    // parse the CA cert PEM when building the TLS stack.
    let kubeconfig = Kubeconfig {
        clusters: vec![NamedCluster {
            name: "standalone".to_string(),
            cluster: Some(Cluster {
                server: Some(server),
                certificate_authority: Some(ca_cert_path),
                ..Default::default()
            }),
        }],
        auth_infos: vec![NamedAuthInfo {
            name: "standalone".to_string(),
            auth_info: Some(AuthInfo {
                // Use token_file so the token is re-read on rotation, not embedded.
                token_file: Some(token_path),
                ..Default::default()
            }),
        }],
        contexts: vec![NamedContext {
            name: "standalone".to_string(),
            context: Some(kube::config::Context {
                cluster: "standalone".to_string(),
                user: Some("standalone".to_string()),
                ..Default::default()
            }),
        }],
        current_context: Some("standalone".to_string()),
        ..Default::default()
    };

    let config = Config::from_custom_kubeconfig(kubeconfig, &KubeConfigOptions::default())
        .await
        .map_err(|e| format!("failed to build Kubernetes client config: {}", e))?;

    Client::try_from(config).map_err(|e| format!("failed to create Kubernetes client: {}", e))
}

/// Build a `kube::Client` using the resolved auth mode.
///
/// Priority order:
/// 1. Explicit env vars (`KUBE_API_SERVER` + `KUBE_TOKEN_PATH` + `KUBE_CA_CERT_PATH`)
/// 2. `KUBECONFIG` env / `~/.kube/config` / in-cluster SA mount (via `try_default`)
#[cfg(feature = "k8s-token-review")]
async fn build_kube_client() -> Result<Client, String> {
    match detect_kube_auth_mode() {
        KubeAuthMode::Explicit {
            server,
            token_path,
            ca_cert_path,
        } => {
            debug!(
                "Kubernetes auth mode: explicit (KUBE_API_SERVER={})",
                server
            );
            build_explicit_kube_client(server, token_path, ca_cert_path).await
        }
        KubeAuthMode::Default => {
            debug!("Kubernetes auth mode: try_default (KUBECONFIG / ~/.kube/config / in-cluster)");
            Client::try_default()
                .await
                .map_err(|e| format!("Failed to create Kubernetes client: {}", e))
        }
    }
}

/// Authentication middleware
///
/// Validates that the request includes a Bearer token in the Authorization header.
/// This token should be a Kubernetes ServiceAccount token.
///
/// # Headers
/// - `Authorization: Bearer <token>` - Required
///
/// # Errors
/// Returns 401 Unauthorized if:
/// - No Authorization header is present
/// - Authorization header is malformed
/// - Token is invalid (future implementation)
pub async fn authenticate(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<AuthError>)> {
    // Extract Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing Authorization header");
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "Missing Authorization header".to_string(),
                }),
            )
        })?;

    // Check Bearer token format
    if !auth_header.starts_with("Bearer ") {
        warn!("Invalid Authorization header format");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(AuthError {
                error: "Invalid Authorization header format. Expected: Bearer <token>".to_string(),
            }),
        ));
    }

    let token = &auth_header[7..]; // Skip "Bearer "

    // Basic validation: token should not be empty
    if token.is_empty() {
        warn!("Empty token in Authorization header");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(AuthError {
                error: "Empty token".to_string(),
            }),
        ));
    }

    // Validate token with Kubernetes TokenReview API if feature is enabled
    #[cfg(feature = "k8s-token-review")]
    if let Err(e) = validate_token_with_k8s(token).await {
        warn!("Token validation failed: {}", e);
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(AuthError {
                error: format!("Token validation failed: {}", e),
            }),
        ));
    }

    #[cfg(feature = "k8s-token-review")]
    debug!("Token validated with Kubernetes TokenReview API");

    #[cfg(not(feature = "k8s-token-review"))]
    debug!("Token validation: basic mode (presence check only)");

    Ok(next.run(request).await)
}

/// Validate a token using Kubernetes TokenReview API
///
/// This function sends the token to the Kubernetes API server for validation.
/// It verifies that the token is authentic, not expired, and belongs to a valid
/// service account.
///
/// Additionally validates:
/// - Token audience matches expected audiences
/// - Service account namespace is in allowed list (if configured)
/// - Service account name is in allowed list (if configured)
///
/// # Arguments
/// * `token` - The bearer token to validate
///
/// # Returns
/// * `Ok(())` if the token is valid and authorized
/// * `Err(String)` if validation fails
#[cfg(feature = "k8s-token-review")]
pub(crate) async fn validate_token_with_k8s(token: &str) -> Result<(), String> {
    // Load security configuration
    let config = TokenReviewConfig::from_env();

    // Create Kubernetes client using the resolved auth mode.
    let client = build_kube_client().await?;

    // Create TokenReview API client
    let token_reviews: Api<TokenReview> = Api::all(client);

    // Build TokenReview request with audience validation
    let audiences = if !config.audiences.is_empty() {
        Some(config.audiences.clone())
    } else {
        None
    };

    let token_review = TokenReview {
        metadata: Default::default(),
        spec: k8s_openapi::api::authentication::v1::TokenReviewSpec {
            token: Some(token.to_string()),
            audiences,
        },
        status: None,
    };

    // Submit TokenReview request
    let result = token_reviews
        .create(&Default::default(), &token_review)
        .await
        .map_err(|e| {
            error!("TokenReview API call failed: {}", e);
            format!("Failed to validate token with Kubernetes API: {}", e)
        })?;

    // Check if token is authenticated
    let status = result
        .status
        .ok_or_else(|| "TokenReview status not available".to_string())?;

    if status.authenticated != Some(true) {
        let error_msg = status
            .error
            .unwrap_or_else(|| "Token not authenticated".to_string());
        warn!("Token authentication failed: {}", error_msg);
        return Err(error_msg);
    }

    debug!("Token authenticated successfully");

    // Validate user information and authorization
    let user = status.user.ok_or_else(|| {
        warn!("TokenReview succeeded but no user information returned");
        "No user information in TokenReview response".to_string()
    })?;

    let username = user.username.as_deref().unwrap_or("");
    debug!("Authenticated user: {}", username);

    // Validate namespace restriction
    if let Some(namespace) = TokenReviewConfig::extract_namespace(username) {
        if !config.is_namespace_allowed(&namespace) {
            warn!(
                "ServiceAccount from unauthorized namespace: {} (from {})",
                namespace, username
            );
            return Err(format!(
                "ServiceAccount from unauthorized namespace: {}",
                namespace
            ));
        }
        debug!("Namespace {} is allowed", namespace);
    }

    // Validate service account allowlist
    if !config.is_service_account_allowed(username) {
        warn!("ServiceAccount not in allowlist: {}", username);
        return Err(format!("ServiceAccount not authorized: {}", username));
    }

    debug!("ServiceAccount {} is allowed", username);
    Ok(())
}
