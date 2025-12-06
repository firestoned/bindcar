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

// Note: Integration tests that actually execute rndc commands require
// a running BIND9 instance with rndc configured. These should be in
// integration tests, not unit tests.
