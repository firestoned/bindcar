// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for zones module

use super::zones::*;
use crate::nsupdate::NsupdateExecutor;
use crate::rndc::RndcExecutor;
use crate::types::{ApiError, AppState};
use axum::extract::{Path, State};
use std::collections::HashMap;
use std::sync::Arc;

/// Build an `AppState` whose executors are constructed offline (no network).
///
/// The RNDC/nsupdate executors are created with a loopback address and a dummy
/// TSIG secret; they never connect unless a command is actually executed. This
/// lets us exercise the handler validation guards, which reject bad input
/// *before* any executor call, without a live BIND9 server.
fn offline_app_state() -> AppState {
    let rndc = RndcExecutor::new(
        "127.0.0.1:953".to_string(),
        "sha256".to_string(),
        "dGVzdC1zZWNyZXQtaGVyZQ==".to_string(),
    )
    .expect("offline rndc executor");
    let nsupdate = NsupdateExecutor::new("127.0.0.1".to_string(), 53, None, None, None)
        .expect("offline nsupdate executor");

    AppState {
        rndc: Arc::new(rndc),
        nsupdate: Arc::new(nsupdate),
        zone_dir: "/tmp".to_string(),
    }
}

/// A zone name that `validate_zone_name` must reject (path traversal).
const MALICIOUS_ZONE_NAME: &str = "../../etc/passwd";

#[test]
fn test_zone_config_to_zone_file() {
    let config = ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604_800,
            negative_ttl: 86400,
        },
        name_servers: vec![
            "ns1.example.com.".to_string(),
            "ns2.example.com.".to_string(),
        ],
        name_server_ips: HashMap::new(),
        records: vec![
            DnsRecord {
                name: "www".to_string(),
                record_type: "A".to_string(),
                value: "192.0.2.1".to_string(),
                ttl: Some(300),
                priority: None,
            },
            DnsRecord {
                name: "@".to_string(),
                record_type: "MX".to_string(),
                value: "mail.example.com.".to_string(),
                ttl: None,
                priority: Some(10),
            },
        ],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: None,
        inline_signing: None,
    };

    let zone_file = config.to_zone_file();

    assert!(zone_file.contains("$TTL 3600"));
    assert!(zone_file.contains("@ IN SOA ns1.example.com. admin.example.com."));
    assert!(zone_file.contains("2025010101"));
    assert!(zone_file.contains("@ IN NS ns1.example.com."));
    assert!(zone_file.contains("@ IN NS ns2.example.com."));
    assert!(zone_file.contains("www 300 IN A 192.0.2.1"));
    assert!(zone_file.contains("@ IN MX 10 mail.example.com."));
}

#[test]
fn test_soa_record_with_defaults() {
    let json = r#"{
        "primaryNs": "ns1.example.com.",
        "adminEmail": "admin.example.com.",
        "serial": 2025010101
    }"#;

    let soa: SoaRecord = serde_json::from_str(json).unwrap();
    assert_eq!(soa.primary_ns, "ns1.example.com.");
    assert_eq!(soa.admin_email, "admin.example.com.");
    assert_eq!(soa.serial, 2025010101);
    assert_eq!(soa.refresh, 3600);
    assert_eq!(soa.retry, 600);
    assert_eq!(soa.expire, 604_800);
    assert_eq!(soa.negative_ttl, 86400);
}

#[test]
fn test_soa_record_default_serial() {
    // Test that serial defaults to YYYYMMDD01 format when not provided
    let json = r#"{
        "primaryNs": "ns1.example.com.",
        "adminEmail": "admin.example.com."
    }"#;

    let soa: SoaRecord = serde_json::from_str(json).unwrap();
    assert_eq!(soa.primary_ns, "ns1.example.com.");
    assert_eq!(soa.admin_email, "admin.example.com.");

    // Serial should be in YYYYMMDD01 format (10 digits, ending in 01)
    let serial_str = soa.serial.to_string();
    assert_eq!(serial_str.len(), 10, "Serial should be 10 digits");
    assert!(serial_str.ends_with("01"), "Serial should end with 01");

    // Verify default values for other fields
    assert_eq!(soa.refresh, 3600);
    assert_eq!(soa.retry, 600);
    assert_eq!(soa.expire, 604_800);
    assert_eq!(soa.negative_ttl, 86400);
}

#[test]
fn test_create_zone_request_deserialization() {
    let json = r#"{
        "zoneName": "example.com",
        "zoneType": "primary",
        "zoneConfig": {
            "ttl": 3600,
            "soa": {
                "primaryNs": "ns1.example.com.",
                "adminEmail": "admin.example.com.",
                "serial": 2025010101
            },
            "nameServers": ["ns1.example.com."],
            "nameServerIps": {
                "ns1.example.com.": "192.0.2.1"
            },
            "records": []
        },
        "updateKeyName": "test-key"
    }"#;

    let request: CreateZoneRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.zone_name, "example.com");
    assert_eq!(request.zone_type, "primary");
    assert_eq!(request.zone_config.ttl, 3600);
    assert_eq!(request.update_key_name, Some("test-key".to_string()));
}

#[test]
fn test_create_zone_request_without_update_key() {
    let json = r#"{
        "zoneName": "example.com",
        "zoneType": "primary",
        "zoneConfig": {
            "ttl": 3600,
            "soa": {
                "primaryNs": "ns1.example.com.",
                "adminEmail": "admin.example.com.",
                "serial": 2025010101
            },
            "nameServers": ["ns1.example.com."],
            "nameServerIps": {}
        }
    }"#;

    let request: CreateZoneRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.zone_name, "example.com");
    assert!(request.update_key_name.is_none());
}

#[test]
fn test_dns_record_without_optional_fields() {
    let json = r#"{
        "name": "www",
        "type": "A",
        "value": "192.0.2.1"
    }"#;

    let record: DnsRecord = serde_json::from_str(json).unwrap();
    assert_eq!(record.name, "www");
    assert_eq!(record.record_type, "A");
    assert_eq!(record.value, "192.0.2.1");
    assert!(record.ttl.is_none());
    assert!(record.priority.is_none());
}

