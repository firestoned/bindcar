// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! RNDC data types for parsing BIND9 output
//!
//! This module defines the core data structures used for parsing
//! RNDC command outputs (showzone, zonestatus, status).

use std::net::IpAddr;

/// DNS class
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DnsClass {
    #[default]
    IN,  // Internet
    CH,  // Chaos
    HS,  // Hesiod
}

impl DnsClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            DnsClass::IN => "IN",
            DnsClass::CH => "CH",
            DnsClass::HS => "HS",
        }
    }
}

/// Zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneType {
    Primary,
    Secondary,
    Stub,
    Forward,
    Hint,
    Mirror,
    Delegation,
    Redirect,
}

impl ZoneType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ZoneType::Primary => "primary",
            ZoneType::Secondary => "secondary",
            ZoneType::Stub => "stub",
            ZoneType::Forward => "forward",
            ZoneType::Hint => "hint",
            ZoneType::Mirror => "mirror",
            ZoneType::Delegation => "delegation-only",
            ZoneType::Redirect => "redirect",
        }
    }

    /// Parse from string, accepting both new and old terminology
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "primary" | "master" => Some(ZoneType::Primary),
            "secondary" | "slave" => Some(ZoneType::Secondary),
            "stub" => Some(ZoneType::Stub),
            "forward" => Some(ZoneType::Forward),
            "hint" => Some(ZoneType::Hint),
            "mirror" => Some(ZoneType::Mirror),
            "delegation-only" => Some(ZoneType::Delegation),
            "redirect" => Some(ZoneType::Redirect),
            _ => None,
        }
    }
}

/// Primary server specification for secondary zones
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimarySpec {
    pub address: IpAddr,
    pub port: Option<u16>,
}

impl PrimarySpec {
    pub fn new(address: IpAddr) -> Self {
        Self {
            address,
            port: None,
        }
    }

    pub fn with_port(address: IpAddr, port: u16) -> Self {
        Self {
            address,
            port: Some(port),
        }
    }
}

/// Zone configuration from `rndc showzone`
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneConfig {
    pub zone_name: String,
    pub class: DnsClass,
    pub zone_type: ZoneType,
    pub file: Option<String>,
    pub primaries: Option<Vec<PrimarySpec>>,
    pub also_notify: Option<Vec<IpAddr>>,
    pub allow_transfer: Option<Vec<IpAddr>>,
    pub allow_update: Option<Vec<IpAddr>>,
    /// Raw allow-update directive (e.g., "{ key \"name\"; }")
    /// Used to preserve key-based allow-update when no IPs are specified
    pub allow_update_raw: Option<String>,
}

impl ZoneConfig {
    /// Create a new zone configuration
    pub fn new(zone_name: String, zone_type: ZoneType) -> Self {
        Self {
            zone_name,
            class: DnsClass::IN,
            zone_type,
            file: None,
            primaries: None,
            also_notify: None,
            allow_transfer: None,
            allow_update: None,
            allow_update_raw: None,
        }
    }

    /// Serialize to RNDC-compatible zone config block
    ///
    /// Returns the configuration in the format expected by `rndc modzone`
    /// and `rndc addzone`, e.g., `{ type primary; file "..."; ... };`
    pub fn to_rndc_block(&self) -> String {
        let mut parts = Vec::new();

        // Type is always required
        parts.push(format!("type {}", self.zone_type.as_str()));

        // File (required for primary zones)
        if let Some(ref file) = self.file {
            parts.push(format!(r#"file "{}""#, file));
        }

        // Primaries (for secondary zones)
        if let Some(ref primaries) = self.primaries {
            if !primaries.is_empty() {
                let primary_list = primaries
                    .iter()
                    .map(|p| {
                        if let Some(port) = p.port {
                            format!("{} port {}", p.address, port)
                        } else {
                            p.address.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                parts.push(format!("primaries {{ {}; }}", primary_list));
            }
        }

        // Also-notify
        if let Some(ref also_notify) = self.also_notify {
            if !also_notify.is_empty() {
                let notify_list = also_notify
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                parts.push(format!("also-notify {{ {}; }}", notify_list));
            }
        }

        // Allow-transfer
        if let Some(ref allow_transfer) = self.allow_transfer {
            if !allow_transfer.is_empty() {
                let transfer_list = allow_transfer
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                parts.push(format!("allow-transfer {{ {}; }}", transfer_list));
            }
        }

        // Allow-update (prefer raw directive if present, otherwise use IP list)
        if let Some(ref raw) = self.allow_update_raw {
            // Raw directive includes the full "{ ... };" but we only want "{ ... }"
            // Strip all trailing semicolons and whitespace
            let raw_trimmed = raw.trim_end().trim_end_matches(';').trim();
            parts.push(format!("allow-update {}", raw_trimmed));
        } else if let Some(ref allow_update) = self.allow_update {
            if !allow_update.is_empty() {
                let update_list = allow_update
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                parts.push(format!("allow-update {{ {}; }}", update_list));
            }
        }

        format!("{{ {}; }};", parts.join("; "))
    }
}
