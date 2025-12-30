// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! RNDC data types for parsing BIND9 output
//!
//! This module defines the core data structures used for parsing
//! RNDC command outputs (showzone, zonestatus, status).

use std::collections::HashMap;
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

/// Forwarder specification for forward zones
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwarderSpec {
    pub address: IpAddr,
    pub port: Option<u16>,
    pub tls_config: Option<String>,
}

impl ForwarderSpec {
    pub fn new(address: IpAddr) -> Self {
        Self {
            address,
            port: None,
            tls_config: None,
        }
    }

    pub fn with_port(address: IpAddr, port: u16) -> Self {
        Self {
            address,
            port: Some(port),
            tls_config: None,
        }
    }

    pub fn with_tls(address: IpAddr, tls_config: String) -> Self {
        Self {
            address,
            port: None,
            tls_config: Some(tls_config),
        }
    }
}

/// NOTIFY mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyMode {
    Yes,
    No,
    Explicit,
    MasterOnly,
    PrimaryOnly,
}

impl NotifyMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotifyMode::Yes => "yes",
            NotifyMode::No => "no",
            NotifyMode::Explicit => "explicit",
            NotifyMode::MasterOnly => "master-only",
            NotifyMode::PrimaryOnly => "primary-only",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "yes" => Some(NotifyMode::Yes),
            "no" => Some(NotifyMode::No),
            "explicit" => Some(NotifyMode::Explicit),
            "master-only" => Some(NotifyMode::MasterOnly),
            "primary-only" => Some(NotifyMode::PrimaryOnly),
            _ => None,
        }
    }
}

/// Forward mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForwardMode {
    Only,
    First,
}

impl ForwardMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ForwardMode::Only => "only",
            ForwardMode::First => "first",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "only" => Some(ForwardMode::Only),
            "first" => Some(ForwardMode::First),
            _ => None,
        }
    }
}

/// Auto DNSSEC mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoDnssecMode {
    Off,
    Maintain,
    Create,
}

impl AutoDnssecMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AutoDnssecMode::Off => "off",
            AutoDnssecMode::Maintain => "maintain",
            AutoDnssecMode::Create => "create",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "off" => Some(AutoDnssecMode::Off),
            "maintain" => Some(AutoDnssecMode::Maintain),
            "create" => Some(AutoDnssecMode::Create),
            _ => None,
        }
    }
}

/// Check names mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckNamesMode {
    Fail,
    Warn,
    Ignore,
}

impl CheckNamesMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckNamesMode::Fail => "fail",
            CheckNamesMode::Warn => "warn",
            CheckNamesMode::Ignore => "ignore",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "fail" => Some(CheckNamesMode::Fail),
            "warn" => Some(CheckNamesMode::Warn),
            "ignore" => Some(CheckNamesMode::Ignore),
            _ => None,
        }
    }
}

/// Masterfile format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterfileFormat {
    Text,
    Raw,
    Map,
}

impl MasterfileFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            MasterfileFormat::Text => "text",
            MasterfileFormat::Raw => "raw",
            MasterfileFormat::Map => "map",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "text" => Some(MasterfileFormat::Text),
            "raw" => Some(MasterfileFormat::Raw),
            "map" => Some(MasterfileFormat::Map),
            _ => None,
        }
    }
}