#[test]
fn test_zone_response_serialization() {
    let response = ZoneResponse {
        success: true,
        message: "Zone created".to_string(),
        details: Some("Output".to_string()),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"message\":\"Zone created\""));
}

// Negative test cases

#[test]
fn test_zone_config_empty_name_servers() {
    let config = ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604_800,
            negative_ttl: 86400,
        },
        name_servers: vec![],
        name_server_ips: HashMap::new(),
        records: vec![],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: None,
        inline_signing: None,
    };

    let zone_file = config.to_zone_file();
    assert!(zone_file.contains("$TTL 3600"));
    assert!(zone_file.contains("@ IN SOA"));
    // Should not have any NS records
    assert!(!zone_file.contains("@ IN NS"));
}

#[test]
fn test_zone_config_special_characters_in_names() {
    let config = ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604_800,
            negative_ttl: 86400,
        },
        name_servers: vec!["ns1.example.com.".to_string()],
        name_server_ips: HashMap::new(),
        records: vec![DnsRecord {
            name: "_dmarc".to_string(),
            record_type: "TXT".to_string(),
            value: "v=DMARC1; p=none".to_string(),
            ttl: None,
            priority: None,
        }],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: None,
        inline_signing: None,
    };

    let zone_file = config.to_zone_file();
    assert!(zone_file.contains("_dmarc"));
    assert!(zone_file.contains("TXT"));
}

#[test]
fn test_zone_config_zero_ttl() {
    let config = ZoneConfig {
        ttl: 0,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 0,
            retry: 0,
            expire: 0,
            negative_ttl: 0,
        },
        name_servers: vec!["ns1.example.com.".to_string()],
        name_server_ips: HashMap::new(),
        records: vec![],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: None,
        inline_signing: None,
    };

    let zone_file = config.to_zone_file();
    assert!(zone_file.contains("$TTL 0"));
}

#[test]
fn test_dns_record_mx_with_priority_zero() {
    let record = DnsRecord {
        name: "@".to_string(),
        record_type: "MX".to_string(),
        value: "mail.example.com.".to_string(),
        ttl: None,
        priority: Some(0),
    };

    let config = ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604_800,
            negative_ttl: 86400,
        },
        name_servers: vec!["ns1.example.com.".to_string()],
        name_server_ips: HashMap::new(),
        records: vec![record],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: None,
        inline_signing: None,
    };

    let zone_file = config.to_zone_file();
    assert!(zone_file.contains("@ IN MX 0 mail.example.com."));
}

#[test]
fn test_multiple_records_same_name() {
    let config = ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604_800,
            negative_ttl: 86400,
        },
        name_servers: vec!["ns1.example.com.".to_string()],
        name_server_ips: HashMap::new(),
        records: vec![
            DnsRecord {
                name: "@".to_string(),
                record_type: "A".to_string(),
                value: "192.0.2.1".to_string(),
                ttl: None,
                priority: None,
            },
            DnsRecord {
                name: "@".to_string(),
                record_type: "A".to_string(),
                value: "192.0.2.2".to_string(),
                ttl: None,
                priority: None,
            },
        ],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: None,
        inline_signing: None,
    };

    let zone_file = config.to_zone_file();
    assert!(zone_file.contains("@ IN A 192.0.2.1"));
    assert!(zone_file.contains("@ IN A 192.0.2.2"));
}

#[test]
fn test_zone_config_with_nameserver_glue_records() {
    let mut ns_ips = HashMap::new();
    ns_ips.insert("ns1.example.com.".to_string(), "192.0.2.1".to_string());
    ns_ips.insert("ns2.example.com.".to_string(), "192.0.2.2".to_string());

    let config = ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604_800,
            negative_ttl: 86400,
        },
        name_servers: vec![
            "ns1.example.com.".to_string(),
            "ns2.example.com.".to_string(),
        ],
        name_server_ips: ns_ips,
        records: vec![],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: None,
        inline_signing: None,
    };

    let zone_file = config.to_zone_file();

    // Check that NS records are present
    assert!(zone_file.contains("@ IN NS ns1.example.com."));
    assert!(zone_file.contains("@ IN NS ns2.example.com."));

    // Check that glue records (A records for nameservers) are present with trailing dots
    assert!(zone_file.contains("ns1.example.com. IN A 192.0.2.1"));
    assert!(zone_file.contains("ns2.example.com. IN A 192.0.2.2"));
}

#[test]
fn test_zone_config_glue_records_serialization() {
    let mut ns_ips = HashMap::new();
    ns_ips.insert("ns1.example.com.".to_string(), "192.0.2.1".to_string());

    let config = ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604_800,
            negative_ttl: 86400,
        },
        name_servers: vec!["ns1.example.com.".to_string()],
        name_server_ips: ns_ips,
        records: vec![],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: None,
        inline_signing: None,
    };

    // Test that it can be serialized to JSON
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("nameServerIps"));

    // Test that it can be deserialized from JSON
    let deserialized: ZoneConfig = serde_json::from_str(&json).unwrap();
    assert!(!deserialized.name_server_ips.is_empty());
    assert_eq!(
        deserialized.name_server_ips.get("ns1.example.com."),
        Some(&"192.0.2.1".to_string())
    );
}

#[test]
fn test_zone_config_without_nameserver_ips() {
    let config = ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604_800,
            negative_ttl: 86400,
        },
        name_servers: vec!["ns1.example.com.".to_string()],
        name_server_ips: HashMap::new(),
        records: vec![],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: None,
        inline_signing: None,
    };

    let zone_file = config.to_zone_file();

    // Should have NS records
    assert!(zone_file.contains("@ IN NS ns1.example.com."));

    // Should NOT have A records for nameservers when name_server_ips is empty
    assert!(!zone_file.contains("ns1.example.com IN A"));
}

