// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! DNS record data types — the pure request/response structs.
//!
//! Free of `axum`, `ApiError` and `AppState` so a library consumer importing
//! only `bindcar::AddRecordRequest` does not inherit the HTTP server stack.
//! The handlers that consume these live in [`crate::records`], which re-exports
//! everything here so existing `bindcar::records::AddRecordRequest` paths keep
//! working.
//!
//! See `.github/community/02-feature-gate-http-server.md` — phase 1 of putting
//! the server behind a cargo feature.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request to add a new DNS record
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddRecordRequest {
    /// Record name (e.g., "www", "@" for apex)
    pub name: String,

    /// Record type (e.g., "A", "AAAA", "CNAME", "MX", "TXT")
    #[serde(rename = "type")]
    pub record_type: String,

    /// Record value (e.g., "192.0.2.1" for A record)
    pub value: String,

    /// TTL in seconds (default: 3600)
    #[serde(default = "default_ttl")]
    pub ttl: u32,

    /// Priority (for MX and SRV records)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u16>,
}

/// Request to remove a DNS record
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRecordRequest {
    /// Record name (e.g., "www", "@" for apex)
    pub name: String,

    /// Record type (e.g., "A", "AAAA", "CNAME")
    #[serde(rename = "type")]
    pub record_type: String,

    /// Record value to remove (optional - if omitted, removes all records of this type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Request to update a DNS record
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecordRequest {
    /// Record name (e.g., "www", "@" for apex)
    pub name: String,

    /// Record type (e.g., "A", "AAAA", "CNAME")
    #[serde(rename = "type")]
    pub record_type: String,

    /// Current record value
    pub current_value: String,

    /// New record value
    pub new_value: String,

    /// TTL in seconds (default: 3600)
    #[serde(default = "default_ttl")]
    pub ttl: u32,

    /// Priority (for MX and SRV records)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u16>,
}

/// Response from record operations
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecordResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

fn default_ttl() -> u32 {
    3600
}
