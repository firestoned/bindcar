// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Authentication middleware for Kubernetes ServiceAccount tokens
//!
//! This module validates that incoming requests include a valid ServiceAccount token
//! in the Authorization header. In a production environment, this should validate
//! the token against the Kubernetes API server.
//!
//! For now, we implement a simple token presence check. Future enhancements:
//! - Validate token signature with Kubernetes API
//! - Check token expiration
//! - Verify service account permissions

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde::Serialize;
use tracing::{debug, warn};

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

    // TODO: Validate token with Kubernetes API
    // For now, we just check that a token is present
    // In production, you should:
    // 1. Use the TokenReview API to validate the token
    // 2. Check that the service account has appropriate permissions
    // 3. Verify token is not expired

    debug!("Request authenticated (token present)");

    Ok(next.run(request).await)
}
