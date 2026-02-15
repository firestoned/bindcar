// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! DNS record management API handlers
//!
//! This module implements HTTP handlers for individual DNS record operations:
//! - Adding records to existing zones
//! - Removing records from existing zones
//! - Updating existing records
//!
//! All operations use nsupdate for dynamic DNS updates with TSIG authentication.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use utoipa::ToSchema;

use crate::{
    metrics, rndc_parser, rndc_types,
    types::{ApiError, AppState},
};

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

/// Supported DNS record types
const VALID_RECORD_TYPES: &[&str] = &["A", "AAAA", "CNAME", "MX", "TXT", "NS", "PTR", "SRV", "CAA"];

/// Validate that a zone exists and supports dynamic updates
///
/// # Arguments
///
/// * `state` - Application state
/// * `zone_name` - Zone name to validate
///
/// # Returns
///
/// Ok if zone exists and has allow-update configured, Err otherwise
async fn validate_zone_for_updates(state: &AppState, zone_name: &str) -> Result<(), ApiError> {
    // Validate zone name is not empty
    if zone_name.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Zone name cannot be empty".to_string(),
        ));
    }

    // Get zone configuration via RNDC
    let zone_config_output = state.rndc.showzone(zone_name).await.map_err(|e| {
        if e.to_string().contains("not found") {
            ApiError::ZoneNotFound(zone_name.to_string())
        } else {
            ApiError::RndcError(e.to_string())
        }
    })?;

    // Parse zone configuration
    let zone_config = rndc_parser::parse_showzone(&zone_config_output).map_err(|e| {
        ApiError::InternalError(format!("Failed to parse zone configuration: {}", e))
    })?;

    // Zone must be primary type
    if zone_config.zone_type != rndc_types::ZoneType::Primary {
        return Err(ApiError::DynamicUpdatesNotEnabled(format!(
            "Zone {} is {} type. Dynamic updates only supported on primary zones",
            zone_name,
            zone_config.zone_type.as_str()
        )));
    }

    // Zone must have allow-update configured
    if zone_config.allow_update.is_none() && zone_config.allow_update_raw.is_none() {
        return Err(ApiError::DynamicUpdatesNotEnabled(format!(
            "Zone {} does not have allow-update configured. \
            Create zone with updateKeyName or modify zone to enable dynamic updates",
            zone_name
        )));
    }

    Ok(())
}

/// Validate DNS record type
fn validate_record_type(record_type: &str) -> Result<(), ApiError> {
    let upper = record_type.to_uppercase();

    if !VALID_RECORD_TYPES.contains(&upper.as_str()) {
        return Err(ApiError::InvalidRecord(format!(
            "Invalid record type: {}. Supported types: {:?}",
            record_type, VALID_RECORD_TYPES
        )));
    }

    Ok(())
}

/// Validate DNS record value based on type
fn validate_record_value(record_type: &str, value: &str) -> Result<(), ApiError> {
    // Value cannot be empty
    if value.is_empty() {
        return Err(ApiError::InvalidRecord(
            "Record value cannot be empty".to_string(),
        ));
    }

    match record_type.to_uppercase().as_str() {
        "A" => {
            // Validate IPv4 address
            value
                .parse::<std::net::Ipv4Addr>()
                .map_err(|_| ApiError::InvalidRecord(format!("Invalid IPv4 address: {}", value)))?;
        }
        "AAAA" => {
            // Validate IPv6 address
            value
                .parse::<std::net::Ipv6Addr>()
                .map_err(|_| ApiError::InvalidRecord(format!("Invalid IPv6 address: {}", value)))?;
        }
        "CNAME" | "NS" | "PTR" | "MX" => {
            // Must be FQDN with trailing dot
            if !value.ends_with('.') {
                return Err(ApiError::InvalidRecord(format!(
                    "{} record value must be a fully qualified domain name ending with '.': {}",
                    record_type, value
                )));
            }
        }
        "TXT" | "CAA" | "SRV" => {
            // Any non-empty string is valid
        }
        _ => {}
    }

    Ok(())
}

