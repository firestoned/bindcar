// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Tests for RNDC types and zone configuration structures

#[cfg(test)]
mod tests {
    use crate::rndc_types::*;

    // ========== Enum Tests ==========

    #[test]
    fn test_dns_class_as_str() {
        assert_eq!(DnsClass::IN.as_str(), "IN");
        assert_eq!(DnsClass::CH.as_str(), "CH");
        assert_eq!(DnsClass::HS.as_str(), "HS");
    }

    #[test]
    fn test_dns_class_default() {
        let class: DnsClass = Default::default();
        assert_eq!(class, DnsClass::IN);
    }

    #[test]
    fn test_zone_type_as_str() {
        assert_eq!(ZoneType::Primary.as_str(), "primary");
        assert_eq!(ZoneType::Secondary.as_str(), "secondary");
        assert_eq!(ZoneType::Stub.as_str(), "stub");
        assert_eq!(ZoneType::Forward.as_str(), "forward");
        assert_eq!(ZoneType::Hint.as_str(), "hint");
        assert_eq!(ZoneType::Mirror.as_str(), "mirror");
        assert_eq!(ZoneType::Delegation.as_str(), "delegation-only");
        assert_eq!(ZoneType::Redirect.as_str(), "redirect");
    }

    #[test]
    fn test_zone_type_parse_modern() {
        assert_eq!(ZoneType::parse("primary"), Some(ZoneType::Primary));
        assert_eq!(ZoneType::parse("secondary"), Some(ZoneType::Secondary));
        assert_eq!(ZoneType::parse("stub"), Some(ZoneType::Stub));
        assert_eq!(ZoneType::parse("forward"), Some(ZoneType::Forward));
        assert_eq!(ZoneType::parse("hint"), Some(ZoneType::Hint));
        assert_eq!(ZoneType::parse("mirror"), Some(ZoneType::Mirror));
        assert_eq!(ZoneType::parse("delegation-only"), Some(ZoneType::Delegation));
        assert_eq!(ZoneType::parse("redirect"), Some(ZoneType::Redirect));
    }

    #[test]
    fn test_zone_type_parse_legacy() {
        assert_eq!(ZoneType::parse("master"), Some(ZoneType::Primary));
        assert_eq!(ZoneType::parse("slave"), Some(ZoneType::Secondary));
    }

    #[test]
    fn test_zone_type_parse_invalid() {
        assert_eq!(ZoneType::parse("invalid"), None);
        assert_eq!(ZoneType::parse(""), None);
    }

    #[test]
    fn test_notify_mode_as_str() {
        assert_eq!(NotifyMode::Yes.as_str(), "yes");
        assert_eq!(NotifyMode::No.as_str(), "no");
        assert_eq!(NotifyMode::Explicit.as_str(), "explicit");
        assert_eq!(NotifyMode::MasterOnly.as_str(), "master-only");
        assert_eq!(NotifyMode::PrimaryOnly.as_str(), "primary-only");
    }

    #[test]
    fn test_notify_mode_parse() {
        assert_eq!(NotifyMode::parse("yes"), Some(NotifyMode::Yes));
        assert_eq!(NotifyMode::parse("no"), Some(NotifyMode::No));
        assert_eq!(NotifyMode::parse("explicit"), Some(NotifyMode::Explicit));
        assert_eq!(NotifyMode::parse("master-only"), Some(NotifyMode::MasterOnly));
        assert_eq!(NotifyMode::parse("primary-only"), Some(NotifyMode::PrimaryOnly));
        assert_eq!(NotifyMode::parse("invalid"), None);
    }

    #[test]
    fn test_forward_mode_as_str() {
        assert_eq!(ForwardMode::Only.as_str(), "only");
        assert_eq!(ForwardMode::First.as_str(), "first");
    }

    #[test]
    fn test_forward_mode_parse() {
        assert_eq!(ForwardMode::parse("only"), Some(ForwardMode::Only));
        assert_eq!(ForwardMode::parse("first"), Some(ForwardMode::First));
        assert_eq!(ForwardMode::parse("invalid"), None);
    }

