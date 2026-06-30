// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for types module

use super::types::*;
use crate::{nsupdate::NsupdateExecutor, rndc::RndcExecutor};
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
    let nsupdate = Arc::new(
        NsupdateExecutor::new(
            "127.0.0.1".to_string(),
            53,
            Some("test-key".to_string()),
            Some("HMAC-SHA256".to_string()),
            Some("dGVzdC1zZWNyZXQtaGVyZQ==".to_string()),
        )
        .expect("Failed to create NsupdateExecutor"),
    );
    let state = AppState {
        rndc: rndc.clone(),
        nsupdate: nsupdate.clone(),
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
fn test_api_error_dynamic_updates_not_enabled() {
    let error = ApiError::DynamicUpdatesNotEnabled("example.com".to_string());
    assert_eq!(
        error.to_string(),
        "Dynamic updates not enabled: example.com"
    );

    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_api_error_nsupdate_error() {
    let error = ApiError::NsupdateError("nsupdate failed".to_string());
    assert_eq!(
        error.to_string(),
        "nsupdate command failed: nsupdate failed"
    );

    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_api_error_invalid_record() {
    let error = ApiError::InvalidRecord("invalid IP address".to_string());
    assert_eq!(error.to_string(), "Invalid record: invalid IP address");

    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// A-3: 5xx response bodies must NOT echo internal detail (raw rndc/nsupdate
/// stderr, paths). They are replaced with a generic message; the detail is
/// logged server-side only.
#[tokio::test]
async fn test_5xx_error_body_is_sanitized() {
    let sensitive = "/etc/bind/rndc.key: secret \"S3CR3T\" leaked via stderr";
    let cases = vec![
        ApiError::RndcError(sensitive.to_string()).into_response(),
        ApiError::NsupdateError(sensitive.to_string()).into_response(),
        ApiError::ZoneFileError(sensitive.to_string()).into_response(),
        ApiError::InternalError(sensitive.to_string()).into_response(),
    ];

    for response in cases {
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(
            !body.contains("S3CR3T") && !body.contains("rndc.key"),
            "5xx body leaked internal detail: {body}"
        );
        assert!(
            body.contains("Internal server error"),
            "5xx body should carry the generic message: {body}"
        );
    }
}

/// A-3: 4xx (client-fault) bodies still carry the helpful, safe detail about
/// the caller's own request.
#[tokio::test]
async fn test_4xx_error_body_retains_detail() {
    let response =
        ApiError::InvalidRequest("zone name cannot be empty".to_string()).into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("zone name cannot be empty"), "got: {body}");
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
        ApiError::DynamicUpdatesNotEnabled("test.com".to_string()),
        ApiError::NsupdateError("nsupdate error".to_string()),
        ApiError::InvalidRecord("invalid".to_string()),
    ];

    for error in errors {
        // Ensure all errors can be displayed
        let _ = format!("{}", error);
        let _ = format!("{:?}", error);
    }
}