/// Normalize record name to FQDN
///
/// # Arguments
///
/// * `name` - Record name (may be relative, @, or FQDN)
/// * `zone` - Zone name
///
/// # Returns
///
/// Fully qualified domain name with trailing dot
fn normalize_record_name(name: &str, zone: &str) -> String {
    if name == "@" {
        // Apex record - use zone name
        format!("{}.", zone)
    } else if name.ends_with('.') {
        // Already a FQDN
        name.to_string()
    } else if name.contains('.') && name.ends_with(zone) {
        // FQDN without trailing dot
        format!("{}.", name)
    } else {
        // Relative name - append zone
        format!("{}.{}.", name, zone)
    }
}

/// Add a DNS record to an existing zone
#[utoipa::path(
    post,
    path = "/api/v1/zones/{zone_name}/records",
    request_body = AddRecordRequest,
    params(
        ("zone_name" = String, Path, description = "Zone name")
    ),
    responses(
        (status = 201, description = "Record added successfully", body = RecordResponse),
        (status = 400, description = "Invalid request or zone not configured for updates"),
        (status = 404, description = "Zone not found"),
        (status = 500, description = "Update failed"),
    ),
    tag = "records"
)]
pub async fn add_record(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
    Json(request): Json<AddRecordRequest>,
) -> Result<(StatusCode, Json<RecordResponse>), ApiError> {
    info!(
        "Adding record to zone {}: {} {} {} (TTL: {})",
        zone_name, request.name, request.record_type, request.value, request.ttl
    );

    // Early return pattern: validate all prerequisites
    validate_zone_for_updates(&state, &zone_name).await?;
    validate_record_type(&request.record_type)?;
    validate_record_value(&request.record_type, &request.value)?;

    // Normalize record name to FQDN
    let fqdn = normalize_record_name(&request.name, &zone_name);

    debug!("Normalized record name: {} -> {}", request.name, fqdn);

    // For MX and SRV records, prepend priority to value
    let value_with_priority = if let Some(priority) = request.priority {
        if request.record_type.to_uppercase() == "MX" || request.record_type.to_uppercase() == "SRV"
        {
            format!("{} {}", priority, request.value)
        } else {
            request.value.clone()
        }
    } else {
        request.value.clone()
    };

    // Execute nsupdate
    let _output = state
        .nsupdate
        .add_record(
            &zone_name,
            &fqdn,
            request.ttl,
            &request.record_type,
            &value_with_priority,
        )
        .await
        .map_err(|e| {
            error!("nsupdate add failed: {}", e);
            metrics::record_record_operation("add", false);
            ApiError::NsupdateError(format!("Failed to add record: {}", e))
        })?;

    info!("Record added successfully to zone {}", zone_name);
    metrics::record_record_operation("add", true);

    Ok((
        StatusCode::CREATED,
        Json(RecordResponse {
            success: true,
            message: format!("Record added to zone {}", zone_name),
            details: Some(serde_json::json!({
                "zone": zone_name,
                "record": {
                    "name": request.name,
                    "type": request.record_type,
                    "value": request.value,
                    "ttl": request.ttl,
                }
            })),
        }),
    ))
}