#[test]
fn test_zone_config_with_also_notify() {
    let json = r#"{
        "ttl": 3600,
        "soa": {
            "primaryNs": "ns1.example.com.",
            "adminEmail": "admin.example.com.",
            "serial": 2025010101
        },
        "nameServers": ["ns1.example.com."],
        "nameServerIps": {},
        "records": [],
        "alsoNotify": ["10.244.2.101", "10.244.2.102"]
    }"#;

    let config: ZoneConfig = serde_json::from_str(json).unwrap();
    assert!(config.also_notify.is_some());
    let also_notify = config.also_notify.unwrap();
    assert_eq!(also_notify.len(), 2);
    assert_eq!(also_notify[0], "10.244.2.101");
    assert_eq!(also_notify[1], "10.244.2.102");
}

#[test]
fn test_zone_config_with_allow_transfer() {
    let json = r#"{
        "ttl": 3600,
        "soa": {
            "primaryNs": "ns1.example.com.",
            "adminEmail": "admin.example.com.",
            "serial": 2025010101
        },
        "nameServers": ["ns1.example.com."],
        "nameServerIps": {},
        "records": [],
        "allowTransfer": ["10.244.2.101", "10.244.2.102", "10.244.2.103"]
    }"#;

    let config: ZoneConfig = serde_json::from_str(json).unwrap();
    assert!(config.allow_transfer.is_some());
    let allow_transfer = config.allow_transfer.unwrap();
    assert_eq!(allow_transfer.len(), 3);
    assert_eq!(allow_transfer[0], "10.244.2.101");
    assert_eq!(allow_transfer[1], "10.244.2.102");
    assert_eq!(allow_transfer[2], "10.244.2.103");
}

#[test]
fn test_zone_config_with_both_notify_and_transfer() {
    let json = r#"{
        "ttl": 3600,
        "soa": {
            "primaryNs": "ns1.example.com.",
            "adminEmail": "admin.example.com.",
            "serial": 2025010101
        },
        "nameServers": ["ns1.example.com."],
        "nameServerIps": {},
        "records": [],
        "alsoNotify": ["10.244.2.101", "10.244.2.102"],
        "allowTransfer": ["10.244.2.101", "10.244.2.102"]
    }"#;

    let config: ZoneConfig = serde_json::from_str(json).unwrap();
    assert!(config.also_notify.is_some());
    assert!(config.allow_transfer.is_some());

    let also_notify = config.also_notify.unwrap();
    let allow_transfer = config.allow_transfer.unwrap();

    assert_eq!(also_notify.len(), 2);
    assert_eq!(allow_transfer.len(), 2);
    assert_eq!(also_notify, allow_transfer); // Same IPs
}

#[test]
fn test_zone_config_without_notify_and_transfer() {
    let json = r#"{
        "ttl": 3600,
        "soa": {
            "primaryNs": "ns1.example.com.",
            "adminEmail": "admin.example.com.",
            "serial": 2025010101
        },
        "nameServers": ["ns1.example.com."],
        "nameServerIps": {},
        "records": []
    }"#;

    let config: ZoneConfig = serde_json::from_str(json).unwrap();
    assert!(config.also_notify.is_none());
    assert!(config.allow_transfer.is_none());
}

#[test]
fn test_zone_config_with_empty_notify_and_transfer() {
    let json = r#"{
        "ttl": 3600,
        "soa": {
            "primaryNs": "ns1.example.com.",
            "adminEmail": "admin.example.com.",
            "serial": 2025010101
        },
        "nameServers": ["ns1.example.com."],
        "nameServerIps": {},
        "records": [],
        "alsoNotify": [],
        "allowTransfer": []
    }"#;

    let config: ZoneConfig = serde_json::from_str(json).unwrap();
    assert!(config.also_notify.is_some());
    assert!(config.allow_transfer.is_some());
    assert_eq!(config.also_notify.unwrap().len(), 0);
    assert_eq!(config.allow_transfer.unwrap().len(), 0);
}

#[test]
fn test_create_zone_request_with_zone_transfer_fields() {
    let json = r#"{
        "zoneName": "example.com",
        "zoneType": "primary",
        "zoneConfig": {
            "ttl": 3600,
            "soa": {
                "primaryNs": "ns1.example.com.",
                "adminEmail": "admin.example.com.",
                "serial": 2025010101
            },
            "nameServers": ["ns1.example.com."],
            "nameServerIps": {},
            "records": [],
            "alsoNotify": ["10.244.2.101", "10.244.2.102"],
            "allowTransfer": ["10.244.2.101", "10.244.2.102"]
        }
    }"#;

    let request: CreateZoneRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.zone_name, "example.com");
    assert_eq!(request.zone_type, "primary");
    assert!(request.zone_config.also_notify.is_some());
    assert!(request.zone_config.allow_transfer.is_some());

    let also_notify = request.zone_config.also_notify.unwrap();
    let allow_transfer = request.zone_config.allow_transfer.unwrap();

    assert_eq!(also_notify.len(), 2);
    assert_eq!(allow_transfer.len(), 2);
}

// Tests for secondary zones with primaries

#[test]
fn test_zone_config_with_primaries() {
    let json = r#"{
        "ttl": 3600,
        "soa": {
            "primaryNs": "ns1.example.com.",
            "adminEmail": "admin.example.com.",
            "serial": 2025010101
        },
        "nameServers": ["ns1.example.com."],
        "nameServerIps": {},
        "records": [],
        "primaries": ["192.0.2.1", "192.0.2.2"]
    }"#;

    let config: ZoneConfig = serde_json::from_str(json).unwrap();
    assert!(config.primaries.is_some());
    let primaries = config.primaries.unwrap();
    assert_eq!(primaries.len(), 2);
    assert_eq!(primaries[0], "192.0.2.1");
    assert_eq!(primaries[1], "192.0.2.2");
}

