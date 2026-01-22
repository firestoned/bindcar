// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for rndc module

use super::rndc::*;

#[test]
fn test_rndc_executor_creation() {
    // Test with valid parameters
    let result = RndcExecutor::new(
        "127.0.0.1:953".to_string(),
        "sha256".to_string(),
        "dGVzdC1zZWNyZXQtaGVyZQ==".to_string(), // base64 encoded test secret
    );
    assert!(result.is_ok());
}

#[test]
fn test_rndc_executor_creation_with_hmac_prefix() {
    // Test with hmac- prefix (should be stripped)
    let result = RndcExecutor::new(
        "127.0.0.1:953".to_string(),
        "hmac-sha256".to_string(),
        "dGVzdC1zZWNyZXQtaGVyZQ==".to_string(),
    );
    assert!(result.is_ok());
}

#[test]
fn test_rndc_executor_creation_with_all_algorithms() {
    // Test all valid algorithms
    let algorithms = vec!["md5", "sha1", "sha224", "sha256", "sha384", "sha512"];

    for algo in algorithms {
        let result = RndcExecutor::new(
            "127.0.0.1:953".to_string(),
            algo.to_string(),
            "dGVzdC1zZWNyZXQtaGVyZQ==".to_string(),
        );
        assert!(result.is_ok(), "Algorithm {} should be valid", algo);
    }
}

#[test]
fn test_rndc_executor_creation_with_hmac_prefix_all_algorithms() {
    // Test all valid algorithms with hmac- prefix
    let algorithms = vec![
        "hmac-md5",
        "hmac-sha1",
        "hmac-sha224",
        "hmac-sha256",
        "hmac-sha384",
        "hmac-sha512",
    ];

    for algo in algorithms {
        let result = RndcExecutor::new(
            "127.0.0.1:953".to_string(),
            algo.to_string(),
            "dGVzdC1zZWNyZXQtaGVyZQ==".to_string(),
        );
        assert!(result.is_ok(), "Algorithm {} should be valid", algo);
    }
}

#[test]
fn test_rndc_executor_creation_with_invalid_algorithm() {
    let result = RndcExecutor::new(
        "127.0.0.1:953".to_string(),
        "invalid-algo".to_string(),
        "dGVzdC1zZWNyZXQtaGVyZQ==".to_string(),
    );
    assert!(result.is_err());
    let error_msg = result.err().unwrap().to_string();
    assert!(error_msg.contains("Invalid algorithm"));
}

#[test]
fn test_rndc_executor_creation_with_whitespace() {
    // Test trimming of whitespace
    let result = RndcExecutor::new(
        " 127.0.0.1:953 ".to_string(),
        " sha256 ".to_string(),
        " dGVzdC1zZWNyZXQtaGVyZQ== ".to_string(),
    );
    assert!(result.is_ok());
}

#[test]
fn test_rndc_executor_clone() {
    let executor = RndcExecutor::new(
        "127.0.0.1:953".to_string(),
        "sha256".to_string(),
        "dGVzdC1zZWNyZXQtaGVyZQ==".to_string(),
    )
    .expect("Failed to create executor");

    let cloned = executor.clone();
    // If clone succeeds without panic, the test passes
    drop(cloned);
}

#[test]
fn test_rndc_config() {
    let config = RndcConfig {
        server: "127.0.0.1:953".to_string(),
        algorithm: "sha256".to_string(),
        secret: "dGVzdC1zZWNyZXQ=".to_string(),
    };

    assert_eq!(config.server, "127.0.0.1:953");
    assert_eq!(config.algorithm, "sha256");
    assert_eq!(config.secret, "dGVzdC1zZWNyZXQ=");
}

#[test]
fn test_rndc_config_clone() {
    let config = RndcConfig {
        server: "127.0.0.1:953".to_string(),
        algorithm: "sha256".to_string(),
        secret: "dGVzdC1zZWNyZXQ=".to_string(),
    };

    let cloned = config.clone();
    assert_eq!(config.server, cloned.server);
    assert_eq!(config.algorithm, cloned.algorithm);
    assert_eq!(config.secret, cloned.secret);
}

#[test]
fn test_parse_rndc_conf_with_temp_file() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a temporary rndc.conf file
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(
        temp_file,
        r#"
key "rndc-key" {{
    algorithm hmac-sha256;
    secret "dGVzdC1zZWNyZXQtaGVyZQ==";
}};

options {{
    default-server 127.0.0.1;
    default-key "rndc-key";
    default-port 953;
}};
"#
    )
    .unwrap();

    let result = parse_rndc_conf(temp_file.path().to_str().unwrap());
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.server, "127.0.0.1:953");
    assert_eq!(config.algorithm, "hmac-sha256");
    assert_eq!(config.secret, "dGVzdC1zZWNyZXQtaGVyZQ==");
}

#[test]
fn test_parse_rndc_conf_with_default_port() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a temporary rndc.conf file without explicit port
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(
        temp_file,
        r#"
key "test-key" {{
    algorithm sha256;
    secret "dGVzdC1zZWNyZXQ=";
}};

options {{
    default-server localhost;
}};
"#
    )
    .unwrap();

    let result = parse_rndc_conf(temp_file.path().to_str().unwrap());
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.server, "localhost:953"); // Should add default port
}

#[test]
fn test_parse_rndc_conf_with_server_and_port() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a temporary rndc.conf file with server:port format
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(
        temp_file,
        r#"
key "test-key" {{
    algorithm sha256;
    secret "dGVzdC1zZWNyZXQ=";
}};

options {{
    default-server 192.168.1.1:8953;
}};
"#
    )
    .unwrap();

    let result = parse_rndc_conf(temp_file.path().to_str().unwrap());
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.server, "192.168.1.1:8953"); // Should keep existing port
}

#[test]
fn test_parse_rndc_conf_with_no_default_server() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a temporary rndc.conf file without default-server
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(
        temp_file,
        r#"
key "test-key" {{
    algorithm sha256;
    secret "dGVzdC1zZWNyZXQ=";
}};
"#
    )
    .unwrap();

    let result = parse_rndc_conf(temp_file.path().to_str().unwrap());
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.server, "127.0.0.1:953"); // Should use default
}

#[test]
fn test_parse_rndc_conf_with_no_keys() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a temporary rndc.conf file without keys
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(
        temp_file,
        r#"
options {{
    default-server 127.0.0.1;
}};
"#
    )
    .unwrap();

    let result = parse_rndc_conf(temp_file.path().to_str().unwrap());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No keys found"));
}

#[test]
fn test_parse_rndc_conf_with_missing_default_key() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a temporary rndc.conf file with non-existent default key
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(
        temp_file,
        r#"
key "test-key" {{
    algorithm sha256;
    secret "dGVzdC1zZWNyZXQ=";
}};

options {{
    default-key "non-existent-key";
}};
"#
    )
    .unwrap();

    let result = parse_rndc_conf(temp_file.path().to_str().unwrap());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_parse_rndc_conf_file_not_found() {
    let result = parse_rndc_conf("/nonexistent/path/rndc.conf");
    assert!(result.is_err());
}

// Note: Integration tests that actually execute rndc commands require
// a running BIND9 instance with rndc configured. These should be in
// integration tests, not unit tests.