/// Zone configuration from `rndc showzone`
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneConfig {
    // Core fields
    pub zone_name: String,
    pub class: DnsClass,
    pub zone_type: ZoneType,
    pub file: Option<String>,

    // Primary/Secondary options
    pub primaries: Option<Vec<PrimarySpec>>,
    pub also_notify: Option<Vec<IpAddr>>,
    pub notify: Option<NotifyMode>,

    // Access Control options
    pub allow_query: Option<Vec<IpAddr>>,
    pub allow_transfer: Option<Vec<IpAddr>>,
    pub allow_update: Option<Vec<IpAddr>>,
    /// Raw allow-update directive (e.g., "{ key \"name\"; }")
    /// Used to preserve key-based allow-update when no IPs are specified
    pub allow_update_raw: Option<String>,
    pub allow_update_forwarding: Option<Vec<IpAddr>>,
    pub allow_notify: Option<Vec<IpAddr>>,

    // Transfer Control options
    pub max_transfer_time_in: Option<u32>,
    pub max_transfer_time_out: Option<u32>,
    pub max_transfer_idle_in: Option<u32>,
    pub max_transfer_idle_out: Option<u32>,
    pub transfer_source: Option<IpAddr>,
    pub transfer_source_v6: Option<IpAddr>,
    pub notify_source: Option<IpAddr>,
    pub notify_source_v6: Option<IpAddr>,

    // Dynamic Update options
    /// Raw update-policy directive (complex grammar, kept as raw)
    pub update_policy: Option<String>,
    pub journal: Option<String>,
    pub ixfr_from_differences: Option<bool>,

    // DNSSEC options
    pub inline_signing: Option<bool>,
    pub auto_dnssec: Option<AutoDnssecMode>,
    pub key_directory: Option<String>,
    pub sig_validity_interval: Option<u32>,
    pub dnskey_sig_validity: Option<u32>,

    // Forwarding options
    pub forward: Option<ForwardMode>,
    pub forwarders: Option<Vec<ForwarderSpec>>,

    // Zone Maintenance options
    pub check_names: Option<CheckNamesMode>,
    pub check_mx: Option<CheckNamesMode>,
    pub check_integrity: Option<bool>,
    pub masterfile_format: Option<MasterfileFormat>,
    pub max_zone_ttl: Option<u32>,

    // Refresh/Retry options
    pub max_refresh_time: Option<u32>,
    pub min_refresh_time: Option<u32>,
    pub max_retry_time: Option<u32>,
    pub min_retry_time: Option<u32>,

    // Miscellaneous options
    pub multi_master: Option<bool>,
    pub request_ixfr: Option<bool>,
    pub request_expire: Option<bool>,

    // Generic catch-all for unrecognized options
    /// Raw options that weren't parsed into structured fields
    /// Key: option name (e.g., "zone-statistics")
    /// Value: raw value as it appears in config (e.g., "yes" or "{ ... }")
    pub raw_options: HashMap<String, String>,
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
            notify: None,
            allow_query: None,
            allow_transfer: None,
            allow_update: None,
            allow_update_raw: None,
            allow_update_forwarding: None,
            allow_notify: None,
            max_transfer_time_in: None,
            max_transfer_time_out: None,
            max_transfer_idle_in: None,
            max_transfer_idle_out: None,
            transfer_source: None,
            transfer_source_v6: None,
            notify_source: None,
            notify_source_v6: None,
            update_policy: None,
            journal: None,
            ixfr_from_differences: None,
            inline_signing: None,
            auto_dnssec: None,
            key_directory: None,
            sig_validity_interval: None,
            dnskey_sig_validity: None,
            forward: None,
            forwarders: None,
            check_names: None,
            check_mx: None,
            check_integrity: None,
            masterfile_format: None,
            max_zone_ttl: None,
            max_refresh_time: None,
            min_refresh_time: None,
            max_retry_time: None,
            min_retry_time: None,
            multi_master: None,
            request_ixfr: None,
            request_expire: None,
            raw_options: HashMap::new(),
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

        // Notify mode
        if let Some(notify) = self.notify {
            parts.push(format!("notify {}", notify.as_str()));
        }

        // Allow-query
        if let Some(ref allow_query) = self.allow_query {
            if !allow_query.is_empty() {
                let query_list = allow_query
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                parts.push(format!("allow-query {{ {}; }}", query_list));
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

        // Allow-update-forwarding
        if let Some(ref allow_update_forwarding) = self.allow_update_forwarding {
            if !allow_update_forwarding.is_empty() {
                let list = allow_update_forwarding
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                parts.push(format!("allow-update-forwarding {{ {}; }}", list));
            }
        }

        // Allow-notify
        if let Some(ref allow_notify) = self.allow_notify {
            if !allow_notify.is_empty() {
                let list = allow_notify
                    .iter()
                    .map(|ip| ip.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                parts.push(format!("allow-notify {{ {}; }}", list));
            }
        }

        // Transfer timeouts
        if let Some(val) = self.max_transfer_time_in {
            parts.push(format!("max-transfer-time-in {}", val));
        }
        if let Some(val) = self.max_transfer_time_out {
            parts.push(format!("max-transfer-time-out {}", val));
        }
        if let Some(val) = self.max_transfer_idle_in {
            parts.push(format!("max-transfer-idle-in {}", val));
        }
        if let Some(val) = self.max_transfer_idle_out {
            parts.push(format!("max-transfer-idle-out {}", val));
        }

        // Transfer sources
        if let Some(ip) = self.transfer_source {
            parts.push(format!("transfer-source {}", ip));
        }
        if let Some(ip) = self.transfer_source_v6 {
            parts.push(format!("transfer-source-v6 {}", ip));
        }
        if let Some(ip) = self.notify_source {
            parts.push(format!("notify-source {}", ip));
        }
        if let Some(ip) = self.notify_source_v6 {
            parts.push(format!("notify-source-v6 {}", ip));
        }

        // Dynamic update options
        if let Some(ref policy) = self.update_policy {
            let policy_trimmed = policy.trim_end().trim_end_matches(';').trim();
            parts.push(format!("update-policy {}", policy_trimmed));
        }
        if let Some(ref journal) = self.journal {
            parts.push(format!(r#"journal "{}""#, journal));
        }
        if let Some(val) = self.ixfr_from_differences {
            parts.push(format!("ixfr-from-differences {}", if val { "yes" } else { "no" }));
        }

        // DNSSEC options
        if let Some(val) = self.inline_signing {
            parts.push(format!("inline-signing {}", if val { "yes" } else { "no" }));
        }
        if let Some(mode) = self.auto_dnssec {
            parts.push(format!("auto-dnssec {}", mode.as_str()));
        }
        if let Some(ref dir) = self.key_directory {
            parts.push(format!(r#"key-directory "{}""#, dir));
        }
        if let Some(val) = self.sig_validity_interval {
            parts.push(format!("sig-validity-interval {}", val));
        }
        if let Some(val) = self.dnskey_sig_validity {
            parts.push(format!("dnskey-sig-validity {}", val));
        }

        // Forwarding options
        if let Some(mode) = self.forward {
            parts.push(format!("forward {}", mode.as_str()));
        }
        if let Some(ref forwarders) = self.forwarders {
            if !forwarders.is_empty() {
                let forwarder_list = forwarders
                    .iter()
                    .map(|f| {
                        if let Some(ref tls) = f.tls_config {
                            format!("{} tls {}", f.address, tls)
                        } else if let Some(port) = f.port {
                            format!("{} port {}", f.address, port)
                        } else {
                            f.address.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                parts.push(format!("forwarders {{ {}; }}", forwarder_list));
            }
        }

        // Zone maintenance options
        if let Some(mode) = self.check_names {
            parts.push(format!("check-names {}", mode.as_str()));
        }
        if let Some(mode) = self.check_mx {
            parts.push(format!("check-mx {}", mode.as_str()));
        }
        if let Some(val) = self.check_integrity {
            parts.push(format!("check-integrity {}", if val { "yes" } else { "no" }));
        }
        if let Some(format) = self.masterfile_format {
            parts.push(format!("masterfile-format {}", format.as_str()));
        }
        if let Some(val) = self.max_zone_ttl {
            parts.push(format!("max-zone-ttl {}", val));
        }

        // Refresh/Retry options
        if let Some(val) = self.max_refresh_time {
            parts.push(format!("max-refresh-time {}", val));
        }
        if let Some(val) = self.min_refresh_time {
            parts.push(format!("min-refresh-time {}", val));
        }
        if let Some(val) = self.max_retry_time {
            parts.push(format!("max-retry-time {}", val));
        }
        if let Some(val) = self.min_retry_time {
            parts.push(format!("min-retry-time {}", val));
        }

        // Miscellaneous options
        if let Some(val) = self.multi_master {
            parts.push(format!("multi-master {}", if val { "yes" } else { "no" }));
        }
        if let Some(val) = self.request_ixfr {
            parts.push(format!("request-ixfr {}", if val { "yes" } else { "no" }));
        }
        if let Some(val) = self.request_expire {
            parts.push(format!("request-expire {}", if val { "yes" } else { "no" }));
        }

        // Raw options (preserve unknown options verbatim)
        for (key, value) in &self.raw_options {
            let value_trimmed = value.trim_end().trim_end_matches(';').trim();
            parts.push(format!("{} {}", key, value_trimmed));
        }

        format!("{{ {}; }};", parts.join("; "))
    }
}
