// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Zone management API handlers
//!
//! This module implements HTTP handlers for all zone-related operations:
//! - Creating zones (with zone file creation)
//! - Deleting zones
//! - Reloading zones
//! - Getting zone status
//! - Freezing/thawing zones
//! - Notifying secondaries

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info};
use utoipa::ToSchema;

use crate::{
    metrics,
    types::{ApiError, AppState},
};

/// SOA (Start of Authority) record configuration
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SoaRecord {
    /// Primary nameserver (e.g., "ns1.example.com.")
    pub primary_ns: String,

    /// Admin email (e.g., "admin.example.com.")
    pub admin_email: String,

    /// Serial number (e.g., 2025010101)
    pub serial: u32,

    /// Refresh interval in seconds (default: 3600)
    #[serde(default = "default_refresh")]
    pub refresh: u32,

    /// Retry interval in seconds (default: 600)
    #[serde(default = "default_retry")]
    pub retry: u32,

    /// Expire time in seconds (default: 604800)
    #[serde(default = "default_expire")]
    pub expire: u32,

    /// Negative TTL in seconds (default: 86400)
    #[serde(default = "default_negative_ttl")]
    pub negative_ttl: u32,
}

fn default_refresh() -> u32 {
    3600
}

fn default_retry() -> u32 {
    600
}

fn default_expire() -> u32 {
    604_800
}

fn default_negative_ttl() -> u32 {
    86400
}

/// DNS record entry
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DnsRecord {
    /// Record name (e.g., "www", "@")
    pub name: String,

    /// Record type (e.g., "A", "AAAA", "CNAME", "MX", "TXT")
    #[serde(rename = "type")]
    pub record_type: String,

    /// Record value (e.g., "192.0.2.1", "example.com.")
    pub value: String,

    /// Optional TTL (uses zone default if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u32>,

    /// Optional priority (for MX, SRV records)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u16>,
}

/// Structured zone configuration
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ZoneConfig {
    /// Default TTL for the zone (e.g., 3600)
    pub ttl: u32,

    /// SOA record
    pub soa: SoaRecord,

    /// Name servers for the zone
    pub name_servers: Vec<String>,

    /// DNS records in the zone
    #[serde(default)]
    pub records: Vec<DnsRecord>,
}

impl ZoneConfig {
    /// Generate BIND9 zone file content from structured configuration
    pub fn to_zone_file(&self) -> String {
        let mut zone_file = String::new();

        // TTL directive
        zone_file.push_str(&format!("$TTL {}\n\n", self.ttl));

        // SOA record
        zone_file.push_str(&format!(
            "@ IN SOA {} {} (\n",
            self.soa.primary_ns, self.soa.admin_email
        ));
        zone_file.push_str(&format!("    {}  ; Serial\n", self.soa.serial));
        zone_file.push_str(&format!("    {}  ; Refresh\n", self.soa.refresh));
        zone_file.push_str(&format!("    {}  ; Retry\n", self.soa.retry));
        zone_file.push_str(&format!("    {}  ; Expire\n", self.soa.expire));
        zone_file.push_str(&format!(
            "    {} ); Negative TTL\n\n",
            self.soa.negative_ttl
        ));

        // Name servers
        for ns in &self.name_servers {
            zone_file.push_str(&format!("@ IN NS {}\n", ns));
        }

        if !self.name_servers.is_empty() {
            zone_file.push('\n');
        }

        // DNS records
        for record in &self.records {
            let ttl_str = if let Some(ttl) = record.ttl {
                format!("{} ", ttl)
            } else {
                String::new()
            };

            let priority_str = if let Some(priority) = record.priority {
                format!("{} ", priority)
            } else {
                String::new()
            };

            zone_file.push_str(&format!(
                "{} {}IN {} {}{}\n",
                record.name, ttl_str, record.record_type, priority_str, record.value
            ));
        }

        zone_file
    }
}

/// Request to create a new zone
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateZoneRequest {
    /// Zone name (e.g., "example.com")
    pub zone_name: String,

    /// Zone type ("master" or "slave")
    pub zone_type: String,

    /// Structured zone configuration
    pub zone_config: ZoneConfig,

    /// Optional: TSIG key name for allow-update
    pub update_key_name: Option<String>,
}

/// Response from zone operations
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZoneResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Server status response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServerStatusResponse {
    pub status: String,
}

/// Zone information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ZoneInfo {
    pub name: String,
    pub zone_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// List of zones response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZoneListResponse {
    pub zones: Vec<String>,
    pub count: usize,
}

