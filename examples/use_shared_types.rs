// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Example showing how to use bindcar's shared types in another project (like bindy)
//!
//! This demonstrates how bindy can import bindcar as a dependency and use the
//! request/response types without maintaining duplicate definitions.

use bindcar::{CreateZoneRequest, DnsRecord, SoaRecord, ZoneConfig, ZoneResponse, ZONE_TYPE_PRIMARY};
use std::collections::HashMap;

fn main() {
    // Example 1: Create a zone creation request
    println!("=== Example 1: Creating a ZoneConfig ===\n");

    let zone_config = ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604800,
            negative_ttl: 86400,
        },
        name_servers: vec![
            "ns1.example.com.".to_string(),
            "ns2.example.com.".to_string(),
        ],
        name_server_ips: HashMap::new(), // Can provide IPs for glue records
        records: vec![
            DnsRecord {
                name: "@".to_string(),
                record_type: "A".to_string(),
                value: "192.0.2.1".to_string(),
                ttl: None,
                priority: None,
            },
            DnsRecord {
                name: "www".to_string(),
                record_type: "A".to_string(),
                value: "192.0.2.2".to_string(),
                ttl: Some(7200),
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
    };

    let request = CreateZoneRequest {
        zone_name: "example.com".to_string(),
        zone_type: ZONE_TYPE_PRIMARY.to_string(),
        zone_config: zone_config.clone(),
        update_key_name: None,
    };

    // Example 2: Serialize to JSON (for HTTP API calls)
    println!("=== Example 2: Serializing to JSON ===\n");
    let json = serde_json::to_string_pretty(&request).unwrap();
    println!("Request JSON:\n{}\n", json);

    // Example 3: Generate zone file
    println!("=== Example 3: Generating Zone File ===\n");
    let zone_file = zone_config.to_zone_file();
    println!("Generated zone file:\n{}", zone_file);

    // Example 4: Parse a response
    println!("=== Example 4: Parsing API Response ===\n");
    let response_json = r#"{
        "success": true,
        "message": "Zone created successfully",
        "details": "Zone example.com added to BIND9"
    }"#;

    let response: ZoneResponse = serde_json::from_str(response_json).unwrap();
    println!("Parsed response:");
    println!("  Success: {}", response.success);
    println!("  Message: {}", response.message);
    if let Some(details) = response.details {
        println!("  Details: {}", details);
    }

    // Example 5: Zone with nameserver glue records
    println!("\n=== Example 5: Zone with Nameserver Glue Records ===\n");

    let mut ns_ips_glue = HashMap::new();
    ns_ips_glue.insert("ns1.example.com.".to_string(), "192.0.2.10".to_string());
    ns_ips_glue.insert("ns2.example.com.".to_string(), "192.0.2.11".to_string());

    let zone_with_glue = ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.example.com.".to_string(),
            admin_email: "admin.example.com.".to_string(),
            serial: 2025010101,
            refresh: 3600,
            retry: 600,
            expire: 604800,
            negative_ttl: 86400,
        },
        name_servers: vec![
            "ns1.example.com.".to_string(),
            "ns2.example.com.".to_string(),
        ],
        name_server_ips: ns_ips_glue, // Provide IPs for glue records
        records: vec![
            DnsRecord {
                name: "@".to_string(),
                record_type: "A".to_string(),
                value: "192.0.2.1".to_string(),
                ttl: None,
                priority: None,
            },
        ],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
    };

    println!("Zone file with glue records:\n{}", zone_with_glue.to_zone_file());

    println!("\n=== Integration Notes ===\n");
    println!("To use these types in bindy, add to Cargo.toml:");
    println!("[dependencies]");
    println!("bindcar = \"0.1.0\"");
    println!("\nThen import the types:");
    println!("use bindcar::{{CreateZoneRequest, ZoneConfig, SoaRecord, DnsRecord}};");
    println!("\nThis ensures type compatibility between bindy and bindcar!");
}
