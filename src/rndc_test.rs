// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for rndc module

use super::rndc::*;

#[test]
fn test_rndc_executor_creation() {
    let executor = RndcExecutor::new(None);
    assert_eq!(executor.rndc_path, "/usr/sbin/rndc");

    let executor_custom = RndcExecutor::new(Some("/custom/path/rndc".to_string()));
    assert_eq!(executor_custom.rndc_path, "/custom/path/rndc");
}

#[test]
fn test_rndc_executor_clone() {
    let executor = RndcExecutor::new(Some("/custom/path/rndc".to_string()));
    let cloned = executor.clone();
    assert_eq!(cloned.rndc_path, "/custom/path/rndc");
}

// Note: Integration tests that actually execute rndc commands require
// a running BIND9 instance with rndc configured. These should be in
// integration tests, not unit tests.