#[test]
fn test_create_secondary_zone_request() {
    let json = r#"{
        "zoneName": "example.com",
        "zoneType": "secondary",
        "zoneConfig": {
            "ttl": 3600,
            "soa": {
                "primaryNs": "ns1.example.com.",
                "adminEmail": "admin.example.com.",
                "serial": 2025010101
            },
            "nameServers": ["ns1.example.com."],
            "nameServerIps": {},
            "records": [],
            "primaries": ["192.0.2.1", "192.0.2.2"]
        }
    }"#;

    let request: CreateZoneRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.zone_name, "example.com");
    assert_eq!(request.zone_type, "secondary");
    assert!(request.zone_config.primaries.is_some());

    let primaries = request.zone_config.primaries.unwrap();
    assert_eq!(primaries.len(), 2);
    assert_eq!(primaries[0], "192.0.2.1");
    assert_eq!(primaries[1], "192.0.2.2");
}

#[test]
fn test_secondary_zone_with_single_primary() {
    let json = r#"{
        "zoneName": "example.com",
        "zoneType": "secondary",
        "zoneConfig": {
            "ttl": 3600,
            "soa": {
                "primaryNs": "ns1.example.com.",
                "adminEmail": "admin.example.com.",
                "serial": 2025010101
            },
            "nameServers": ["ns1.example.com."],
            "nameServerIps": {},
            "records": [],
            "primaries": ["192.0.2.1"]
        }
    }"#;

    let request: CreateZoneRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.zone_type, "secondary");
    assert!(request.zone_config.primaries.is_some());

    let primaries = request.zone_config.primaries.unwrap();
    assert_eq!(primaries.len(), 1);
    assert_eq!(primaries[0], "192.0.2.1");
}

#[test]
fn test_zone_config_without_primaries() {
    let json = r#"{
        "ttl": 3600,
        "soa": {
            "primaryNs": "ns1.example.com.",
            "adminEmail": "admin.example.com.",
            "serial": 2025010101
        },
        "nameServers": ["ns1.example.com."],
        "nameServerIps": {},
        "records": []
    }"#;

    let config: ZoneConfig = serde_json::from_str(json).unwrap();
    assert!(config.primaries.is_none());
}

#[test]
fn test_zone_config_with_empty_primaries() {
    let json = r#"{
        "ttl": 3600,
        "soa": {
            "primaryNs": "ns1.example.com.",
            "adminEmail": "admin.example.com.",
            "serial": 2025010101
        },
        "nameServers": ["ns1.example.com."],
        "nameServerIps": {},
        "records": [],
        "primaries": []
    }"#;

    let config: ZoneConfig = serde_json::from_str(json).unwrap();
    assert!(config.primaries.is_some());
    assert_eq!(config.primaries.unwrap().len(), 0);
}

#[test]
fn test_secondary_zone_with_all_transfer_fields() {
    let json = r#"{
        "zoneName": "example.com",
        "zoneType": "secondary",
        "zoneConfig": {
            "ttl": 3600,
            "soa": {
                "primaryNs": "ns1.example.com.",
                "adminEmail": "admin.example.com.",
                "serial": 2025010101
            },
            "nameServers": ["ns1.example.com."],
            "nameServerIps": {},
            "records": [],
            "primaries": ["192.0.2.1", "192.0.2.2"],
            "alsoNotify": ["10.244.2.101"],
            "allowTransfer": ["10.244.2.101", "10.244.2.102"]
        }
    }"#;

    let request: CreateZoneRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.zone_type, "secondary");
    assert!(request.zone_config.primaries.is_some());
    assert!(request.zone_config.also_notify.is_some());
    assert!(request.zone_config.allow_transfer.is_some());

    let primaries = request.zone_config.primaries.unwrap();
    let also_notify = request.zone_config.also_notify.unwrap();
    let allow_transfer = request.zone_config.allow_transfer.unwrap();

    assert_eq!(primaries.len(), 2);
    assert_eq!(also_notify.len(), 1);
    assert_eq!(allow_transfer.len(), 2);
}

// Tests for ModifyZoneRequest

#[test]
fn test_modify_zone_request_with_both_fields() {
    let json = r#"{
        "alsoNotify": ["10.244.2.101", "10.244.2.102"],
        "allowTransfer": ["10.244.2.103", "10.244.2.104"]
    }"#;

    let request: ModifyZoneRequest = serde_json::from_str(json).unwrap();
    assert!(request.also_notify.is_some());
    assert!(request.allow_transfer.is_some());

    let also_notify = request.also_notify.unwrap();
    let allow_transfer = request.allow_transfer.unwrap();

    assert_eq!(also_notify.len(), 2);
    assert_eq!(also_notify[0], "10.244.2.101");
    assert_eq!(also_notify[1], "10.244.2.102");

    assert_eq!(allow_transfer.len(), 2);
    assert_eq!(allow_transfer[0], "10.244.2.103");
    assert_eq!(allow_transfer[1], "10.244.2.104");
}

#[test]
fn test_modify_zone_request_only_also_notify() {
    let json = r#"{
        "alsoNotify": ["10.244.2.101"]
    }"#;

    let request: ModifyZoneRequest = serde_json::from_str(json).unwrap();
    assert!(request.also_notify.is_some());
    assert!(request.allow_transfer.is_none());

    let also_notify = request.also_notify.unwrap();
    assert_eq!(also_notify.len(), 1);
    assert_eq!(also_notify[0], "10.244.2.101");
}

#[test]
fn test_modify_zone_request_only_allow_transfer() {
    let json = r#"{
        "allowTransfer": ["10.244.2.101", "10.244.2.102", "10.244.2.103"]
    }"#;

    let request: ModifyZoneRequest = serde_json::from_str(json).unwrap();
    assert!(request.also_notify.is_none());
    assert!(request.allow_transfer.is_some());

    let allow_transfer = request.allow_transfer.unwrap();
    assert_eq!(allow_transfer.len(), 3);
    assert_eq!(allow_transfer[0], "10.244.2.101");
    assert_eq!(allow_transfer[1], "10.244.2.102");
    assert_eq!(allow_transfer[2], "10.244.2.103");
}

