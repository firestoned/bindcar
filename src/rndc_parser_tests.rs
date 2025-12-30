// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Comprehensive unit tests for RNDC parser
//!
//! This module contains all tests for the RNDC parser, including:
//! - Basic parsing primitives (quoted strings, IP addresses, etc.)
//! - Zone configuration parsing (primary, secondary, etc.)
//! - Allow-update directive handling (IPs and key-based)
//! - Serialization and round-trip tests
//! - Real-world production scenario tests

#[cfg(test)]
mod tests {
    use crate::rndc_parser::parse_showzone;
    use crate::rndc_types::{ZoneConfig, ZoneType};
    use std::net::IpAddr;

    // ========== Parsing Primitives ==========

    #[test]
    fn test_parse_quoted_string() {
        use crate::rndc_parser::quoted_string;
        assert_eq!(quoted_string(r#""example.com""#).unwrap().1, "example.com");
    }

    #[test]
    fn test_parse_ip_addr() {
        use crate::rndc_parser::ip_addr;
        assert_eq!(
            ip_addr("192.168.1.1").unwrap().1,
            "192.168.1.1".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            ip_addr("2001:db8::1").unwrap().1,
            "2001:db8::1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn test_parse_ip_addr_with_cidr() {
        use crate::rndc_parser::ip_addr;
        // Test CIDR notation (subnet mask is parsed but ignored)
        assert_eq!(
            ip_addr("192.168.1.1/32").unwrap().1,
            "192.168.1.1".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            ip_addr("10.244.1.18/32").unwrap().1,
            "10.244.1.18".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            ip_addr("2001:db8::1/128").unwrap().1,
            "2001:db8::1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn test_parse_ip_with_port() {
        use crate::rndc_parser::ip_with_port;
        let result = ip_with_port("192.168.1.1 port 5353").unwrap().1;
        assert_eq!(result.address, "192.168.1.1".parse::<IpAddr>().unwrap());
        assert_eq!(result.port, Some(5353));
    }

    // ========== Zone Type Parsing ==========

    #[test]
    fn test_parse_primary_zone() {
        let input = r#"zone "example.com" { type primary; file "/var/cache/bind/example.com.zone"; };"#;
        let config = parse_showzone(input).unwrap();
        assert_eq!(config.zone_name, "example.com");
        assert_eq!(config.zone_type, ZoneType::Primary);
        assert_eq!(config.file, Some("/var/cache/bind/example.com.zone".to_string()));
    }

    #[test]
    fn test_parse_secondary_zone() {
        let input = r#"zone "example.org" { type secondary; primaries { 192.0.2.1; 192.0.2.2 port 5353; }; file "/var/cache/bind/secondary/example.org.zone"; };"#;
        let config = parse_showzone(input).unwrap();
        assert_eq!(config.zone_name, "example.org");
        assert_eq!(config.zone_type, ZoneType::Secondary);
        assert_eq!(config.primaries.as_ref().unwrap().len(), 2);
        assert_eq!(config.primaries.as_ref().unwrap()[1].port, Some(5353));
    }

    // ========== Zone Directives Parsing ==========

    #[test]
    fn test_parse_zone_with_also_notify() {
        let input = r#"zone "example.com" { type primary; file "/var/cache/bind/example.com.zone"; also-notify { 10.244.2.101; 10.244.2.102; }; };"#;
        let config = parse_showzone(input).unwrap();
        assert_eq!(config.also_notify.as_ref().unwrap().len(), 2);
        assert_eq!(config.also_notify.as_ref().unwrap()[0], "10.244.2.101".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_parse_zone_with_allow_transfer() {
        let input = r#"zone "example.com" { type primary; file "/var/cache/bind/example.com.zone"; allow-transfer { 10.1.1.1; 10.2.2.2; }; };"#;
        let config = parse_showzone(input).unwrap();
        assert_eq!(config.allow_transfer.as_ref().unwrap().len(), 2);
        assert_eq!(config.allow_transfer.as_ref().unwrap()[0], "10.1.1.1".parse::<IpAddr>().unwrap());
        assert_eq!(config.allow_transfer.as_ref().unwrap()[1], "10.2.2.2".parse::<IpAddr>().unwrap());
    }

    // ========== Allow-Update Parsing Tests ==========

    #[test]
    fn test_parse_allow_update_with_key_only() {
        // Test parsing allow-update with only key reference (no IPs)
        let input = r#"zone "test.com" { type primary; file "/var/cache/bind/test.com.zone"; allow-update { key "mykey"; }; };"#;

        let config = parse_showzone(input).unwrap();

        // Should have raw directive
        assert!(
            config.allow_update_raw.is_some(),
            "Should capture raw directive for key-based allow-update"
        );
        let raw = config.allow_update_raw.as_ref().unwrap();
        assert!(raw.contains("key"), "Raw should contain 'key' keyword");
        assert!(raw.contains("mykey"), "Raw should contain key name");

        // Should not have IP list
        assert!(
            config.allow_update.is_none() || config.allow_update.as_ref().unwrap().is_empty(),
            "Should not have IPs when only key is present"
        );
    }

    #[test]
    fn test_parse_allow_update_with_ips_only() {
        // Test parsing allow-update with only IP addresses (no keys)
        let input = r#"zone "test.com" { type primary; file "/var/cache/bind/test.com.zone"; allow-update { 10.1.1.1; 10.2.2.2; }; };"#;

        let config = parse_showzone(input).unwrap();

        // Should have IP list
        assert!(config.allow_update.is_some(), "Should have IP list");
        let ips = config.allow_update.as_ref().unwrap();
        assert_eq!(ips.len(), 2, "Should have 2 IPs");
        assert_eq!(ips[0].to_string(), "10.1.1.1");
        assert_eq!(ips[1].to_string(), "10.2.2.2");

        // Should not have raw directive
        assert!(
            config.allow_update_raw.is_none(),
            "Should not have raw directive when only IPs present"
        );
    }

    #[test]
    fn test_parse_allow_update_with_multiple_keys() {
        // Test parsing allow-update with multiple key references
        let input = r#"zone "test.com" { type primary; file "/var/cache/bind/test.com.zone"; allow-update { key "key1"; key "key2"; }; };"#;

        let config = parse_showzone(input).unwrap();

        // Should capture raw directive
        assert!(
            config.allow_update_raw.is_some(),
            "Should capture raw directive"
        );
        let raw = config.allow_update_raw.as_ref().unwrap();
        assert!(raw.contains("key1"), "Raw should contain first key");
        assert!(raw.contains("key2"), "Raw should contain second key");
    }

    // ========== Serialization Tests ==========

    #[test]
    fn test_serialize_key_based_allow_update() {
        // Test that serialization prefers raw directive over empty IP list
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.file = Some("/var/cache/bind/test.com.zone".to_string());
        config.allow_update_raw = Some(r#"{ key "bindy-operator"; };"#.to_string());

        let serialized = config.to_rndc_block();

        // Should contain the raw directive
        assert!(
            serialized.contains("allow-update"),
            "Should include allow-update"
        );
        assert!(serialized.contains("key"), "Should include key keyword");
        assert!(
            serialized.contains("bindy-operator"),
            "Should include key name"
        );
    }

    #[test]
    fn test_serialize_prefer_raw_over_empty_ips() {
        // Test that raw directive is used even if IP list is empty
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.file = Some("/var/cache/bind/test.com.zone".to_string());
        config.allow_update = Some(Vec::new()); // Empty IP list
        config.allow_update_raw = Some(r#"{ key "mykey"; };"#.to_string());

        let serialized = config.to_rndc_block();

        // Should use raw directive, not empty IP list
        assert!(
            serialized.contains("allow-update"),
            "Should include allow-update from raw"
        );
        assert!(serialized.contains("key"), "Should use raw directive");
    }

    #[test]
    fn test_serialize_ips_when_no_raw() {
        // Test that IP list is serialized when raw directive is not present
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.file = Some("/var/cache/bind/test.com.zone".to_string());
        config.allow_update = Some(vec![
            "10.1.1.1".parse().unwrap(),
            "10.2.2.2".parse().unwrap(),
        ]);
        config.allow_update_raw = None;

        let serialized = config.to_rndc_block();

        // Should serialize IP list
        assert!(
            serialized.contains("allow-update"),
            "Should include allow-update"
        );
        assert!(serialized.contains("10.1.1.1"), "Should include first IP");
        assert!(serialized.contains("10.2.2.2"), "Should include second IP");
        assert!(!serialized.contains("key"), "Should not include 'key' keyword");
    }

    #[test]
    fn test_serialize_no_double_semicolon() {
        // Test that raw directive doesn't create double semicolons
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.file = Some("/var/cache/bind/test.com.zone".to_string());
        config.allow_update_raw = Some(r#"{ key "bindy-operator"; };"#.to_string());
        config.also_notify = Some(vec!["10.1.1.1".parse().unwrap()]);

        let serialized = config.to_rndc_block();

        // Should not have double semicolons
        assert!(!serialized.contains(";;"), "Should not have ';;' anywhere: {}", serialized);

        // Should have proper format
        assert!(serialized.contains("allow-update { key \"bindy-operator\"; }"),
                "Should have properly formatted allow-update: {}", serialized);
    }

    // ========== PATCH Operation Tests ==========

    #[test]
    fn test_patch_preserves_key_when_modifying_allow_transfer() {
        // PATCH only allow-transfer, preserve key-based allow-update
        let showzone_output = r#"zone "example.ca" { type primary; file "/var/cache/bind/example.ca.zone"; allow-transfer  { 10.244.1.27/32; }; allow-update { key "bindy-operator"; }; also-notify { 10.244.1.27; }; };"#;

        let mut config = parse_showzone(showzone_output).unwrap();

        // Verify raw directive was captured
        assert!(
            config.allow_update_raw.is_some(),
            "Should have raw allow-update directive"
        );

        // Simulate PATCH: update allow-transfer only
        config.allow_transfer = Some(vec!["10.244.1.28".parse().unwrap()]);

        // Serialize back
        let serialized = config.to_rndc_block();

        // Verify allow-update is still present
        assert!(
            serialized.contains("allow-update"),
            "Should preserve allow-update"
        );
        assert!(serialized.contains("key"), "Should preserve key reference");
        assert!(
            serialized.contains("bindy-operator"),
            "Should preserve key name"
        );

        // Verify allow-transfer was updated
        assert!(
            serialized.contains("10.244.1.28"),
            "Should have new allow-transfer IP"
        );
    }

    #[test]
    fn test_patch_preserves_key_when_modifying_also_notify() {
        // PATCH only also-notify, preserve key-based allow-update
        let showzone_output = r#"zone "example.ca" { type primary; file "/var/cache/bind/example.ca.zone"; allow-update { key "bindy-operator"; }; also-notify { 10.244.1.27; }; };"#;

        let mut config = parse_showzone(showzone_output).unwrap();

        // Verify raw directive was captured
        assert!(
            config.allow_update_raw.is_some(),
            "Should have raw allow-update directive"
        );

        // Simulate PATCH: update also-notify only
        config.also_notify = Some(vec!["10.244.1.99".parse().unwrap()]);

        // Serialize back
        let serialized = config.to_rndc_block();

        // Verify allow-update is still present
        assert!(
            serialized.contains("allow-update"),
            "Should preserve allow-update"
        );
        assert!(serialized.contains("key"), "Should preserve key reference");
        assert!(
            serialized.contains("bindy-operator"),
            "Should preserve key name"
        );

        // Verify also-notify was updated
        assert!(
            serialized.contains("10.244.1.99"),
            "Should have new also-notify IP"
        );
    }

    #[test]
    fn test_patch_preserves_key_when_modifying_both_transfer_and_notify() {
        // PATCH both allow-transfer and also-notify, preserve key-based allow-update
        let showzone_output = r#"zone "example.ca" { type primary; file "/var/cache/bind/example.ca.zone"; allow-transfer { 10.244.1.27; }; allow-update { key "bindy-operator"; }; also-notify { 10.244.1.27; }; };"#;

        let mut config = parse_showzone(showzone_output).unwrap();

        // Verify raw directive was captured
        assert!(
            config.allow_update_raw.is_some(),
            "Should have raw allow-update directive"
        );

        // Simulate PATCH: update both allow-transfer and also-notify
        config.allow_transfer = Some(vec!["10.244.1.100".parse().unwrap()]);
        config.also_notify = Some(vec!["10.244.1.101".parse().unwrap()]);

        // Serialize back
        let serialized = config.to_rndc_block();

        // Verify allow-update is still present
        assert!(
            serialized.contains("allow-update"),
            "Should preserve allow-update"
        );
        assert!(serialized.contains("key"), "Should preserve key reference");
        assert!(
            serialized.contains("bindy-operator"),
            "Should preserve key name"
        );

        // Verify both fields were updated
        assert!(
            serialized.contains("10.244.1.100"),
            "Should have new allow-transfer IP"
        );
        assert!(
            serialized.contains("10.244.1.101"),
            "Should have new also-notify IP"
        );
    }

    #[test]
    fn test_patch_replaces_key_when_modifying_allow_update() {
        // PATCH allow-transfer, also-notify, AND allow-update (with IPs), should replace key
        let showzone_output = r#"zone "example.ca" { type primary; file "/var/cache/bind/example.ca.zone"; allow-transfer { 10.244.1.27; }; allow-update { key "bindy-operator"; }; also-notify { 10.244.1.27; }; };"#;

        let mut config = parse_showzone(showzone_output).unwrap();

        // Verify raw directive was captured
        assert!(
            config.allow_update_raw.is_some(),
            "Should have raw allow-update directive"
        );

        // Simulate PATCH: update all three fields (replace key-based with IPs)
        config.allow_transfer = Some(vec!["10.244.1.200".parse().unwrap()]);
        config.also_notify = Some(vec!["10.244.1.201".parse().unwrap()]);
        config.allow_update = Some(vec!["10.244.1.202".parse().unwrap()]);
        config.allow_update_raw = None; // Clear raw when setting IPs

        // Serialize back
        let serialized = config.to_rndc_block();

        // Verify key is gone, IP is present
        assert!(
            serialized.contains("allow-update"),
            "Should have allow-update"
        );
        assert!(!serialized.contains("key"), "Should not have key reference");
        assert!(
            serialized.contains("10.244.1.202"),
            "Should have allow-update IP"
        );

        // Verify other fields were updated
        assert!(
            serialized.contains("10.244.1.200"),
            "Should have allow-transfer IP"
        );
        assert!(
            serialized.contains("10.244.1.201"),
            "Should have also-notify IP"
        );
    }

    #[test]
    fn test_patch_clears_raw_when_setting_ips() {
        // Simulate PATCH operation that replaces key-based allow-update with IPs
        let showzone_output = r#"zone "example.ca" { type primary; file "/var/cache/bind/example.ca.zone"; allow-update { key "bindy-operator"; }; };"#;

        let mut config = parse_showzone(showzone_output).unwrap();

        // Verify raw directive was captured
        assert!(
            config.allow_update_raw.is_some(),
            "Should have raw allow-update directive"
        );

        // Simulate PATCH: replace with IP list
        config.allow_update = Some(vec!["10.1.1.1".parse().unwrap()]);
        config.allow_update_raw = None; // Clear raw when setting IPs

        // Serialize back
        let serialized = config.to_rndc_block();

        // Verify key is gone, IP is present
        assert!(serialized.contains("allow-update"), "Should have allow-update");
        assert!(!serialized.contains("key"), "Should not have key reference");
        assert!(serialized.contains("10.1.1.1"), "Should have IP address");
    }

    // ========== Round-trip Tests ==========

    #[test]
    fn test_roundtrip() {
        let input = r#"zone "example.com" { type primary; file "/var/cache/bind/example.com.zone"; also-notify { 10.0.0.1; }; allow-transfer { 10.0.0.2; }; };"#;
        let config = parse_showzone(input).unwrap();
        let serialized = format!("zone \"{}\" {}", config.zone_name, config.to_rndc_block());
        let config2 = parse_showzone(&serialized).unwrap();
        assert_eq!(config.zone_type, config2.zone_type);
        assert_eq!(config.file, config2.file);
        assert_eq!(config.also_notify, config2.also_notify);
    }

    #[test]
    fn test_roundtrip_with_key_based_allow_update() {
        // Test that key-based allow-update is preserved during round-trip
        let input = r#"zone "example.com" { type primary; file "/var/cache/bind/example.com.zone"; allow-update { key "bindy-operator"; }; also-notify { 10.244.1.27; }; };"#;

        let config = parse_showzone(input).unwrap();

        // Verify raw directive was captured
        assert!(config.allow_update_raw.is_some());

        // Serialize back to RNDC format
        let serialized = config.to_rndc_block();

        // Verify the serialized output contains the allow-update directive
        assert!(serialized.contains("allow-update"), "Serialized config should contain allow-update");
        assert!(serialized.contains("key"), "Serialized config should preserve key reference");
        assert!(serialized.contains("bindy-operator"), "Serialized config should preserve key name");
    }

    // ========== Real-World Production Tests ==========

    #[test]
    fn test_parse_real_world_output() {
        // Real output from BIND9 with CIDR notation and key-based allow-update
        let input = r#"zone "internal.local" { type primary; file "/var/cache/bind/internal.local.zone"; allow-transfer  { 10.244.1.18/32; 10.244.1.21/32; }; allow-update { key "bindy-operator"; }; also-notify { 10.244.1.18; 10.244.1.21; }; };"#;
        let config = parse_showzone(input).unwrap();

        assert_eq!(config.zone_name, "internal.local");
        assert_eq!(config.zone_type, ZoneType::Primary);
        assert_eq!(config.file, Some("/var/cache/bind/internal.local.zone".to_string()));

        // Verify allow-transfer parsed correctly (CIDR notation stripped)
        assert_eq!(config.allow_transfer.as_ref().unwrap().len(), 2);
        assert_eq!(config.allow_transfer.as_ref().unwrap()[0], "10.244.1.18".parse::<IpAddr>().unwrap());
        assert_eq!(config.allow_transfer.as_ref().unwrap()[1], "10.244.1.21".parse::<IpAddr>().unwrap());

        // Verify allow-update parsed (key references are captured in raw)
        if let Some(allow_update) = &config.allow_update {
            assert_eq!(allow_update.len(), 0, "Should not extract IPs from key-based allow-update");
        }

        // Verify also-notify parsed correctly
        assert_eq!(config.also_notify.as_ref().unwrap().len(), 2);
        assert_eq!(config.also_notify.as_ref().unwrap()[0], "10.244.1.18".parse::<IpAddr>().unwrap());
        assert_eq!(config.also_notify.as_ref().unwrap()[1], "10.244.1.21".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_parse_exact_production_output() {
        // Exact string from production environment
        let input = r#"zone "internal.local" { type primary; file "/var/cache/bind/internal.local.zone"; allow-transfer  { 10.244.1.18/32; 10.244.1.21/32; }; allow-update { key "bindy-operator"; }; also-notify { 10.244.1.18; 10.244.1.21; }; };"#;

        // Test parsing succeeds
        let result = parse_showzone(input);
        assert!(result.is_ok(), "Failed to parse production output: {:?}", result.err());

        let config = result.unwrap();

        // Verify all fields
        assert_eq!(config.zone_name, "internal.local");
        assert_eq!(config.zone_type, ZoneType::Primary);
        assert_eq!(config.file, Some("/var/cache/bind/internal.local.zone".to_string()));

        // Verify allow-transfer (CIDR /32 should be stripped)
        let allow_transfer = config.allow_transfer.as_ref().expect("allow-transfer should be present");
        assert_eq!(allow_transfer.len(), 2);
        assert_eq!(allow_transfer[0].to_string(), "10.244.1.18");
        assert_eq!(allow_transfer[1].to_string(), "10.244.1.21");

        // Verify also-notify
        let also_notify = config.also_notify.as_ref().expect("also-notify should be present");
        assert_eq!(also_notify.len(), 2);
        assert_eq!(also_notify[0].to_string(), "10.244.1.18");
        assert_eq!(also_notify[1].to_string(), "10.244.1.21");

        // Verify allow-update raw directive is captured (key-based updates)
        assert!(config.allow_update_raw.is_some(), "allow-update-raw should be present for key-based updates");
        assert!(config.allow_update_raw.as_ref().unwrap().contains("key"), "Raw directive should contain 'key' keyword");
        assert!(config.allow_update_raw.as_ref().unwrap().contains("bindy-operator"), "Raw directive should contain key name");
    }

    #[test]
    fn test_exact_production_scenario() {
        // This is the exact scenario from the bug report
        let showzone_output = r#"zone "example.ca" { type primary; file "/var/cache/bind/example.ca.zone"; allow-transfer  { 10.244.1.27/32; }; allow-update { key "bindy-operator"; }; also-notify { 10.244.1.27; }; };"#;

        // Parse the zone
        let mut config = parse_showzone(showzone_output).unwrap();

        // Verify we captured everything
        assert_eq!(config.zone_name, "example.ca");
        assert_eq!(config.zone_type, ZoneType::Primary);
        assert!(
            config.allow_update_raw.is_some(),
            "Must capture raw allow-update"
        );
        assert!(config.allow_transfer.is_some(), "Must capture allow-transfer");
        assert!(config.also_notify.is_some(), "Must capture also-notify");

        // Simulate PATCH: only update allow-transfer
        config.allow_transfer = Some(vec!["10.244.1.27".parse().unwrap()]);
        // NOTE: We do NOT touch allow_update or allow_update_raw

        // Serialize for modzone
        let modzone_config = config.to_rndc_block();

        // CRITICAL: Verify allow-update is preserved
        assert!(
            modzone_config.contains("allow-update"),
            "BUG: allow-update was lost!"
        );
        assert!(
            modzone_config.contains("key"),
            "BUG: key reference was lost!"
        );
        assert!(
            modzone_config.contains("bindy-operator"),
            "BUG: key name was lost!"
        );

        // Also verify other fields are correct
        assert!(modzone_config.contains("type primary"));
        assert!(modzone_config.contains("file"));
        assert!(modzone_config.contains("allow-transfer"));
        assert!(modzone_config.contains("also-notify"));
        assert!(modzone_config.contains("10.244.1.27"));
    }

    #[test]
    fn test_exact_modzone_format() {
        // Test the exact format that will be sent to BIND9 modzone
        let showzone_output = r#"zone "example.com" { type primary; file "/var/cache/bind/example.com.zone"; allow-transfer { 10.244.1.31; }; allow-update { key "bindy-operator"; }; also-notify { 10.244.1.31; }; };"#;

        let mut config = parse_showzone(showzone_output).unwrap();

        // Simulate PATCH: update also-notify
        config.also_notify = Some(vec!["10.244.1.31".parse().unwrap()]);

        let modzone_config = config.to_rndc_block();

        // The format should be: { type primary; file "..."; also-notify { ...; }; allow-transfer { ...; }; allow-update { key "..."; }; };
        // NOT: { ... allow-update { key "..."; };; };  (double semicolon)

        // Verify no syntax errors (double semicolons)
        assert!(!modzone_config.contains(";; }"), "Should not have ';; }}' at end: {}", modzone_config);
        assert!(!modzone_config.contains(";;"), "Should not have ';;' anywhere: {}", modzone_config);

        // Verify proper ending
        assert!(modzone_config.ends_with("};"), "Should end with '}};': {}", modzone_config);

        // Verify the allow-update section is properly formatted
        assert!(modzone_config.contains("allow-update { key \"bindy-operator\"; }"),
                "Should have properly formatted allow-update: {}", modzone_config);
    }

    // ========== Enhanced Features: Unknown Option Preservation ==========

    #[test]
    fn test_unknown_options_preserved() {
        let input = r#"zone "example.com" {
            type primary;
            file "/var/cache/bind/example.com.zone";
            zone-statistics yes;
            max-zone-ttl 86400;
        };"#;

        let config = parse_showzone(input).unwrap();

        assert_eq!(config.zone_name, "example.com");
        assert_eq!(config.zone_type, ZoneType::Primary);
        assert!(config.raw_options.contains_key("zone-statistics"));
        assert!(config.raw_options.contains_key("max-zone-ttl"));
    }

    #[test]
    fn test_unknown_options_with_braces() {
        let input = r#"zone "example.com" {
            type primary;
            file "/var/cache/bind/example.com.zone";
            update-policy { grant example.com. zonesub any; };
        };"#;

        let config = parse_showzone(input).unwrap();

        assert_eq!(config.zone_name, "example.com");
        assert!(config.raw_options.contains_key("update-policy"));
        let update_policy = config.raw_options.get("update-policy").unwrap();
        assert!(update_policy.contains("grant"));
        assert!(update_policy.contains("zonesub"));
    }

    #[test]
    fn test_roundtrip_with_unknown_options() {
        let input = r#"zone "example.com" {
            type primary;
            file "/var/cache/bind/example.com.zone";
            zone-statistics full;
            check-names warn;
        };"#;

        let config = parse_showzone(input).unwrap();
        let serialized = config.to_rndc_block();

        // Verify essential fields are preserved
        assert!(serialized.contains("type primary"));
        assert!(serialized.contains(r#"file "/var/cache/bind/example.com.zone""#));

        // Verify unknown options are preserved
        assert!(serialized.contains("zone-statistics"));
        assert!(serialized.contains("full"));
        assert!(serialized.contains("check-names"));
        assert!(serialized.contains("warn"));
    }

    #[test]
    fn test_complex_zone_with_multiple_unknowns() {
        let input = r#"zone "example.com" {
            type primary;
            file "/var/cache/bind/example.com.zone";
            allow-transfer { 10.1.1.1; 10.2.2.2; };
            also-notify { 10.3.3.3; };
            check-integrity yes;
            check-mx fail;
            dialup no;
            max-transfer-time-in 60;
        };"#;

        let config = parse_showzone(input).unwrap();

        // Known options should be parsed
        assert_eq!(config.allow_transfer.as_ref().unwrap().len(), 2);
        assert_eq!(config.also_notify.as_ref().unwrap().len(), 1);

        // Unknown options should be in raw_options
        assert!(config.raw_options.contains_key("check-integrity"));
        assert!(config.raw_options.contains_key("check-mx"));
        assert!(config.raw_options.contains_key("dialup"));
        assert!(config.raw_options.contains_key("max-transfer-time-in"));
    }

    #[test]
    fn test_serialize_preserves_order() {
        let input = r#"zone "example.com" {
            type primary;
            file "/var/cache/bind/example.com.zone";
            allow-transfer { 10.1.1.1; };
            zone-statistics yes;
        };"#;

        let config = parse_showzone(input).unwrap();
        let serialized = config.to_rndc_block();

        // Verify no double semicolons
        assert!(!serialized.contains(";;"));

        // Verify proper format
        assert!(serialized.starts_with("{ "));
        assert!(serialized.ends_with("; };"));
    }

    #[test]
    fn test_empty_raw_options() {
        let input = r#"zone "example.com" {
            type primary;
            file "/var/cache/bind/example.com.zone";
        };"#;

        let config = parse_showzone(input).unwrap();

        assert_eq!(config.raw_options.len(), 0);

        let serialized = config.to_rndc_block();
        assert!(serialized.contains("type primary"));
        assert!(!serialized.contains(";;"));
    }

    #[test]
    fn test_mixed_known_and_unknown_options() {
        let input = r#"zone "example.com" {
            type secondary;
            file "/var/cache/bind/example.com.zone";
            primaries { 192.168.1.1; 192.168.1.2 port 5353; };
            max-refresh-time 3600;
            min-retry-time 600;
            request-ixfr yes;
        };"#;

        let config = parse_showzone(input).unwrap();

        // Known options
        assert_eq!(config.zone_type, ZoneType::Secondary);
        assert!(config.primaries.is_some());
        assert_eq!(config.primaries.as_ref().unwrap().len(), 2);

        // Verify port parsing
        assert_eq!(config.primaries.as_ref().unwrap()[1].port, Some(5353));

        // Unknown options
        assert!(config.raw_options.contains_key("max-refresh-time"));
        assert!(config.raw_options.contains_key("min-retry-time"));
        assert!(config.raw_options.contains_key("request-ixfr"));
    }

    #[test]
    fn test_serialize_raw_options_no_trailing_semicolons() {
        let mut config = ZoneConfig::new(
            "example.com".to_string(),
            ZoneType::Primary,
        );
        config.file = Some("/var/cache/bind/example.com.zone".to_string());
        config.raw_options.insert("zone-statistics".to_string(), "yes".to_string());
        config.raw_options.insert(
            "check-names".to_string(),
            "warn".to_string(),
        );

        let serialized = config.to_rndc_block();

        // Should not have double semicolons
        assert!(!serialized.contains(";;"));

        // Should contain the options
        assert!(serialized.contains("zone-statistics"));
        assert!(serialized.contains("check-names"));
    }

    #[test]
    fn test_unknown_option_with_quoted_value() {
        let input = r#"zone "example.com" {
            type primary;
            file "/var/cache/bind/example.com.zone";
            journal "/var/lib/bind/journal/example.com.jnl";
        };"#;

        let config = parse_showzone(input).unwrap();

        assert!(config.raw_options.contains_key("journal"));
        let journal = config.raw_options.get("journal").unwrap();
        assert!(journal.contains("/var/lib/bind/journal/example.com.jnl"));
    }

    #[test]
    fn test_parse_and_reserialize_preserves_functionality() {
        let original = r#"zone "test.com" {
            type primary;
            file "/var/cache/bind/test.com.zone";
            allow-transfer { 10.1.1.1; 10.2.2.2; };
            also-notify { 10.3.3.3; };
            allow-update { key "mykey"; };
            check-mx warn;
            zone-statistics full;
        };"#;

        let config = parse_showzone(original).unwrap();
        let serialized = config.to_rndc_block();

        // Parse the serialized version
        // Note: We don't have a full zone statement parser, so we'll just verify format
        assert!(serialized.contains("type primary"));
        assert!(serialized.contains("allow-transfer"));
        assert!(serialized.contains("also-notify"));
        assert!(serialized.contains("allow-update"));
        assert!(serialized.contains("check-mx"));
        assert!(serialized.contains("zone-statistics"));

        // Verify key-based allow-update is preserved
        assert!(serialized.contains("key"));
        assert!(serialized.contains("mykey"));
    }

    #[test]
    fn test_many_unknown_options() {
        let input = r#"zone "example.com" {
            type primary;
            file "/var/cache/bind/example.com.zone";
            option1 value1;
            option2 value2;
            option3 value3;
            option4 value4;
            option5 value5;
        };"#;

        let config = parse_showzone(input).unwrap();

        assert_eq!(config.raw_options.len(), 5);
        assert!(config.raw_options.contains_key("option1"));
        assert!(config.raw_options.contains_key("option2"));
        assert!(config.raw_options.contains_key("option3"));
        assert!(config.raw_options.contains_key("option4"));
        assert!(config.raw_options.contains_key("option5"));

        let serialized = config.to_rndc_block();
        assert!(serialized.contains("option1"));
        assert!(serialized.contains("option5"));
    }
}