    #[test]
    fn test_auto_dnssec_mode_as_str() {
        assert_eq!(AutoDnssecMode::Off.as_str(), "off");
        assert_eq!(AutoDnssecMode::Maintain.as_str(), "maintain");
        assert_eq!(AutoDnssecMode::Create.as_str(), "create");
    }

    #[test]
    fn test_auto_dnssec_mode_parse() {
        assert_eq!(AutoDnssecMode::parse("off"), Some(AutoDnssecMode::Off));
        assert_eq!(AutoDnssecMode::parse("maintain"), Some(AutoDnssecMode::Maintain));
        assert_eq!(AutoDnssecMode::parse("create"), Some(AutoDnssecMode::Create));
        assert_eq!(AutoDnssecMode::parse("invalid"), None);
    }

    #[test]
    fn test_check_names_mode_as_str() {
        assert_eq!(CheckNamesMode::Fail.as_str(), "fail");
        assert_eq!(CheckNamesMode::Warn.as_str(), "warn");
        assert_eq!(CheckNamesMode::Ignore.as_str(), "ignore");
    }

    #[test]
    fn test_check_names_mode_parse() {
        assert_eq!(CheckNamesMode::parse("fail"), Some(CheckNamesMode::Fail));
        assert_eq!(CheckNamesMode::parse("warn"), Some(CheckNamesMode::Warn));
        assert_eq!(CheckNamesMode::parse("ignore"), Some(CheckNamesMode::Ignore));
        assert_eq!(CheckNamesMode::parse("invalid"), None);
    }

    #[test]
    fn test_masterfile_format_as_str() {
        assert_eq!(MasterfileFormat::Text.as_str(), "text");
        assert_eq!(MasterfileFormat::Raw.as_str(), "raw");
        assert_eq!(MasterfileFormat::Map.as_str(), "map");
    }

    #[test]
    fn test_masterfile_format_parse() {
        assert_eq!(MasterfileFormat::parse("text"), Some(MasterfileFormat::Text));
        assert_eq!(MasterfileFormat::parse("raw"), Some(MasterfileFormat::Raw));
        assert_eq!(MasterfileFormat::parse("map"), Some(MasterfileFormat::Map));
        assert_eq!(MasterfileFormat::parse("invalid"), None);
    }

    // ========== Struct Tests ==========

    #[test]
    fn test_primary_spec_new() {
        let addr = "192.168.1.1".parse().unwrap();
        let spec = PrimarySpec::new(addr);

        assert_eq!(spec.address, addr);
        assert_eq!(spec.port, None);
    }

    #[test]
    fn test_primary_spec_with_port() {
        let addr = "192.168.1.1".parse().unwrap();
        let spec = PrimarySpec::with_port(addr, 5353);

        assert_eq!(spec.address, addr);
        assert_eq!(spec.port, Some(5353));
    }

    #[test]
    fn test_forwarder_spec_new() {
        let addr = "8.8.8.8".parse().unwrap();
        let spec = ForwarderSpec::new(addr);

        assert_eq!(spec.address, addr);
        assert_eq!(spec.port, None);
        assert_eq!(spec.tls_config, None);
    }

    #[test]
    fn test_forwarder_spec_with_port() {
        let addr = "8.8.8.8".parse().unwrap();
        let spec = ForwarderSpec::with_port(addr, 853);

        assert_eq!(spec.address, addr);
        assert_eq!(spec.port, Some(853));
        assert_eq!(spec.tls_config, None);
    }

    #[test]
    fn test_forwarder_spec_with_tls() {
        let addr = "8.8.8.8".parse().unwrap();
        let spec = ForwarderSpec::with_tls(addr, "tls-config".to_string());

        assert_eq!(spec.address, addr);
        assert_eq!(spec.port, None);
        assert_eq!(spec.tls_config, Some("tls-config".to_string()));
    }

    #[test]
    fn test_zone_config_new() {
        let config = ZoneConfig::new("example.com".to_string(), ZoneType::Primary);

        assert_eq!(config.zone_name, "example.com");
        assert_eq!(config.zone_type, ZoneType::Primary);
        assert_eq!(config.class, DnsClass::IN);
        assert_eq!(config.file, None);
        assert_eq!(config.primaries, None);
        assert_eq!(config.also_notify, None);
        assert_eq!(config.raw_options.len(), 0);
    }