#[test]
fn test_modify_zone_request_with_empty_arrays() {
    let json = r#"{
        "alsoNotify": [],
        "allowTransfer": []
    }"#;

    let request: ModifyZoneRequest = serde_json::from_str(json).unwrap();
    assert!(request.also_notify.is_some());
    assert!(request.allow_transfer.is_some());

    assert_eq!(request.also_notify.unwrap().len(), 0);
    assert_eq!(request.allow_transfer.unwrap().len(), 0);
}

#[test]
fn test_modify_zone_request_empty_json() {
    let json = r#"{}"#;

    let request: ModifyZoneRequest = serde_json::from_str(json).unwrap();
    assert!(request.also_notify.is_none());
    assert!(request.allow_transfer.is_none());
}

#[test]
fn test_modify_zone_request_serialization() {
    let request = ModifyZoneRequest {
        also_notify: Some(vec!["10.244.2.101".to_string()]),
        allow_transfer: Some(vec!["10.244.2.102".to_string()]),
        allow_update: Some(vec!["10.244.2.103".to_string()]),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("alsoNotify"));
    assert!(json.contains("allowTransfer"));
    assert!(json.contains("allowUpdate"));
    assert!(json.contains("10.244.2.101"));
    assert!(json.contains("10.244.2.102"));
    assert!(json.contains("10.244.2.103"));
}

#[test]
fn test_modify_zone_request_serialization_skip_none() {
    let request = ModifyZoneRequest {
        also_notify: Some(vec!["10.244.2.101".to_string()]),
        allow_transfer: None,
        allow_update: None,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("alsoNotify"));
    assert!(!json.contains("allowTransfer")); // Should be skipped when None
    assert!(!json.contains("allowUpdate")); // Should be skipped when None
}

#[test]
fn test_modify_zone_request_with_ipv6_addresses() {
    let json = r#"{
        "alsoNotify": ["2001:db8::1", "2001:db8::2"],
        "allowTransfer": ["2001:db8::3"]
    }"#;

    let request: ModifyZoneRequest = serde_json::from_str(json).unwrap();
    assert!(request.also_notify.is_some());
    assert!(request.allow_transfer.is_some());

    let also_notify = request.also_notify.unwrap();
    let allow_transfer = request.allow_transfer.unwrap();

    assert_eq!(also_notify.len(), 2);
    assert_eq!(also_notify[0], "2001:db8::1");
    assert_eq!(allow_transfer[0], "2001:db8::3");
}

#[test]
fn test_modify_zone_request_with_mixed_ip_versions() {
    let json = r#"{
        "alsoNotify": ["10.244.2.101", "2001:db8::1"],
        "allowTransfer": ["192.168.1.1", "fe80::1"]
    }"#;

    let request: ModifyZoneRequest = serde_json::from_str(json).unwrap();
    let also_notify = request.also_notify.unwrap();
    let allow_transfer = request.allow_transfer.unwrap();

    assert_eq!(also_notify.len(), 2);
    assert_eq!(also_notify[0], "10.244.2.101");
    assert_eq!(also_notify[1], "2001:db8::1");

    assert_eq!(allow_transfer.len(), 2);
    assert_eq!(allow_transfer[0], "192.168.1.1");
    assert_eq!(allow_transfer[1], "fe80::1");
}

// Tests for journal file cleanup behavior

#[tokio::test]
async fn test_journal_file_cleanup_on_zone_creation() {
    use tempfile::TempDir;

    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let zone_name = "test-journal-create.com";

    // Create a fake old journal file
    let journal_file_name = format!("{}.zone.jnl", zone_name);
    let journal_file_path = temp_dir.path().join(&journal_file_name);
    tokio::fs::write(&journal_file_path, "old journal data")
        .await
        .unwrap();

    // Verify the journal file exists
    assert!(journal_file_path.exists());

    // Simulate the cleanup logic from create_zone
    if journal_file_path.exists() {
        tokio::fs::remove_file(&journal_file_path).await.unwrap();
    }

    // Verify the journal file was removed
    assert!(!journal_file_path.exists());
}

#[tokio::test]
async fn test_journal_file_cleanup_on_zone_deletion() {
    use tempfile::TempDir;

    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let zone_name = "test-journal-delete.com";

    // Create fake zone and journal files
    let zone_file_name = format!("{}.zone", zone_name);
    let zone_file_path = temp_dir.path().join(&zone_file_name);
    tokio::fs::write(&zone_file_path, "zone data")
        .await
        .unwrap();

    let journal_file_name = format!("{}.zone.jnl", zone_name);
    let journal_file_path = temp_dir.path().join(&journal_file_name);
    tokio::fs::write(&journal_file_path, "journal data")
        .await
        .unwrap();

    // Verify both files exist
    assert!(zone_file_path.exists());
    assert!(journal_file_path.exists());

    // Simulate the cleanup logic from delete_zone
    if zone_file_path.exists() {
        tokio::fs::remove_file(&zone_file_path).await.unwrap();
    }

    if journal_file_path.exists() {
        tokio::fs::remove_file(&journal_file_path).await.unwrap();
    }

    // Verify both files were removed
    assert!(!zone_file_path.exists());
    assert!(!journal_file_path.exists());
}

#[tokio::test]
async fn test_journal_file_cleanup_when_journal_does_not_exist() {
    use tempfile::TempDir;

    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let zone_name = "test-no-journal.com";

    // Create only the zone file, no journal
    let zone_file_name = format!("{}.zone", zone_name);
    let zone_file_path = temp_dir.path().join(&zone_file_name);
    tokio::fs::write(&zone_file_path, "zone data")
        .await
        .unwrap();

    let journal_file_name = format!("{}.zone.jnl", zone_name);
    let journal_file_path = temp_dir.path().join(&journal_file_name);

    // Verify journal doesn't exist
    assert!(!journal_file_path.exists());

    // Simulate the cleanup logic - should not error when journal doesn't exist
    if journal_file_path.exists() {
        tokio::fs::remove_file(&journal_file_path).await.unwrap();
    }

    // Should complete without errors
    assert!(!journal_file_path.exists());
}