/// Create a new zone
///
/// This endpoint:
/// 1. Generates zone file from structured configuration
/// 2. Writes the zone file to disk
/// 3. Executes `rndc addzone` to add the zone to BIND9
#[utoipa::path(
    post,
    path = "/api/v1/zones",
    request_body = CreateZoneRequest,
    responses(
        (status = 201, description = "Zone created successfully", body = ZoneResponse),
        (status = 400, description = "Invalid request"),
        (status = 502, description = "RNDC command failed"),
        (status = 500, description = "Internal server error")
    ),
    tag = "zones"
)]
pub async fn create_zone(
    State(state): State<AppState>,
    Json(request): Json<CreateZoneRequest>,
) -> Result<(StatusCode, Json<ZoneResponse>), ApiError> {
    info!("Creating zone: {}", request.zone_name);

    // Validate zone name
    if request.zone_name.is_empty() {
        metrics::record_zone_operation("create", false);
        return Err(ApiError::InvalidRequest(
            "Zone name cannot be empty".to_string(),
        ));
    }

    // Validate zone type
    if request.zone_type != "master" && request.zone_type != "slave" {
        metrics::record_zone_operation("create", false);
        return Err(ApiError::InvalidRequest(format!(
            "Invalid zone type: {}. Must be 'master' or 'slave'",
            request.zone_type
        )));
    }

    // Generate zone file content from structured configuration
    let zone_content = request.zone_config.to_zone_file();

    info!(
        "Generated zone file content for {}: {} bytes",
        request.zone_name,
        zone_content.len()
    );

    // Create zone file path
    let zone_file_name = format!("{}.zone", request.zone_name);
    let zone_file_path = PathBuf::from(&state.zone_dir).join(&zone_file_name);

    // Write zone file
    tokio::fs::write(&zone_file_path, &zone_content)
        .await
        .map_err(|e| {
            error!(
                "Failed to write zone file {}: {}",
                zone_file_path.display(),
                e
            );
            metrics::record_zone_operation("create", false);
            ApiError::ZoneFileError(format!("Failed to write zone file: {}", e))
        })?;

    info!("Wrote zone file: {}", zone_file_path.display());

    // Build zone configuration for rndc addzone
    let zone_file_full_path = format!("{}/{}", state.zone_dir, zone_file_name);
    let zone_config = if let Some(key_name) = &request.update_key_name {
        format!(
            r#"{{ type {}; file "{}"; allow-update {{ key "{}"; }}; }};"#,
            request.zone_type, zone_file_full_path, key_name
        )
    } else {
        format!(
            r#"{{ type {}; file "{}"; }};"#,
            request.zone_type, zone_file_full_path
        )
    };

    // Execute rndc addzone
    let output = state
        .rndc
        .addzone(&request.zone_name, &zone_config)
        .await
        .map_err(|e| {
            error!("RNDC addzone failed for {}: {}", request.zone_name, e);
            metrics::record_zone_operation("create", false);
            ApiError::RndcError(format!("Failed to add zone: {}", e))
        })?;

    info!("Zone {} created successfully", request.zone_name);
    metrics::record_zone_operation("create", true);

    Ok((
        StatusCode::CREATED,
        Json(ZoneResponse {
            success: true,
            message: format!("Zone {} created successfully", request.zone_name),
            details: Some(output),
        }),
    ))
}

/// Delete a zone
#[utoipa::path(
    delete,
    path = "/api/v1/zones/{name}",
    params(
        ("name" = String, Path, description = "Zone name to delete")
    ),
    responses(
        (status = 200, description = "Zone deleted successfully", body = ZoneResponse),
        (status = 502, description = "RNDC command failed")
    ),
    tag = "zones"
)]
pub async fn delete_zone(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
) -> Result<Json<ZoneResponse>, ApiError> {
    info!("Deleting zone: {}", zone_name);

    // Execute rndc delzone
    let output = state.rndc.delzone(&zone_name).await.map_err(|e| {
        error!("RNDC delzone failed for {}: {}", zone_name, e);
        metrics::record_zone_operation("delete", false);
        ApiError::RndcError(format!("Failed to delete zone: {}", e))
    })?;

    // Optionally delete zone file
    let zone_file_name = format!("{}.zone", zone_name);
    let zone_file_path = PathBuf::from(&state.zone_dir).join(&zone_file_name);

    if zone_file_path.exists() {
        if let Err(e) = tokio::fs::remove_file(&zone_file_path).await {
            error!(
                "Failed to delete zone file {}: {}",
                zone_file_path.display(),
                e
            );
            // Don't fail the request if file deletion fails - zone is already removed from BIND9
        } else {
            info!("Deleted zone file: {}", zone_file_path.display());
        }
    }

    info!("Zone {} deleted successfully", zone_name);
    metrics::record_zone_operation("delete", true);

    Ok(Json(ZoneResponse {
        success: true,
        message: format!("Zone {} deleted successfully", zone_name),
        details: Some(output),
    }))
}

