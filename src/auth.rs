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
use kube::{Api, Client};
#[cfg(feature = "k8s-token-review")]
use tracing::error;
#[cfg(feature = "k8s-token-review")]
use std::env;

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
        self.allowed_service_accounts.contains(&username.to_string())
    }

    /// Extract namespace from service account username
    /// Format: "system:serviceaccount:namespace:name"
    pub(crate) fn extract_namespace(username: &str) -> Option<String> {
        let parts: Vec<&str> = username.split(':').collect();
        if parts.len() == 4 && parts[0] == "system" && parts[1] == "serviceaccount" {
            Some(parts[2].to_string())
        } else {
            None
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
    {
        if let Err(e) = validate_token_with_k8s(token).await {
            warn!("Token validation failed: {}", e);
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: format!("Token validation failed: {}", e),
                }),
            ));
        }
        debug!("Token validated with Kubernetes TokenReview API");
    }

    #[cfg(not(feature = "k8s-token-review"))]
    {
        debug!("Token validation: basic mode (presence check only)");
    }

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

    // Create Kubernetes client
    let client = Client::try_default()
        .await
        .map_err(|e| format!("Failed to create Kubernetes client: {}", e))?;

    // Create TokenReview API client
    let token_reviews: Api<TokenReview> = Api::all(client);

    // Build TokenReview request with audience validation
    let audiences = if config.audiences.is_empty() {
        None
    } else {
        Some(config.audiences.clone())
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
    if let Some(status) = result.status {
        if let Some(true) = status.authenticated {
            debug!("Token authenticated successfully");

            // Validate user information and authorization
            if let Some(user) = status.user {
                let username = user.username.as_deref().unwrap_or("");
                debug!("Authenticated user: {}", username);

                // Validate namespace restriction
                if let Some(namespace) = TokenReviewConfig::extract_namespace(username) {
                    if !config.is_namespace_allowed(&namespace) {
                        warn!("ServiceAccount from unauthorized namespace: {} (from {})", namespace, username);
                        return Err(format!("ServiceAccount from unauthorized namespace: {}", namespace));
                    }
                    debug!("Namespace {} is allowed", namespace);
                }

                // Validate service account allowlist
                if !config.is_service_account_allowed(username) {
                    warn!("ServiceAccount not in allowlist: {}", username);
                    return Err(format!("ServiceAccount not authorized: {}", username));
                }
                debug!("ServiceAccount {} is allowed", username);
            } else {
                warn!("TokenReview succeeded but no user information returned");
                return Err("No user information in TokenReview response".to_string());
            }

            Ok(())
        } else {
            let error_msg = status
                .error
                .unwrap_or_else(|| "Token not authenticated".to_string());
            warn!("Token authentication failed: {}", error_msg);
            Err(error_msg)
        }
    } else {
        Err("TokenReview status not available".to_string())
    }
}