#[test]
fn test_journal_file_naming_convention() {
    // Test that journal file names follow the expected pattern
    let zone_name = "example.com";
    let expected_zone_file = format!("{}.zone", zone_name);
    let expected_journal_file = format!("{}.zone.jnl", zone_name);

    assert_eq!(expected_zone_file, "example.com.zone");
    assert_eq!(expected_journal_file, "example.com.zone.jnl");

    // Test with subdomain
    let zone_name = "sub.example.com";
    let expected_journal_file = format!("{}.zone.jnl", zone_name);
    assert_eq!(expected_journal_file, "sub.example.com.zone.jnl");
}

// ---------------------------------------------------------------------------
// Input validation security tests (B-1 path traversal, B-3 RNDC injection)
// ---------------------------------------------------------------------------

#[test]
fn test_validate_zone_name_accepts_legitimate_names() {
    for name in [
        "example.com",
        "sub.example.com",
        "a",
        "my-zone_1.test",
        "10",
    ] {
        assert!(
            validate_zone_name(name).is_ok(),
            "expected {name:?} to be accepted"
        );
    }
}

#[test]
fn test_validate_zone_name_rejects_path_traversal() {
    for name in [
        "../etc/passwd",
        "..",
        "foo/../bar",
        "a/b",
        "/etc/bind",
        "foo\\bar",
    ] {
        assert!(
            validate_zone_name(name).is_err(),
            "expected {name:?} to be rejected"
        );
    }
}

#[test]
fn test_validate_zone_name_rejects_control_chars_and_whitespace() {
    for name in ["foo\nbar", "foo\rbar", "foo\0bar", "foo bar", "\tzone"] {
        assert!(
            validate_zone_name(name).is_err(),
            "expected {name:?} to be rejected"
        );
    }
}

#[test]
fn test_validate_zone_name_rejects_empty_and_leading_separator() {
    assert!(validate_zone_name("").is_err());
    assert!(validate_zone_name(".example.com").is_err());
    assert!(validate_zone_name("-example.com").is_err());
    assert!(validate_zone_name("_dmarc.example.com").is_err());
}

#[test]
fn test_validate_zone_name_rejects_overlong() {
    let long = "a".repeat(254);
    assert!(validate_zone_name(&long).is_err());
}

#[test]
fn test_validate_rndc_identifier_accepts_legitimate_values() {
    for value in ["update-key", "rndc-key", "default", "high.security_1"] {
        assert!(
            validate_rndc_identifier("updateKeyName", value).is_ok(),
            "expected {value:?} to be accepted"
        );
    }
}

#[test]
fn test_validate_rndc_identifier_rejects_quote_breakout() {
    // Payloads that would break out of the quoted rndc config literal.
    for value in [
        "key\"; };",
        "evil\"",
        "a;b",
        "a{b}",
        "key with space",
        "key\nallow-update { any; };",
    ] {
        assert!(
            validate_rndc_identifier("updateKeyName", value).is_err(),
            "expected {value:?} to be rejected"
        );
    }
}

#[test]
fn test_validate_rndc_identifier_rejects_empty() {
    assert!(validate_rndc_identifier("dnssecPolicy", "").is_err());
}

#[test]
fn test_resolve_zone_dir_returns_canonical_path_for_existing_directory() {
    // Arrange: a real directory on disk.
    let dir = tempfile::tempdir().expect("create temp dir");

    // Act
    let resolved =
        resolve_zone_dir(dir.path().to_str().unwrap()).expect("existing directory should resolve");

    // Assert: result equals the fully canonicalized path (symlinks like macOS
    // /var -> /private/var are resolved).
    let expected = std::fs::canonicalize(dir.path()).unwrap();
    assert_eq!(resolved, expected.to_str().unwrap());
}

#[test]
fn test_resolve_zone_dir_normalizes_parent_directory_references() {
    // Arrange: a nested directory accessed via a `..` traversal segment.
    let dir = tempfile::tempdir().expect("create temp dir");
    let nested = dir.path().join("sub");
    std::fs::create_dir(&nested).unwrap();
    let traversal = nested.join(".."); // <tmp>/sub/.. == <tmp>

    // Act
    let resolved =
        resolve_zone_dir(traversal.to_str().unwrap()).expect("traversal path should resolve");

    // Assert: `..` is collapsed against the real filesystem.
    let expected = std::fs::canonicalize(dir.path()).unwrap();
    assert_eq!(resolved, expected.to_str().unwrap());
}

#[test]
fn test_resolve_zone_dir_rejects_nonexistent_path() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let missing = dir.path().join("does-not-exist");

    let result = resolve_zone_dir(missing.to_str().unwrap());

    assert!(result.is_err(), "non-existent path must be rejected");
}

#[test]
fn test_resolve_zone_dir_rejects_non_directory() {
    // Arrange: a regular file, not a directory.
    let dir = tempfile::tempdir().expect("create temp dir");
    let file_path = dir.path().join("named.conf");
    std::fs::write(&file_path, b"// not a directory").unwrap();

    // Act
    let result = resolve_zone_dir(file_path.to_str().unwrap());

    // Assert
    assert!(result.is_err(), "a file path must be rejected");
}

// ---------------------------------------------------------------------------
// C-1 / C-2: create_zone injection-sink validation (pure helpers)
// ---------------------------------------------------------------------------

/// Build a minimal, valid primary-zone `ZoneConfig` for mutation in tests.
fn clean_zone_config() -> ZoneConfig {
    ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604_800,
            negative_ttl: 86400,
        },
        name_servers: vec!["ns1.example.com.".to_string()],
        name_server_ips: HashMap::new(),
        records: vec![DnsRecord {
            name: "www".to_string(),
            record_type: "A".to_string(),
            value: "192.0.2.1".to_string(),
            ttl: None,
            priority: None,
        }],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: None,
        inline_signing: None,
    }
}

#[test]
fn test_validate_ip_list_accepts_valid_v4_and_v6() {
    let ips = vec![
        "192.0.2.1".to_string(),
        "10.0.0.5".to_string(),
        "2001:db8::1".to_string(),
    ];
    assert!(validate_ip_list("primaries", &ips).is_ok());
    // Empty list is a no-op.
    assert!(validate_ip_list("primaries", &[]).is_ok());
}