/// Reload a zone
#[utoipa::path(
    post,
    path = "/api/v1/zones/{name}/reload",
    params(
        ("name" = String, Path, description = "Zone name to reload")
    ),
    responses(
        (status = 200, description = "Zone reloaded successfully", body = ZoneResponse),
        (status = 502, description = "RNDC command failed")
    ),
    tag = "zones"
)]
pub async fn reload_zone(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
) -> Result<Json<ZoneResponse>, ApiError> {
    info!("Reloading zone: {}", zone_name);

    let output = state.rndc.reload(&zone_name).await.map_err(|e| {
        error!("RNDC reload failed for {}: {}", zone_name, e);
        metrics::record_zone_operation("reload", false);
        ApiError::RndcError(format!("Failed to reload zone: {}", e))
    })?;

    info!("Zone {} reloaded successfully", zone_name);
    metrics::record_zone_operation("reload", true);

    Ok(Json(ZoneResponse {
        success: true,
        message: format!("Zone {} reloaded successfully", zone_name),
        details: Some(output),
    }))
}

/// Get zone status
#[utoipa::path(
    get,
    path = "/api/v1/zones/{name}/status",
    params(
        ("name" = String, Path, description = "Zone name")
    ),
    responses(
        (status = 200, description = "Zone status retrieved", body = ZoneResponse),
        (status = 404, description = "Zone not found"),
        (status = 502, description = "RNDC command failed")
    ),
    tag = "zones"
)]
pub async fn zone_status(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
) -> Result<Json<ZoneResponse>, ApiError> {
    info!("Getting status for zone: {}", zone_name);

    let output = state.rndc.zonestatus(&zone_name).await.map_err(|e| {
        error!("RNDC zonestatus failed for {}: {}", zone_name, e);
        if e.to_string().contains("not found") {
            ApiError::ZoneNotFound(zone_name.clone())
        } else {
            ApiError::RndcError(format!("Failed to get zone status: {}", e))
        }
    })?;

    Ok(Json(ZoneResponse {
        success: true,
        message: format!("Zone {} status retrieved", zone_name),
        details: Some(output),
    }))
}

/// Freeze a zone (disable dynamic updates)
#[utoipa::path(
    post,
    path = "/api/v1/zones/{name}/freeze",
    params(
        ("name" = String, Path, description = "Zone name to freeze")
    ),
    responses(
        (status = 200, description = "Zone frozen successfully", body = ZoneResponse),
        (status = 502, description = "RNDC command failed")
    ),
    tag = "zones"
)]
pub async fn freeze_zone(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
) -> Result<Json<ZoneResponse>, ApiError> {
    info!("Freezing zone: {}", zone_name);

    let output = state.rndc.freeze(&zone_name).await.map_err(|e| {
        error!("RNDC freeze failed for {}: {}", zone_name, e);
        metrics::record_zone_operation("freeze", false);
        ApiError::RndcError(format!("Failed to freeze zone: {}", e))
    })?;

    info!("Zone {} frozen successfully", zone_name);
    metrics::record_zone_operation("freeze", true);

    Ok(Json(ZoneResponse {
        success: true,
        message: format!("Zone {} frozen successfully", zone_name),
        details: Some(output),
    }))
}

/// Thaw a zone (enable dynamic updates)
#[utoipa::path(
    post,
    path = "/api/v1/zones/{name}/thaw",
    params(
        ("name" = String, Path, description = "Zone name to thaw")
    ),
    responses(
        (status = 200, description = "Zone thawed successfully", body = ZoneResponse),
        (status = 502, description = "RNDC command failed")
    ),
    tag = "zones"
)]
pub async fn thaw_zone(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
) -> Result<Json<ZoneResponse>, ApiError> {
    info!("Thawing zone: {}", zone_name);

    let output = state.rndc.thaw(&zone_name).await.map_err(|e| {
        error!("RNDC thaw failed for {}: {}", zone_name, e);
        metrics::record_zone_operation("thaw", false);
        ApiError::RndcError(format!("Failed to thaw zone: {}", e))
    })?;

    info!("Zone {} thawed successfully", zone_name);
    metrics::record_zone_operation("thaw", true);

    Ok(Json(ZoneResponse {
        success: true,
        message: format!("Zone {} thawed successfully", zone_name),
        details: Some(output),
    }))
}

