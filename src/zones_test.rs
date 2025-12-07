// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for zones module

use super::zones::*;
use std::collections::HashMap;

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
    };

    let zone_file = config.to_zone_file();

    // Should have NS records
    assert!(zone_file.contains("@ IN NS ns1.example.com."));

    // Should NOT have A records for nameservers when name_server_ips is empty
    assert!(!zone_file.contains("ns1.example.com IN A"));
}
