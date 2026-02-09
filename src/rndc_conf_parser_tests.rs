use super::*;

#[test]
fn test_line_comment() {
    assert!(line_comment("// comment\n").is_ok());
    assert!(line_comment("// comment").is_ok());
}

#[test]
fn test_hash_comment() {
    assert!(hash_comment("# comment\n").is_ok());
}

#[test]
fn test_block_comment() {
    assert!(block_comment("/* comment */").is_ok());
    assert!(block_comment("/* multi\nline */").is_ok());
}

#[test]
fn test_quoted_string() {
    assert_eq!(quoted_string(r#""hello""#).unwrap().1, "hello");
    assert_eq!(quoted_string(r#""hello world""#).unwrap().1, "hello world");
}

#[test]
fn test_quoted_string_with_escapes() {
    assert_eq!(
        quoted_string(r#""hello\"world""#).unwrap().1,
        "hello\"world"
    );
    assert_eq!(
        quoted_string(r#""line1\nline2""#).unwrap().1,
        "line1\nline2"
    );
}

#[test]
fn test_identifier() {
    assert_eq!(identifier("hmac-sha256").unwrap().1, "hmac-sha256");
    assert_eq!(identifier("rndc-key").unwrap().1, "rndc-key");
    assert_eq!(identifier("localhost").unwrap().1, "localhost");
}

#[test]
fn test_ipv4_addr() {
    let result = ipv4_addr("192.168.1.1").unwrap().1;
    assert_eq!(result, "192.168.1.1".parse::<IpAddr>().unwrap());
}

#[test]
fn test_ipv6_addr() {
    let result = ipv6_addr("2001:db8::1").unwrap().1;
    assert_eq!(result, "2001:db8::1".parse::<IpAddr>().unwrap());
}

#[test]
fn test_port_number() {
    assert_eq!(port_number("953").unwrap().1, 953);
    assert_eq!(port_number("8080").unwrap().1, 8080);
}

#[test]
fn test_parse_key_block() {
    let input = r#"key "rndc-key" {
        algorithm hmac-sha256;
        secret "dGVzdC1zZWNyZXQ=";
    };"#;

    let (_, (name, key)) = parse_key_block(input).unwrap();
    assert_eq!(name, "rndc-key");
    assert_eq!(key.algorithm, "hmac-sha256");
    assert_eq!(key.secret, "dGVzdC1zZWNyZXQ=");
}

#[test]
fn test_parse_server_block() {
    let input = r#"server localhost {
        key "rndc-key";
        port 953;
    };"#;

    let (_, (addr, server)) = parse_server_block(input).unwrap();
    assert_eq!(addr, "localhost");
    assert_eq!(server.key, Some("rndc-key".to_string()));
    assert_eq!(server.port, Some(953));
}

#[test]
fn test_parse_options_block() {
    let input = r#"options {
        default-server localhost;
        default-key "rndc-key";
        default-port 953;
    };"#;

    let (_, options) = parse_options_block(input).unwrap();
    assert_eq!(options.default_server, Some("localhost".to_string()));
    assert_eq!(options.default_key, Some("rndc-key".to_string()));
    assert_eq!(options.default_port, Some(953));
}

#[test]
fn test_parse_include_stmt() {
    let input = r#"include "/etc/bind/rndc.key";"#;
    let (_, path) = parse_include_stmt(input).unwrap();
    assert_eq!(path, PathBuf::from("/etc/bind/rndc.key"));
}

#[test]
fn test_parse_complete_conf() {
    let input = r#"
    # Example rndc.conf
    include "/etc/bind/rndc.key";

    key "rndc-key" {
        algorithm hmac-sha256;
        secret "dGVzdC1zZWNyZXQ=";
    };

    server localhost {
        key "rndc-key";
        port 953;
    };

    options {
        default-server localhost;
        default-key "rndc-key";
    };
    "#;

    let conf = parse_rndc_conf_str(input).unwrap();
    assert_eq!(conf.keys.len(), 1);
    assert_eq!(conf.servers.len(), 1);
    assert_eq!(conf.includes.len(), 1);
    assert_eq!(conf.options.default_server, Some("localhost".to_string()));
}

#[test]
fn test_parse_with_comments() {
    let input = r#"
    // Line comment
    # Hash comment
    /* Block comment */
    key "test-key" {
        algorithm hmac-sha256; // inline comment
        secret "secret"; # another comment
    };
    "#;

    let conf = parse_rndc_conf_str(input).unwrap();
    assert_eq!(conf.keys.len(), 1);
}

#[test]
fn test_roundtrip() {
    let input = r#"
    key "rndc-key" {
        algorithm hmac-sha256;
        secret "dGVzdC1zZWNyZXQ=";
    };

    options {
        default-server localhost;
        default-key "rndc-key";
    };
    "#;

    let conf = parse_rndc_conf_str(input).unwrap();
    let serialized = conf.to_conf_file();
    let conf2 = parse_rndc_conf_str(&serialized).unwrap();

    assert_eq!(conf.keys.len(), conf2.keys.len());
    assert_eq!(conf.options.default_server, conf2.options.default_server);
}

// Error handling tests
#[test]
fn test_parse_empty_input() {
    let input = "";
    let result = parse_rndc_conf_str(input);
    assert!(result.is_ok());
    let conf = result.unwrap();
    assert_eq!(conf.keys.len(), 0);
}

#[test]
fn test_parse_incomplete_key_block() {
    // Test with no secret field - should still parse with default
    let input = r#"key "test-key" { };"#;
    let result = parse_rndc_conf_str(input);
    assert!(result.is_ok());
    let conf = result.unwrap();
    assert!(conf.keys.contains_key("test-key"));
}

#[test]
fn test_parse_invalid_ip_address() {
    let input = "999.999.999.999";
    let result = ip_addr(input);
    assert!(result.is_err());
}

#[test]
fn test_parse_server_with_ipv6() {
    let input = r#"server 2001:db8::1 {
        key "rndc-key";
        port 953;
    };"#;

    let (_, (addr, server)) = parse_server_block(input).unwrap();
    assert!(addr.contains("2001:db8::1"));
    assert_eq!(server.key, Some("rndc-key".to_string()));
    assert_eq!(server.port, Some(953));
}

#[test]
fn test_parse_empty_options_block() {
    let input = r#"options { };"#;
    let (_, options) = parse_options_block(input).unwrap();
    assert!(options.is_empty());
}

#[test]
fn test_parse_key_block_without_algorithm() {
    let input = r#"key "test-key" {
        secret "dGVzdA==";
    };"#;

    let (_, (name, key)) = parse_key_block(input).unwrap();
    assert_eq!(name, "test-key");
    assert_eq!(key.algorithm, "hmac-sha256"); // Should default
    assert_eq!(key.secret, "dGVzdA==");
}

#[test]
fn test_parse_server_block_full() {
    // Test server block with key and port
    let input = r#"server 192.168.1.1 {
        key "test-key";
        port 8953;
    };"#;

    let (_, (addr, server)) = parse_server_block(input).unwrap();
    assert!(addr.contains("192.168.1.1"));
    assert_eq!(server.key, Some("test-key".to_string()));
    assert_eq!(server.port, Some(8953));
}

