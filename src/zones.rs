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
use tracing::{debug, error, info};
use utoipa::ToSchema;

use crate::{
    metrics,
    types::{ApiError, AppState},
};

/// Zone type constants
pub const ZONE_TYPE_PRIMARY: &str = "primary";
pub const ZONE_TYPE_SECONDARY: &str = "secondary";

/// SOA (Start of Authority) record configuration
///
/// # Default Values
///
/// - `serial`: Automatically generated in YYYYMMDD01 format (e.g., 2025120601) if not provided
/// - `refresh`: 3600 seconds
/// - `retry`: 600 seconds
/// - `expire`: 604800 seconds (7 days)
/// - `negative_ttl`: 86400 seconds (1 day)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SoaRecord {
    /// Primary nameserver (e.g., "ns1.example.com.")
    pub primary_ns: String,

    /// Admin email (e.g., "admin.example.com.")
    pub admin_email: String,

    /// Serial number (e.g., 2025120601) - defaults to current date in YYYYMMDD01 format
    #[serde(default = "default_serial")]
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

fn default_serial() -> u32 {
    // Generate serial as YYYYMMDD01
    let now = chrono::Utc::now();
    let date_part = now.format("%Y%m%d").to_string();
    format!("{}01", date_part).parse().unwrap_or(2025120601)
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

    /// A records for nameservers (glue records)
    /// Maps nameserver hostname to IP address (e.g., "ns1.example.com." -> "192.0.2.1")
    pub name_server_ips: std::collections::HashMap<String, String>,

    /// DNS records in the zone
    #[serde(default)]
    pub records: Vec<DnsRecord>,

    /// IP addresses of secondary servers to notify when zone changes (BIND9 also-notify)
    /// Example: ["10.244.2.101", "10.244.2.102"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub also_notify: Option<Vec<String>>,

    /// IP addresses allowed to transfer the zone (BIND9 allow-transfer)
    /// Example: ["10.244.2.101", "10.244.2.102"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_transfer: Option<Vec<String>>,

    /// IP addresses of primary servers for secondary zones (BIND9 primaries/masters)
    /// Example: ["192.0.2.1", "192.0.2.2"]
    /// Required for secondary zone types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primaries: Option<Vec<String>>,
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

        // Glue records (A records for nameservers)
        for (ns_name, ip) in &self.name_server_ips {
            // Use FQDN with trailing dot to prevent BIND9 from appending zone name
            zone_file.push_str(&format!("{} IN A {}\n", ns_name, ip));
        }
        if !self.name_server_ips.is_empty() {
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

    /// Zone type ("primary" or "secondary")
    pub zone_type: String,

    /// Structured zone configuration
    pub zone_config: ZoneConfig,

    /// Optional: TSIG key name for allow-update
    pub update_key_name: Option<String>,
}

/// Request to modify a zone configuration
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModifyZoneRequest {
    /// IP addresses of secondary servers to notify when zone changes (BIND9 also-notify)
    /// Example: ["10.244.2.101", "10.244.2.102"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub also_notify: Option<Vec<String>>,

    /// IP addresses allowed to transfer the zone (BIND9 allow-transfer)
    /// Example: ["10.244.2.101", "10.244.2.102"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_transfer: Option<Vec<String>>,

    /// IP addresses allowed to update the zone dynamically (BIND9 allow-update)
    /// Example: ["10.244.2.101", "10.244.2.102"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_update: Option<Vec<String>>,
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
        (status = 409, description = "Zone already exists"),
        (status = 500, description = "RNDC command failed"),
        (status = 500, description = "Internal server error")
    ),
    tag = "zones"
)]
pub async fn create_zone(
    State(state): State<AppState>,
    Json(request): Json<CreateZoneRequest>,
) -> Result<(StatusCode, Json<ZoneResponse>), ApiError> {
    info!("Creating zone: {}", request.zone_name);

    // Debug log the full request payload
    if let Ok(json_payload) = serde_json::to_string_pretty(&request) {
        debug!("POST /api/v1/zones payload: {}", json_payload);
    }

    // Validate zone name
    if request.zone_name.is_empty() {
        metrics::record_zone_operation("create", false);
        return Err(ApiError::InvalidRequest(
            "Zone name cannot be empty".to_string(),
        ));
    }

    // Validate zone type
    if request.zone_type != ZONE_TYPE_PRIMARY && request.zone_type != ZONE_TYPE_SECONDARY {
        metrics::record_zone_operation("create", false);
        return Err(ApiError::InvalidRequest(format!(
            "Invalid zone type: {}. Must be '{}' or '{}'",
            request.zone_type, ZONE_TYPE_PRIMARY, ZONE_TYPE_SECONDARY
        )));
    }

    // Validate secondary zone requirements
    if request.zone_type == ZONE_TYPE_SECONDARY
        && request
            .zone_config
            .primaries
            .as_ref()
            .map_or(true, |p| p.is_empty())
    {
        metrics::record_zone_operation("create", false);
        return Err(ApiError::InvalidRequest(
            "Secondary zones require at least one primary server in 'primaries' field".to_string(),
        ));
    }

    // Generate zone file content from structured configuration (only for primary zones)
    let zone_content = if request.zone_type == ZONE_TYPE_PRIMARY {
        request.zone_config.to_zone_file()
    } else {
        String::new() // Secondary zones don't need zone files
    };

    // Only write zone file for primary zones
    let zone_file_name = format!("{}.zone", request.zone_name);
    let zone_file_path = PathBuf::from(&state.zone_dir).join(&zone_file_name);

    if request.zone_type == ZONE_TYPE_PRIMARY {
        info!(
            "Generated zone file content for {}: {} bytes",
            request.zone_name,
            zone_content.len()
        );

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
    }

    // Build zone configuration for rndc addzone
    let mut config_parts = vec![format!(r#"type {}"#, request.zone_type)];

    // Add file path for primary zones
    if request.zone_type == ZONE_TYPE_PRIMARY {
        let zone_file_full_path = format!("{}/{}", state.zone_dir, zone_file_name);
        config_parts.push(format!(r#"file "{}""#, zone_file_full_path));
    }

    // Add primaries for secondary zones
    if request.zone_type == ZONE_TYPE_SECONDARY {
        if let Some(primaries) = &request.zone_config.primaries {
            if !primaries.is_empty() {
                let primaries_list = primaries
                    .iter()
                    .map(|ip| format!("{}; ", ip))
                    .collect::<String>();
                config_parts.push(format!(r#"primaries {{ {} }}"#, primaries_list));
            }
        }
    }

    // Add allow-update if TSIG key is provided
    if let Some(key_name) = &request.update_key_name {
        config_parts.push(format!(r#"allow-update {{ key "{}"; }}"#, key_name));
    }

    // Add also-notify if secondary IPs are provided
    if let Some(also_notify) = &request.zone_config.also_notify {
        if !also_notify.is_empty() {
            let notify_list = also_notify
                .iter()
                .map(|ip| format!("{}; ", ip))
                .collect::<String>();
            config_parts.push(format!(r#"also-notify {{ {} }}"#, notify_list));
        }
    }

    // Add allow-transfer if secondary IPs are provided
    if let Some(allow_transfer) = &request.zone_config.allow_transfer {
        if !allow_transfer.is_empty() {
            let transfer_list = allow_transfer
                .iter()
                .map(|ip| format!("{}; ", ip))
                .collect::<String>();
            config_parts.push(format!(r#"allow-transfer {{ {} }}"#, transfer_list));
        }
    }

    // Join all parts into final configuration
    let zone_config = format!("{{ {}; }};", config_parts.join("; "));

    // Execute rndc addzone
    let output = state
        .rndc
        .addzone(&request.zone_name, &zone_config)
        .await
        .map_err(|e| {
            error!("RNDC addzone failed for {}: {}", request.zone_name, e);
            metrics::record_zone_operation("create", false);

            // Check if zone already exists
            let error_msg = e.to_string();
            if error_msg.contains("already exists") {
                ApiError::ZoneAlreadyExists(request.zone_name.clone())
            } else {
                ApiError::RndcError(error_msg)
            }
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
        (status = 500, description = "RNDC command failed")
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
        ApiError::RndcError(e.to_string())
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
        (status = 500, description = "RNDC command failed")
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
        ApiError::RndcError(e.to_string())
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
        (status = 500, description = "RNDC command failed")
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
            ApiError::RndcError(e.to_string())
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
        (status = 500, description = "RNDC command failed")
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
        ApiError::RndcError(e.to_string())
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
        (status = 500, description = "RNDC command failed")
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
        ApiError::RndcError(e.to_string())
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
        (status = 500, description = "RNDC command failed")
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
        ApiError::RndcError(e.to_string())
    })?;

    info!("Zone {} notify sent successfully", zone_name);
    metrics::record_zone_operation("notify", true);

    Ok(Json(ZoneResponse {
        success: true,
        message: format!("Notify sent for zone {}", zone_name),
        details: Some(output),
    }))
}

/// Force a zone retransfer from primary
#[utoipa::path(
    post,
    path = "/api/v1/zones/{name}/retransfer",
    params(
        ("name" = String, Path, description = "Zone name to retransfer")
    ),
    responses(
        (status = 200, description = "Zone retransfer initiated", body = ZoneResponse),
        (status = 500, description = "RNDC command failed")
    ),
    tag = "zones"
)]
pub async fn retransfer_zone(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
) -> Result<Json<ZoneResponse>, ApiError> {
    info!("Retransferring zone: {}", zone_name);

    let output = state.rndc.retransfer(&zone_name).await.map_err(|e| {
        error!("RNDC retransfer failed for {}: {}", zone_name, e);
        metrics::record_zone_operation("retransfer", false);
        ApiError::RndcError(e.to_string())
    })?;

    info!("Zone {} retransfer initiated successfully", zone_name);
    metrics::record_zone_operation("retransfer", true);

    Ok(Json(ZoneResponse {
        success: true,
        message: format!("Retransfer initiated for zone {}", zone_name),
        details: Some(output),
    }))
}

/// Get server status
#[utoipa::path(
    get,
    path = "/api/v1/server/status",
    responses(
        (status = 200, description = "Server status retrieved", body = ServerStatusResponse),
        (status = 500, description = "RNDC command failed")
    ),
    tag = "server"
)]
pub async fn server_status(
    State(state): State<AppState>,
) -> Result<Json<ServerStatusResponse>, ApiError> {
    info!("Getting server status");

    let output = state.rndc.status().await.map_err(|e| {
        error!("RNDC status failed: {}", e);
        ApiError::RndcError(e.to_string())
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
        (status = 500, description = "RNDC command failed")
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
            ApiError::RndcError(e.to_string())
        }
    })?;

    // Parse zone type and serial from status output
    let mut zone_type = "unknown".to_string();
    let mut serial = None;

    for line in status_output.lines() {
        if let Some(type_str) = line.strip_prefix("type:").or_else(|| {
            line.contains("type:")
                .then(|| line.split("type:").nth(1))
                .flatten()
        }) {
            zone_type = type_str.trim().to_string();
        }

        if let Some(serial_str) = line.strip_prefix("serial:").or_else(|| {
            line.contains("serial:")
                .then(|| line.split("serial:").nth(1))
                .flatten()
        }) {
            if let Ok(s) = serial_str.trim().parse::<u32>() {
                serial = Some(s);
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

/// Modify a zone configuration
///
/// This endpoint allows updating zone configuration parameters such as
/// also-notify and allow-transfer IP addresses without recreating the zone.
/// It uses the `rndc modzone` command to dynamically update the zone configuration.
#[utoipa::path(
    patch,
    path = "/api/v1/zones/{name}",
    request_body = ModifyZoneRequest,
    params(
        ("name" = String, Path, description = "Zone name to modify")
    ),
    responses(
        (status = 200, description = "Zone modified successfully", body = ZoneResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Zone not found"),
        (status = 500, description = "RNDC command failed"),
        (status = 500, description = "Internal server error")
    ),
    tag = "zones"
)]
pub async fn modify_zone(
    State(state): State<AppState>,
    Path(zone_name): Path<String>,
    Json(request): Json<ModifyZoneRequest>,
) -> Result<Json<ZoneResponse>, ApiError> {
    info!("Modifying zone: {}", zone_name);

    // Debug log the full request payload
    if let Ok(json_payload) = serde_json::to_string_pretty(&request) {
        debug!("PATCH /api/v1/zones/{} payload: {}", zone_name, json_payload);
    }

    // Validate that at least one field is being updated
    if request.also_notify.is_none() && request.allow_transfer.is_none() && request.allow_update.is_none() {
        metrics::record_zone_operation("modify", false);
        return Err(ApiError::InvalidRequest(
            "At least one field (alsoNotify, allowTransfer, or allowUpdate) must be provided".to_string(),
        ));
    }

    // Check if zone exists by checking for zone file or querying status
    let zone_file_name = format!("{}.zone", zone_name);
    let zone_file_path = PathBuf::from(&state.zone_dir).join(&zone_file_name);

    // For secondary zones, there may not be a zone file, so we should check status instead
    let zone_exists = if zone_file_path.exists() {
        true
    } else {
        // Try to get zone status to see if it exists
        match state.rndc.zonestatus(&zone_name).await {
            Ok(_) => true,
            Err(e) => {
                if e.to_string().contains("not found") {
                    false
                } else {
                    // Some other error, but zone might exist
                    true
                }
            }
        }
    };

    if !zone_exists {
        metrics::record_zone_operation("modify", false);
        return Err(ApiError::ZoneNotFound(zone_name.clone()));
    }

    // Get current zone configuration from BIND9
    let showzone_output = state.rndc.showzone(&zone_name).await.map_err(|e| {
        error!("Failed to get zone configuration for {}: {}", zone_name, e);
        if e.to_string().contains("not found") {
            ApiError::ZoneNotFound(zone_name.clone())
        } else {
            ApiError::RndcError(e.to_string())
        }
    })?;

    // Parse the zone configuration
    let mut zone_config = crate::rndc_parser::parse_showzone(&showzone_output).map_err(|e| {
        error!("Failed to parse zone configuration for {}: {}", zone_name, e);
        ApiError::RndcError(format!("Failed to parse zone configuration: {}", e))
    })?;

    info!("Zone {} has type: {}", zone_name, zone_config.zone_type.as_str());

    // Update the configuration with new values from the request
    if let Some(also_notify) = &request.also_notify {
        // Convert string IPs to IpAddr
        let ip_addrs: Result<Vec<std::net::IpAddr>, _> = also_notify
            .iter()
            .map(|s| s.parse())
            .collect();

        match ip_addrs {
            Ok(addrs) => {
                zone_config.also_notify = if addrs.is_empty() {
                    None
                } else {
                    Some(addrs)
                };
            }
            Err(e) => {
                metrics::record_zone_operation("modify", false);
                return Err(ApiError::InvalidRequest(format!("Invalid IP address in also-notify: {}", e)));
            }
        }
    }

    if let Some(allow_transfer) = &request.allow_transfer {
        // Convert string IPs to IpAddr
        let ip_addrs: Result<Vec<std::net::IpAddr>, _> = allow_transfer
            .iter()
            .map(|s| s.parse())
            .collect();

        match ip_addrs {
            Ok(addrs) => {
                zone_config.allow_transfer = if addrs.is_empty() {
                    None
                } else {
                    Some(addrs)
                };
            }
            Err(e) => {
                metrics::record_zone_operation("modify", false);
                return Err(ApiError::InvalidRequest(format!("Invalid IP address in allow-transfer: {}", e)));
            }
        }
    }

    if let Some(allow_update) = &request.allow_update {
        // Convert string IPs to IpAddr
        let ip_addrs: Result<Vec<std::net::IpAddr>, _> = allow_update
            .iter()
            .map(|s| s.parse())
            .collect();

        match ip_addrs {
            Ok(addrs) => {
                zone_config.allow_update = if addrs.is_empty() {
                    None
                } else {
                    Some(addrs)
                };
                // Clear the raw directive since we're setting explicit IPs
                zone_config.allow_update_raw = None;
            }
            Err(e) => {
                metrics::record_zone_operation("modify", false);
                return Err(ApiError::InvalidRequest(format!("Invalid IP address in allow-update: {}", e)));
            }
        }
    }

    // Serialize the updated configuration back to RNDC format
    let rndc_config_block = zone_config.to_rndc_block();

    info!("Modifying zone {} with config: {}", zone_name, rndc_config_block);

    // Execute rndc modzone
    let output = state
        .rndc
        .modzone(&zone_name, &rndc_config_block)
        .await
        .map_err(|e| {
            error!("RNDC modzone failed for {}: {}", zone_name, e);
            metrics::record_zone_operation("modify", false);
            ApiError::RndcError(e.to_string())
        })?;

    info!("Zone {} modified successfully", zone_name);
    metrics::record_zone_operation("modify", true);

    Ok(Json(ZoneResponse {
        success: true,
        message: format!("Zone {} modified successfully", zone_name),
        details: Some(output),
    }))
}