#[test]
fn test_validate_ip_list_rejects_rndc_config_injection() {
    // C-1: the classic brace-breakout payload must be rejected.
    let payload = vec![r#"1.2.3.4; }; zone "x" { type primary; file "/etc/passwd"; "#.to_string()];
    assert!(validate_ip_list("primaries", &payload).is_err());

    // Other non-IP / metacharacter entries.
    assert!(validate_ip_list("also-notify", &["not-an-ip".to_string()]).is_err());
    assert!(validate_ip_list("allow-transfer", &["192.0.2.1; any".to_string()]).is_err());
    assert!(validate_ip_list("primaries", &["".to_string()]).is_err());
}

#[test]
fn test_validate_ip_port_list_accepts_bare_and_ported_entries() {
    let entries = vec![
        "192.0.2.1".to_string(),
        "2001:db8::1".to_string(),
        "192.0.2.2:5353".to_string(),
        "[2001:db8::2]:53".to_string(),
    ];
    assert!(validate_ip_port_list("primaries", &entries).is_ok());
    assert!(validate_ip_port_list("also-notify", &[]).is_ok());
}

#[test]
fn test_validate_ip_port_list_rejects_bad_entries() {
    assert!(validate_ip_port_list("primaries", &["not-an-ip".to_string()]).is_err());
    assert!(validate_ip_port_list("primaries", &["192.0.2.1:99999".to_string()]).is_err());
    assert!(validate_ip_port_list("primaries", &["192.0.2.1:abc".to_string()]).is_err());
    assert!(validate_ip_port_list("primaries", &["192.0.2.1 port 5353".to_string()]).is_err());
    assert!(validate_ip_port_list("primaries", &["[bad]:5353".to_string()]).is_err());
    assert!(validate_ip_port_list(
        "primaries",
        &[r#"1.2.3.4; }; zone "x" { type primary; "#.to_string()]
    )
    .is_err());
    assert!(validate_ip_port_list("primaries", &["".to_string()]).is_err());
}

#[test]
fn test_render_ip_port_entry() {
    // Bare IPs pass through unchanged (with trailing "; ")
    assert_eq!(render_ip_port_entry("192.0.2.1"), "192.0.2.1; ");
    assert_eq!(render_ip_port_entry("2001:db8::1"), "2001:db8::1; ");
    // IPv4:port converts to BIND "port" syntax
    assert_eq!(
        render_ip_port_entry("192.0.2.2:5353"),
        "192.0.2.2 port 5353; "
    );
    // IPv6 bracketed:port converts to BIND "port" syntax (unbracketed)
    assert_eq!(
        render_ip_port_entry("[2001:db8::2]:53"),
        "2001:db8::2 port 53; "
    );
}

#[test]
fn test_validate_zone_config_content_accepts_clean_config() {
    assert!(validate_zone_config_content(&clean_zone_config()).is_ok());
}

#[test]
fn test_validate_zone_config_content_rejects_include_via_record_value() {
    // C-2: newline + $INCLUDE injected through a record value (A record here is
    // also rejected for not being a valid IPv4, but a newline is the core gap).
    let mut config = clean_zone_config();
    config.records[0].value = "192.0.2.1\n$INCLUDE /etc/bind/rndc.key".to_string();
    assert!(validate_zone_config_content(&config).is_err());

    // Same via a TXT value, which previously accepted "any non-empty string".
    let mut config = clean_zone_config();
    config.records[0].record_type = "TXT".to_string();
    config.records[0].value = "\"ok\"\n$INCLUDE /etc/shadow".to_string();
    assert!(validate_zone_config_content(&config).is_err());
}

#[test]
fn test_validate_zone_config_content_rejects_injection_via_record_name() {
    let mut config = clean_zone_config();
    config.records[0].name = "x\n$INCLUDE /etc/shadow".to_string();
    assert!(validate_zone_config_content(&config).is_err());
}

#[test]
fn test_validate_zone_config_content_rejects_include_directive_without_control_char() {
    // The record name is the first token on its zone-file line, so a directive
    // planted with spaces + `;` (no newline, no control char) must be rejected.
    let mut config = clean_zone_config();
    config.records[0].name = "$INCLUDE /etc/bind/rndc.key ;".to_string();
    assert!(
        validate_zone_config_content(&config).is_err(),
        "control-char-free $INCLUDE via record name must be rejected"
    );

    // $GENERATE resource-exhaustion via a control-char-free record name.
    let mut config = clean_zone_config();
    config.records[0].name = "$GENERATE 1-16777215 host$".to_string();
    assert!(validate_zone_config_content(&config).is_err());
}

#[test]
fn test_validate_zone_config_content_rejects_directive_via_glue_hostname() {
    // Glue hostnames (nameServerIps keys) are also rendered at line start.
    let mut config = clean_zone_config();
    config.name_server_ips.insert(
        "$INCLUDE /etc/shadow ;".to_string(),
        "192.0.2.1".to_string(),
    );
    assert!(
        validate_zone_config_content(&config).is_err(),
        "control-char-free $INCLUDE via glue hostname must be rejected"
    );
}

#[test]
fn test_validate_zone_config_content_rejects_injection_via_soa_and_ns() {
    // SOA primaryNs / adminEmail.
    let mut config = clean_zone_config();
    config.soa.primary_ns = "ns1.example.com.\n$INCLUDE /etc/shadow".to_string();
    assert!(validate_zone_config_content(&config).is_err());

    let mut config = clean_zone_config();
    config.soa.admin_email = "admin.example.com.\n$GENERATE 1-9999999".to_string();
    assert!(validate_zone_config_content(&config).is_err());

    // Name-server entry.
    let mut config = clean_zone_config();
    config
        .name_servers
        .push("evil.\n$INCLUDE /etc/shadow".to_string());
    assert!(validate_zone_config_content(&config).is_err());
}

#[test]
fn test_validate_zone_config_content_rejects_bad_glue_ip() {
    let mut config = clean_zone_config();
    config.name_server_ips.insert(
        "ns1.example.com.".to_string(),
        "1.2.3.4\n$INCLUDE /x".to_string(),
    );
    assert!(validate_zone_config_content(&config).is_err());

    // Non-IP glue value also rejected.
    let mut config = clean_zone_config();
    config
        .name_server_ips
        .insert("ns1.example.com.".to_string(), "not-an-ip".to_string());
    assert!(validate_zone_config_content(&config).is_err());
}

#[tokio::test]
async fn test_reload_zone_rejects_invalid_zone_name() {
    let state = offline_app_state();
    let result = reload_zone(State(state), Path(MALICIOUS_ZONE_NAME.to_string())).await;
    assert!(
        matches!(result, Err(ApiError::InvalidRequest(_))),
        "reload_zone must reject an invalid zone name before touching rndc"
    );
}

#[tokio::test]
async fn test_zone_status_rejects_invalid_zone_name() {
    let state = offline_app_state();
    let result = zone_status(State(state), Path(MALICIOUS_ZONE_NAME.to_string())).await;
    assert!(
        matches!(result, Err(ApiError::InvalidRequest(_))),
        "zone_status must reject an invalid zone name before touching rndc"
    );
}

#[tokio::test]
async fn test_freeze_zone_rejects_invalid_zone_name() {
    let state = offline_app_state();
    let result = freeze_zone(State(state), Path(MALICIOUS_ZONE_NAME.to_string())).await;
    assert!(
        matches!(result, Err(ApiError::InvalidRequest(_))),
        "freeze_zone must reject an invalid zone name before touching rndc"
    );
}

#[tokio::test]
async fn test_thaw_zone_rejects_invalid_zone_name() {
    let state = offline_app_state();
    let result = thaw_zone(State(state), Path(MALICIOUS_ZONE_NAME.to_string())).await;
    assert!(
        matches!(result, Err(ApiError::InvalidRequest(_))),
        "thaw_zone must reject an invalid zone name before touching rndc"
    );
}

#[tokio::test]
async fn test_notify_zone_rejects_invalid_zone_name() {
    let state = offline_app_state();
    let result = notify_zone(State(state), Path(MALICIOUS_ZONE_NAME.to_string())).await;
    assert!(
        matches!(result, Err(ApiError::InvalidRequest(_))),
        "notify_zone must reject an invalid zone name before touching rndc"
    );
}

#[tokio::test]
async fn test_retransfer_zone_rejects_invalid_zone_name() {
    let state = offline_app_state();
    let result = retransfer_zone(State(state), Path(MALICIOUS_ZONE_NAME.to_string())).await;
    assert!(
        matches!(result, Err(ApiError::InvalidRequest(_))),
        "retransfer_zone must reject an invalid zone name before touching rndc"
    );
}

#[tokio::test]
async fn test_get_zone_rejects_invalid_zone_name() {
    // get_zone joins the name into a filesystem path (zone_dir/{name}.zone), so
    // an unvalidated traversal name is a path-traversal existence oracle. The
    // name must be rejected before the path is ever constructed or probed.
    let state = offline_app_state();
    let result = get_zone(State(state), Path(MALICIOUS_ZONE_NAME.to_string())).await;
    assert!(
        matches!(result, Err(ApiError::InvalidRequest(_))),
        "get_zone must reject an invalid zone name before building a filesystem path"
    );
}

#[tokio::test]
async fn test_modify_zone_rejects_invalid_zone_name() {
    // modify_zone also joins the name into a filesystem path and forwards it to
    // rndc; validation must run before either, even when the body is well-formed.
    let state = offline_app_state();
    let request = ModifyZoneRequest {
        also_notify: Some(vec!["192.0.2.1".to_string()]),
        allow_transfer: None,
        allow_update: None,
    };
    let result = modify_zone(
        State(state),
        Path(MALICIOUS_ZONE_NAME.to_string()),
        axum::Json(request),
    )
    .await;
    assert!(
        matches!(result, Err(ApiError::InvalidRequest(_))),
        "modify_zone must reject an invalid zone name before building a filesystem path"
    );
}

#[test]
fn test_is_normalized_zone_dir_accepts_absolute_normalized_path() {
    assert!(is_normalized_zone_dir("/etc/bind/zones"));
    assert!(is_normalized_zone_dir("/var/lib/bind"));
    // A single root component is still absolute and normalized.
    assert!(is_normalized_zone_dir("/"));
}

#[test]
fn test_is_normalized_zone_dir_rejects_relative_path() {
    assert!(!is_normalized_zone_dir("etc/bind/zones"));
    assert!(!is_normalized_zone_dir("zones"));
    assert!(!is_normalized_zone_dir(""));
}

#[test]
fn test_is_normalized_zone_dir_rejects_parent_dir_traversal() {
    assert!(!is_normalized_zone_dir("/etc/bind/../../etc/shadow"));
    assert!(!is_normalized_zone_dir("/../etc"));
    // A leading separator followed by ".." must not slip through.
    assert!(!is_normalized_zone_dir("/etc/bind/.."));
}

#[test]
fn test_is_normalized_zone_dir_accepts_embedded_current_dir() {
    // `Path::components()` transparently drops `.` (current-dir) components from
    // an absolute path, so these normalize to a safe location and are accepted.
    // The meaningful traversal guard is against `..`, which Rust cannot collapse.
    assert!(is_normalized_zone_dir("/etc/bind/./zones"));
    assert!(is_normalized_zone_dir("/./etc"));
}

#[test]
fn test_resolve_zone_dir_output_is_normalized() {
    // The canonicalized output of resolve_zone_dir must always satisfy the
    // ready-check barrier; this couples the startup guard to the sink guard.
    let dir = tempfile::tempdir().expect("create temp dir");
    let resolved =
        resolve_zone_dir(dir.path().to_str().unwrap()).expect("existing directory should resolve");
    assert!(is_normalized_zone_dir(&resolved));
}
