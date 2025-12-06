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
//! - Requires in-cluster configuration or kubeconfig

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

/// Error response for authentication failures
#[derive(Serialize)]
pub struct AuthError {
    pub error: String,
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
/// # Arguments
/// * `token` - The bearer token to validate
///
/// # Returns
/// * `Ok(())` if the token is valid
/// * `Err(String)` if validation fails
#[cfg(feature = "k8s-token-review")]
async fn validate_token_with_k8s(token: &str) -> Result<(), String> {
    // Create Kubernetes client
    let client = Client::try_default()
        .await
        .map_err(|e| format!("Failed to create Kubernetes client: {}", e))?;

    // Create TokenReview API client
    let token_reviews: Api<TokenReview> = Api::all(client);

    // Build TokenReview request
    let token_review = TokenReview {
        metadata: Default::default(),
        spec: k8s_openapi::api::authentication::v1::TokenReviewSpec {
            token: Some(token.to_string()),
            audiences: None,
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
            if let Some(user) = status.user {
                debug!("Authenticated user: {:?}", user.username);
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
