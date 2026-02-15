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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ApiError::ZoneFileError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::RndcError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::ZoneNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::ZoneAlreadyExists(_) => (StatusCode::CONFLICT, self.to_string()),
            ApiError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::DynamicUpdatesNotEnabled(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::NsupdateError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::InvalidRecord(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };

        let body = Json(ErrorResponse {
            error: error_message,
            details: None,
        });

        (status, body).into_response()
    }
}