#[test]
fn test_parse_multiple_keys() {
    let input = r#"
    key "key1" {
        algorithm hmac-sha256;
        secret "secret1";
    };

    key "key2" {
        algorithm hmac-md5;
        secret "secret2";
    };
    "#;

    let conf = parse_rndc_conf_str(input).unwrap();
    assert_eq!(conf.keys.len(), 2);
    assert!(conf.keys.contains_key("key1"));
    assert!(conf.keys.contains_key("key2"));
}

#[test]
fn test_parse_multiple_servers() {
    let input = r#"
    server 127.0.0.1 {
        key "key1";
    };

    server localhost {
        key "key2";
        port 8953;
    };
    "#;

    let conf = parse_rndc_conf_str(input).unwrap();
    assert_eq!(conf.servers.len(), 2);
}

#[test]
fn test_error_display() {
    let error = RndcConfParseError::MissingField("algorithm".to_string());
    assert_eq!(error.to_string(), "Missing required field: algorithm");

    let error = RndcConfParseError::CircularInclude("/path/to/file".to_string());
    assert_eq!(
        error.to_string(),
        "Circular include detected: /path/to/file"
    );
}

// File-based parsing tests
#[test]
fn test_parse_file_not_found() {
    let result = parse_rndc_conf_file(Path::new("/nonexistent/path/rndc.conf"));
    assert!(result.is_err());
    match result {
        Err(RndcConfParseError::FileNotFound(_)) => {}
        _ => panic!("Expected FileNotFound error"),
    }
}

