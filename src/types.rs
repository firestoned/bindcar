// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Common types and errors used throughout the bindcar library

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::error;

use crate::{nsupdate::NsupdateExecutor, rndc::RndcExecutor};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// RNDC command executor
    pub rndc: Arc<RndcExecutor>,
    /// nsupdate command executor
    pub nsupdate: Arc<NsupdateExecutor>,
    /// Zone file directory
    pub zone_dir: String,
}

/// Error response
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

/// API error type
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Zone file error: {0}")]
    ZoneFileError(String),

    #[error("RNDC command failed: {0}")]
    RndcError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Zone not found: {0}")]
    ZoneNotFound(String),

    #[error("Zone already exists: {0}")]
    ZoneAlreadyExists(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Dynamic updates not enabled: {0}")]
    DynamicUpdatesNotEnabled(String),

    #[error("nsupdate command failed: {0}")]
    NsupdateError(String),

    #[error("Invalid record: {0}")]
    InvalidRecord(String),
}

/// Generic, non-revealing message returned to clients for any 5xx error.
///
/// The detailed cause (raw `rndc`/`nsupdate` stderr, internal filesystem paths,
/// kube API-server errors) is logged server-side instead. Returning it to the
/// caller is an information-disclosure vector (A-3): BIND/nsupdate stderr can
/// leak key names, zone-internal configuration, server addresses, and file
/// contents useful for further attack.
const GENERIC_SERVER_ERROR: &str = "Internal server error";

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // 4xx variants describe a problem with the caller's own request (their
        // input, a missing/duplicate zone) and are safe — and useful — to
        // return verbatim. 5xx variants carry internal detail and are replaced
        // with a generic message after the full error is logged server-side.
        let (status, error_message) = match &self {
            ApiError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::ZoneNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::ZoneAlreadyExists(_) => (StatusCode::CONFLICT, self.to_string()),
            ApiError::DynamicUpdatesNotEnabled(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::InvalidRecord(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::ZoneFileError(_)
            | ApiError::RndcError(_)
            | ApiError::InternalError(_)
            | ApiError::NsupdateError(_) => {
                error!("returning 500 to client; internal error: {}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    GENERIC_SERVER_ERROR.to_string(),
                )
            }
        };

        let body = Json(ErrorResponse {
            error: error_message,
            details: None,
        });

        (status, body).into_response()
    }
}