/// Notify secondaries about zone changes
#[utoipa::path(
    post,
    path = "/api/v1/zones/{name}/notify",
    params(
        ("name" = String, Path, description = "Zone name")
    ),
    responses(
        (status = 200, description = "Notify sent successfully", body = ZoneResponse),
        (status = 502, description = "RNDC command failed")
    ),
    tag = "zones"
)]
pub async fn notify_zone(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
) -> Result<Json<ZoneResponse>, ApiError> {
    info!("Notifying secondaries for zone: {}", zone_name);

    let output = state.rndc.notify(&zone_name).await.map_err(|e| {
        error!("RNDC notify failed for {}: {}", zone_name, e);
        metrics::record_zone_operation("notify", false);
        ApiError::RndcError(format!("Failed to notify zone: {}", e))
    })?;

    info!("Zone {} notify sent successfully", zone_name);
    metrics::record_zone_operation("notify", true);

    Ok(Json(ZoneResponse {
        success: true,
        message: format!("Notify sent for zone {}", zone_name),
        details: Some(output),
    }))
}

/// Get server status
#[utoipa::path(
    get,
    path = "/api/v1/server/status",
    responses(
        (status = 200, description = "Server status retrieved", body = ServerStatusResponse),
        (status = 502, description = "RNDC command failed")
    ),
    tag = "server"
)]
pub async fn server_status(
    State(state): State<AppState>,
) -> Result<Json<ServerStatusResponse>, ApiError> {
    info!("Getting server status");

    let output = state.rndc.status().await.map_err(|e| {
        error!("RNDC status failed: {}", e);
        ApiError::RndcError(format!("Failed to get server status: {}", e))
    })?;

    Ok(Json(ServerStatusResponse { status: output }))
}

/// List all zones
#[utoipa::path(
    get,
    path = "/api/v1/zones",
    responses(
        (status = 200, description = "List of zones", body = ZoneListResponse),
        (status = 500, description = "Failed to read zone directory")
    ),
    tag = "zones"
)]
pub async fn list_zones(State(state): State<AppState>) -> Result<Json<ZoneListResponse>, ApiError> {
    info!("Listing all zones");

    // Get zone files from directory
    let mut zones = Vec::new();
    let mut entries = tokio::fs::read_dir(&state.zone_dir).await.map_err(|e| {
        error!("Failed to read zone directory: {}", e);
        ApiError::InternalError(format!("Failed to read zone directory: {}", e))
    })?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Ok(file_name) = entry.file_name().into_string() {
            if file_name.ends_with(".zone") {
                // Extract zone name from filename (remove .zone extension)
                if let Some(zone_name) = file_name.strip_suffix(".zone") {
                    zones.push(zone_name.to_string());
                }
            }
        }
    }

    zones.sort();
    let count = zones.len();

    info!("Found {} zones", count);
    metrics::update_zones_count(count as i64);

    Ok(Json(ZoneListResponse { zones, count }))
}

/// Get a specific zone
#[utoipa::path(
    get,
    path = "/api/v1/zones/{name}",
    params(
        ("name" = String, Path, description = "Zone name")
    ),
    responses(
        (status = 200, description = "Zone information", body = ZoneInfo),
        (status = 404, description = "Zone not found"),
        (status = 502, description = "RNDC command failed")
    ),
    tag = "zones"
)]
pub async fn get_zone(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
) -> Result<Json<ZoneInfo>, ApiError> {
    info!("Getting zone: {}", zone_name);

    // Check if zone file exists
    let zone_file_name = format!("{}.zone", zone_name);
    let zone_file_path = PathBuf::from(&state.zone_dir).join(&zone_file_name);

    if !zone_file_path.exists() {
        return Err(ApiError::ZoneNotFound(zone_name.clone()));
    }

    // Get zone status from BIND9
    let status_output = state.rndc.zonestatus(&zone_name).await.map_err(|e| {
        error!("RNDC zonestatus failed for {}: {}", zone_name, e);
        if e.to_string().contains("not found") {
            ApiError::ZoneNotFound(zone_name.clone())
        } else {
            ApiError::RndcError(format!("Failed to get zone status: {}", e))
        }
    })?;

    // Parse zone type and serial from status output
    let mut zone_type = "unknown".to_string();
    let mut serial = None;

    for line in status_output.lines() {
        if line.contains("type:") {
            if let Some(type_str) = line.split("type:").nth(1) {
                zone_type = type_str.trim().to_string();
            }
        }
        if line.contains("serial:") {
            if let Some(serial_str) = line.split("serial:").nth(1) {
                if let Ok(s) = serial_str.trim().parse::<u32>() {
                    serial = Some(s);
                }
            }
        }
    }

    Ok(Json(ZoneInfo {
        name: zone_name,
        zone_type,
        serial,
        file_path: Some(zone_file_path.display().to_string()),
    }))
}
