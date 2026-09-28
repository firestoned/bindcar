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
    /// DNSSEC key directory (`BIND_KEY_DIR`), canonicalized at startup.
    /// `None` when not configured; only the DS endpoint requires it
    /// (ADR-0001) — everything else works without it.
    pub key_dir: Option<String>,
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

    /// A DNSSEC transition that would take the zone dark for validating
    /// resolvers (ADR-0001), e.g. removing `dnssec-policy` while DNSKEY
    /// records are still served. 409: the request conflicts with the zone's
    /// current signing state, not with its syntax.
    #[error("Unsafe DNSSEC transition: {0}")]
    UnsafeDnssecTransition(String),

    /// The requested DNSSEC material does not exist for this zone (e.g. DS
    /// records of an unsigned zone). 404: the sub-resource is absent.
    #[error("DNSSEC material not available: {0}")]
    DsNotAvailable(String),

    /// The deployment lacks a prerequisite for this endpoint (e.g. the DS
    /// endpoint without a `BIND_KEY_DIR` mount, ADR-0001). 501: the server
    /// cannot fulfil the request until it is reconfigured.
    #[error("Not configured on this server: {0}")]
    NotConfigured(String),
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
            ApiError::UnsafeDnssecTransition(_) => (StatusCode::CONFLICT, self.to_string()),
            ApiError::DsNotAvailable(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::NotConfigured(_) => (StatusCode::NOT_IMPLEMENTED, self.to_string()),
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