/// Remove a DNS record from an existing zone
#[utoipa::path(
    delete,
    path = "/api/v1/zones/{zone_name}/records",
    request_body = RemoveRecordRequest,
    params(
        ("zone_name" = String, Path, description = "Zone name")
    ),
    responses(
        (status = 200, description = "Record removed successfully", body = RecordResponse),
        (status = 400, description = "Invalid request or zone not configured for updates"),
        (status = 404, description = "Zone not found"),
        (status = 500, description = "Update failed"),
    ),
    tag = "records"
)]
pub async fn remove_record(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
    Json(request): Json<RemoveRecordRequest>,
) -> Result<Json<RecordResponse>, ApiError> {
    info!(
        "Removing record from zone {}: {} {} {:?}",
        zone_name, request.name, request.record_type, request.value
    );

    // Early return pattern: validate all prerequisites
    validate_zone_for_updates(&state, &zone_name).await?;
    validate_record_type(&request.record_type)?;

    // Validate value if provided
    if let Some(ref value) = request.value {
        validate_record_value(&request.record_type, value)?;
    }

    // Normalize record name to FQDN
    let fqdn = normalize_record_name(&request.name, &zone_name);

    debug!("Normalized record name: {} -> {}", request.name, fqdn);

    // Execute nsupdate
    let value_str = request.value.as_deref().unwrap_or("");
    let _output = state
        .nsupdate
        .remove_record(&zone_name, &fqdn, &request.record_type, value_str)
        .await
        .map_err(|e| {
            error!("nsupdate remove failed: {}", e);
            metrics::record_record_operation("remove", false);
            ApiError::NsupdateError(format!("Failed to remove record: {}", e))
        })?;

    info!("Record removed successfully from zone {}", zone_name);
    metrics::record_record_operation("remove", true);

    Ok(Json(RecordResponse {
        success: true,
        message: format!("Record removed from zone {}", zone_name),
        details: Some(serde_json::json!({
            "zone": zone_name,
            "record": {
                "name": request.name,
                "type": request.record_type,
                "value": request.value,
            }
        })),
    }))
}

/// Update a DNS record in an existing zone
#[utoipa::path(
    put,
    path = "/api/v1/zones/{zone_name}/records",
    request_body = UpdateRecordRequest,
    params(
        ("zone_name" = String, Path, description = "Zone name")
    ),
    responses(
        (status = 200, description = "Record updated successfully", body = RecordResponse),
        (status = 400, description = "Invalid request or zone not configured for updates"),
        (status = 404, description = "Zone not found"),
        (status = 500, description = "Update failed"),
    ),
    tag = "records"
)]
pub async fn update_record(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
    Json(request): Json<UpdateRecordRequest>,
) -> Result<Json<RecordResponse>, ApiError> {
    info!(
        "Updating record in zone {}: {} {} from {} to {} (TTL: {})",
        zone_name,
        request.name,
        request.record_type,
        request.current_value,
        request.new_value,
        request.ttl
    );

    // Early return pattern: validate all prerequisites
    validate_zone_for_updates(&state, &zone_name).await?;
    validate_record_type(&request.record_type)?;
    validate_record_value(&request.record_type, &request.current_value)?;
    validate_record_value(&request.record_type, &request.new_value)?;

    // Normalize record name to FQDN
    let fqdn = normalize_record_name(&request.name, &zone_name);

    debug!("Normalized record name: {} -> {}", request.name, fqdn);

    // For MX and SRV records, prepend priority to values
    let (current_with_priority, new_with_priority) = if let Some(priority) = request.priority {
        if request.record_type.to_uppercase() == "MX" || request.record_type.to_uppercase() == "SRV"
        {
            (
                format!("{} {}", priority, request.current_value),
                format!("{} {}", priority, request.new_value),
            )
        } else {
            (request.current_value.clone(), request.new_value.clone())
        }
    } else {
        (request.current_value.clone(), request.new_value.clone())
    };

    // Execute nsupdate
    let _output = state
        .nsupdate
        .update_record(
            &zone_name,
            &fqdn,
            request.ttl,
            &request.record_type,
            &current_with_priority,
            &new_with_priority,
        )
        .await
        .map_err(|e| {
            error!("nsupdate update failed: {}", e);
            metrics::record_record_operation("update", false);
            ApiError::NsupdateError(format!("Failed to update record: {}", e))
        })?;

    info!("Record updated successfully in zone {}", zone_name);
    metrics::record_record_operation("update", true);

    Ok(Json(RecordResponse {
        success: true,
        message: format!("Record updated in zone {}", zone_name),
        details: Some(serde_json::json!({
            "zone": zone_name,
            "record": {
                "name": request.name,
                "type": request.record_type,
                "currentValue": request.current_value,
                "newValue": request.new_value,
                "ttl": request.ttl,
            }
        })),
    }))
}
