// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Zone data types — the pure request/response and configuration structs.
//!
//! These are the types a library consumer needs in order to talk to bindcar's
//! zone API, with no HTTP machinery attached. They are deliberately free of
//! `axum`, `ApiError` and `AppState` so that a consumer importing only
//! `bindcar::ZoneConfig` does not inherit the server stack.
//!
//! The HTTP handlers that consume these live in [`crate::zones`], which
//! re-exports everything here so existing `bindcar::zones::ZoneConfig` paths
//! keep working.
//!
//! See `.github/community/02-feature-gate-http-server.md` — this split is
//! phase 1 of putting the server behind a cargo feature.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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

    /// DNSSEC policy name to apply to this zone (BIND9 9.16+)
    ///
    /// Specifies the name of a `dnssec-policy` block defined in `named.conf.options`.
    /// When set, BIND9 will automatically sign the zone using the specified policy.
    ///
    /// Example: `"default"`, `"high-security"`
    ///
    /// Requires `inline_signing` to be enabled for DNSSEC to function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dnssec_policy: Option<String>,

    /// Enable inline signing for DNSSEC (BIND9 inline-signing)
    ///
    /// When `true`, BIND9 will sign the zone inline rather than requiring pre-signed zone files.
    /// This is required for DNSSEC with dynamic zones and modern BIND9 configurations.
    ///
    /// Should be set to `true` when `dnssec_policy` is specified.
    ///
    /// Default: `false`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_signing: Option<bool>,
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