#[test]
fn test_parse_file_with_includes() {
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let main_file = temp_dir.path().join("rndc.conf");
    let include_file = temp_dir.path().join("rndc.key");

    // Write included file
    let mut file = fs::File::create(&include_file).unwrap();
    writeln!(
        file,
        r#"key "rndc-key" {{
    algorithm hmac-sha256;
    secret "dGVzdC1zZWNyZXQ=";
}};"#
    )
    .unwrap();

    // Write main file with include directive
    let mut file = fs::File::create(&main_file).unwrap();
    writeln!(file, r#"include "{}";"#, include_file.display()).unwrap();
    writeln!(
        file,
        r#"options {{
    default-server localhost;
    default-key "rndc-key";
}};"#
    )
    .unwrap();

    // Parse the file
    let conf = parse_rndc_conf_file(&main_file).unwrap();
    assert_eq!(conf.keys.len(), 1);
    assert!(conf.keys.contains_key("rndc-key"));
    assert_eq!(conf.options.default_server, Some("localhost".to_string()));
    assert_eq!(conf.includes.len(), 1);
}

#[test]
fn test_parse_file_circular_include() {
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.conf");
    let file2 = temp_dir.path().join("file2.conf");

    // Create circular includes: file1 -> file2 -> file1
    let mut f = fs::File::create(&file1).unwrap();
    writeln!(f, r#"include "{}";"#, file2.display()).unwrap();

    let mut f = fs::File::create(&file2).unwrap();
    writeln!(f, r#"include "{}";"#, file1.display()).unwrap();

    // Try to parse
    let result = parse_rndc_conf_file(&file1);
    assert!(result.is_err());
    match result {
        Err(RndcConfParseError::CircularInclude(_)) => {}
        _ => panic!("Expected CircularInclude error"),
    }
}

#[test]
fn test_parse_file_relative_include() {
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    let main_file = temp_dir.path().join("rndc.conf");
    let include_file = subdir.join("rndc.key");

    // Write included file
    let mut file = fs::File::create(&include_file).unwrap();
    writeln!(
        file,
        r#"key "test-key" {{
    algorithm hmac-sha256;
    secret "test";
}};"#
    )
    .unwrap();

    // Write main file with relative include
    let mut file = fs::File::create(&main_file).unwrap();
    writeln!(file, r#"include "subdir/rndc.key";"#).unwrap();

    // Parse the file
    let conf = parse_rndc_conf_file(&main_file).unwrap();
    assert_eq!(conf.keys.len(), 1);
    assert!(conf.keys.contains_key("test-key"));
}

#[test]
fn test_parse_file_options_merging() {
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let main_file = temp_dir.path().join("rndc.conf");
    let include_file = temp_dir.path().join("defaults.conf");

    // Write included file with default options
    let mut file = fs::File::create(&include_file).unwrap();
    writeln!(
        file,
        r#"options {{
    default-server 192.168.1.1;
    default-port 8953;
}};"#
    )
    .unwrap();

    // Write main file that overrides default-server
    let mut file = fs::File::create(&main_file).unwrap();
    writeln!(file, r#"include "{}";"#, include_file.display()).unwrap();
    writeln!(
        file,
        r#"options {{
    default-server localhost;
}};"#
    )
    .unwrap();

    // Parse the file
    let conf = parse_rndc_conf_file(&main_file).unwrap();
    // Main file options take precedence
    assert_eq!(conf.options.default_server, Some("localhost".to_string()));
    // But included port is preserved
    assert_eq!(conf.options.default_port, Some(8953));
}