    // ========== Serialization Tests ==========

    #[test]
    fn test_to_rndc_block_minimal() {
        let config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        let block = config.to_rndc_block();

        assert!(block.contains("type primary"));
        assert!(block.starts_with("{ "));
        assert!(block.ends_with("; };"));
    }

    #[test]
    fn test_to_rndc_block_with_file() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.file = Some("/var/cache/bind/test.com.zone".to_string());

        let block = config.to_rndc_block();

        assert!(block.contains("type primary"));
        assert!(block.contains(r#"file "/var/cache/bind/test.com.zone""#));
    }

    #[test]
    fn test_to_rndc_block_with_primaries() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Secondary);
        config.primaries = Some(vec![
            PrimarySpec::new("192.168.1.1".parse().unwrap()),
            PrimarySpec::with_port("192.168.1.2".parse().unwrap(), 5353),
        ]);

        let block = config.to_rndc_block();

        assert!(block.contains("type secondary"));
        assert!(block.contains("primaries { 192.168.1.1; 192.168.1.2 port 5353; }"));
    }

    #[test]
    fn test_to_rndc_block_with_also_notify() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.also_notify = Some(vec![
            "10.0.0.1".parse().unwrap(),
            "10.0.0.2".parse().unwrap(),
        ]);

        let block = config.to_rndc_block();

        assert!(block.contains("also-notify { 10.0.0.1; 10.0.0.2; }"));
    }

    #[test]
    fn test_to_rndc_block_with_allow_transfer() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.allow_transfer = Some(vec![
            "10.1.1.1".parse().unwrap(),
        ]);

        let block = config.to_rndc_block();

        assert!(block.contains("allow-transfer { 10.1.1.1; }"));
    }

    #[test]
    fn test_to_rndc_block_with_allow_update_ips() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.allow_update = Some(vec![
            "10.2.2.2".parse().unwrap(),
        ]);

        let block = config.to_rndc_block();

        assert!(block.contains("allow-update { 10.2.2.2; }"));
    }

    #[test]
    fn test_to_rndc_block_with_allow_update_raw() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.allow_update_raw = Some(r#"{ key "update-key"; };"#.to_string());

        let block = config.to_rndc_block();

        assert!(block.contains("allow-update { key \"update-key\"; }"));
        assert!(!block.contains(";;"));  // No double semicolons
    }

    #[test]
    fn test_to_rndc_block_prefers_raw_over_ips() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.allow_update = Some(vec!["10.1.1.1".parse().unwrap()]);
        config.allow_update_raw = Some(r#"{ key "mykey"; };"#.to_string());

        let block = config.to_rndc_block();

        // Raw should be used
        assert!(block.contains("allow-update { key \"mykey\"; }"));
        // IP should not appear
        assert!(!block.contains("10.1.1.1"));
    }

    #[test]
    fn test_to_rndc_block_with_notify_mode() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.notify = Some(NotifyMode::Explicit);

        let block = config.to_rndc_block();

        assert!(block.contains("notify explicit"));
    }

    #[test]
    fn test_to_rndc_block_with_forwarders() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Forward);
        config.forwarders = Some(vec![
            ForwarderSpec::new("8.8.8.8".parse().unwrap()),
            ForwarderSpec::with_port("8.8.4.4".parse().unwrap(), 853),
        ]);

        let block = config.to_rndc_block();

        assert!(block.contains("forwarders { 8.8.8.8; 8.8.4.4 port 853; }"));
    }

    #[test]
    fn test_to_rndc_block_with_raw_options() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.raw_options.insert("zone-statistics".to_string(), "full".to_string());
        config.raw_options.insert("check-names".to_string(), "warn".to_string());

        let block = config.to_rndc_block();

        assert!(block.contains("zone-statistics full"));
        assert!(block.contains("check-names warn"));
        assert!(!block.contains(";;"));
    }

    #[test]
    fn test_to_rndc_block_raw_options_no_double_semicolons() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.raw_options.insert("option1".to_string(), "value1;".to_string());  // Has trailing semicolon
        config.raw_options.insert("option2".to_string(), "value2".to_string());   // No trailing semicolon

        let block = config.to_rndc_block();

        // Should not have double semicolons
        assert!(!block.contains(";;"));
        assert!(block.contains("option1 value1"));
        assert!(block.contains("option2 value2"));
    }

    #[test]
    fn test_to_rndc_block_comprehensive() {
        let mut config = ZoneConfig::new("example.com".to_string(), ZoneType::Primary);
        config.file = Some("/var/cache/bind/example.com.zone".to_string());
        config.also_notify = Some(vec!["10.1.1.1".parse().unwrap()]);
        config.allow_transfer = Some(vec!["10.2.2.2".parse().unwrap()]);
        config.allow_update_raw = Some(r#"{ key "mykey"; };"#.to_string());
        config.notify = Some(NotifyMode::Yes);
        config.max_transfer_time_in = Some(3600);
        config.inline_signing = Some(true);
        config.check_names = Some(CheckNamesMode::Warn);
        config.raw_options.insert("zone-statistics".to_string(), "yes".to_string());

        let block = config.to_rndc_block();

        assert!(block.contains("type primary"));
        assert!(block.contains(r#"file "/var/cache/bind/example.com.zone""#));
        assert!(block.contains("also-notify { 10.1.1.1; }"));
        assert!(block.contains("allow-transfer { 10.2.2.2; }"));
        assert!(block.contains("allow-update { key \"mykey\"; }"));
        assert!(block.contains("notify yes"));
        assert!(block.contains("max-transfer-time-in 3600"));
        assert!(block.contains("inline-signing yes"));
        assert!(block.contains("check-names warn"));
        assert!(block.contains("zone-statistics yes"));
        assert!(!block.contains(";;"));
    }

    #[test]
    fn test_zone_config_equality() {
        let config1 = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        let config2 = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);

        assert_eq!(config1, config2);
    }

    #[test]
    fn test_zone_config_inequality_zone_name() {
        let config1 = ZoneConfig::new("test1.com".to_string(), ZoneType::Primary);
        let config2 = ZoneConfig::new("test2.com".to_string(), ZoneType::Primary);

        assert_ne!(config1, config2);
    }

    #[test]
    fn test_zone_config_inequality_zone_type() {
        let config1 = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        let config2 = ZoneConfig::new("test.com".to_string(), ZoneType::Secondary);

        assert_ne!(config1, config2);
    }

    #[test]
    fn test_zone_config_clone() {
        let mut config1 = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config1.file = Some("/var/cache/bind/test.com.zone".to_string());
        config1.raw_options.insert("test".to_string(), "value".to_string());

        let config2 = config1.clone();

        assert_eq!(config1, config2);
        assert_eq!(config2.file, Some("/var/cache/bind/test.com.zone".to_string()));
        assert_eq!(config2.raw_options.get("test"), Some(&"value".to_string()));
    }

    #[test]
    fn test_boolean_serialization_true() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.inline_signing = Some(true);
        config.check_integrity = Some(true);
        config.multi_master = Some(true);

        let block = config.to_rndc_block();

        assert!(block.contains("inline-signing yes"));
        assert!(block.contains("check-integrity yes"));
        assert!(block.contains("multi-master yes"));
    }

    #[test]
    fn test_boolean_serialization_false() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.inline_signing = Some(false);
        config.check_integrity = Some(false);
        config.multi_master = Some(false);

        let block = config.to_rndc_block();

        assert!(block.contains("inline-signing no"));
        assert!(block.contains("check-integrity no"));
        assert!(block.contains("multi-master no"));
    }

    #[test]
    fn test_empty_collections_not_serialized() {
        let mut config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);
        config.also_notify = Some(vec![]);
        config.allow_transfer = Some(vec![]);
        config.allow_update = Some(vec![]);

        let block = config.to_rndc_block();

        // Empty collections should not be serialized
        assert!(!block.contains("also-notify"));
        assert!(!block.contains("allow-transfer"));
        assert!(!block.contains("allow-update"));
    }

    #[test]
    fn test_empty_raw_options_not_serialized() {
        let config = ZoneConfig::new("test.com".to_string(), ZoneType::Primary);

        let block = config.to_rndc_block();

        // Should only contain type
        assert!(block.contains("type primary"));
        // Should be minimal
        assert_eq!(block, "{ type primary; };");
    }
}
