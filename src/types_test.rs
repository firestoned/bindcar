// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for types module

use super::types::*;
use crate::rndc::RndcExecutor;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;

#[test]
fn test_app_state_clone() {
    let rndc = Arc::new(
        RndcExecutor::new(
            "127.0.0.1:953".to_string(),
            "sha256".to_string(),
            "dGVzdC1zZWNyZXQtaGVyZQ==".to_string(),
        )
        .expect("Failed to create RndcExecutor"),
    );
    let state = AppState {
        rndc: rndc.clone(),
        zone_dir: "/test/dir".to_string(),
    };

    let cloned = state.clone();
    assert_eq!(cloned.zone_dir, "/test/dir");
}

#[test]
fn test_error_response_serialization() {
    let response = ErrorResponse {
        error: "Test error".to_string(),
        details: Some("Details here".to_string()),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Test error"));
    assert!(json.contains("Details here"));
}

#[test]
fn test_error_response_without_details() {
    let response = ErrorResponse {
        error: "Test error".to_string(),
        details: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Test error"));
    assert!(json.contains("null")); // None is serialized as null
}

#[test]
fn test_api_error_zone_file_error() {
    let error = ApiError::ZoneFileError("Failed to write file".to_string());
    assert_eq!(error.to_string(), "Zone file error: Failed to write file");

    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_api_error_rndc_error() {
    let error = ApiError::RndcError("RNDC command failed".to_string());
    assert_eq!(
        error.to_string(),
        "RNDC command failed: RNDC command failed"
    );

    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_api_error_invalid_request() {
    let error = ApiError::InvalidRequest("Invalid zone name".to_string());
    assert_eq!(error.to_string(), "Invalid request: Invalid zone name");

    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_api_error_zone_not_found() {
    let error = ApiError::ZoneNotFound("example.com".to_string());
    assert_eq!(error.to_string(), "Zone not found: example.com");

    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_api_error_zone_already_exists() {
    let error = ApiError::ZoneAlreadyExists("example.com".to_string());
    assert_eq!(error.to_string(), "Zone already exists: example.com");

    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn test_api_error_internal_error() {
    let error = ApiError::InternalError("Database connection failed".to_string());
    assert_eq!(
        error.to_string(),
        "Internal server error: Database connection failed"
    );

    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_api_error_display() {
    let errors = vec![
        ApiError::ZoneFileError("file error".to_string()),
        ApiError::RndcError("rndc error".to_string()),
        ApiError::InvalidRequest("invalid".to_string()),
        ApiError::ZoneNotFound("test.com".to_string()),
        ApiError::ZoneAlreadyExists("test.com".to_string()),
        ApiError::InternalError("internal".to_string()),
    ];

    for error in errors {
        // Ensure all errors can be displayed
        let _ = format!("{}", error);
        let _ = format!("{:?}", error);
    }
}
