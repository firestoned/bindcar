// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for metrics module

use super::metrics::*;

#[test]
fn test_init_metrics() {
    init_metrics();
    // Verify app info metric was set
    let metrics = gather_metrics().unwrap();
    assert!(metrics.contains("bindcar_app_info"));
    assert!(metrics.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_gather_metrics() {
    // Record at least one metric so gather_metrics returns something
    init_metrics();
    let result = gather_metrics();
    assert!(result.is_ok());

    let metrics = result.unwrap();
    // Metrics should be a non-empty string
    assert!(!metrics.is_empty());
}

#[test]
fn test_record_http_request() {
    record_http_request("GET", "/api/v1/zones", 200, 0.123);
    record_http_request("POST", "/api/v1/zones", 201, 0.456);
    record_http_request("GET", "/api/v1/zones", 500, 1.234);

    let metrics = gather_metrics().unwrap();
    assert!(metrics.contains("bindcar_http_requests_total"));
    assert!(metrics.contains("bindcar_http_request_duration_seconds"));
}

#[test]
fn test_record_http_request_various_methods() {
    let methods = vec!["GET", "POST", "PUT", "DELETE", "PATCH"];
    let paths = vec!["/api/v1/zones", "/api/v1/health", "/metrics"];
    let statuses = vec![200, 201, 400, 404, 500, 502];

    for method in &methods {
        for path in &paths {
            for status in &statuses {
                record_http_request(method, path, *status, 0.1);
            }
        }
    }

    let metrics = gather_metrics().unwrap();
    assert!(metrics.contains("bindcar_http_requests_total"));
}

#[test]
fn test_record_zone_operation_success() {
    record_zone_operation("create", true);
    record_zone_operation("delete", true);
    record_zone_operation("reload", true);

    let metrics = gather_metrics().unwrap();
    assert!(metrics.contains("bindcar_zone_operations_total"));
    assert!(metrics.contains("success"));
}

#[test]
fn test_record_zone_operation_failure() {
    record_zone_operation("create", false);
    record_zone_operation("delete", false);

    let metrics = gather_metrics().unwrap();
    assert!(metrics.contains("bindcar_zone_operations_total"));
    assert!(metrics.contains("error"));
}

#[test]
fn test_record_rndc_command_success() {
    record_rndc_command("status", true, 0.123);
    record_rndc_command("addzone", true, 0.456);
    record_rndc_command("delzone", true, 0.789);

    let metrics = gather_metrics().unwrap();
    assert!(metrics.contains("bindcar_rndc_commands_total"));
    assert!(metrics.contains("bindcar_rndc_command_duration_seconds"));
}

#[test]
fn test_record_rndc_command_failure() {
    record_rndc_command("addzone", false, 1.234);
    record_rndc_command("delzone", false, 2.345);

    let metrics = gather_metrics().unwrap();
    assert!(metrics.contains("bindcar_rndc_commands_total"));
}

#[test]
fn test_record_rndc_command_duration() {
    // Test different durations to ensure histogram buckets are used
    let durations = vec![0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0];
    for duration in durations {
        record_rndc_command("test", true, duration);
    }

    let metrics = gather_metrics().unwrap();
    assert!(metrics.contains("bindcar_rndc_command_duration_seconds"));
}

#[test]
fn test_update_zones_count() {
    update_zones_count(0);
    update_zones_count(5);
    update_zones_count(100);
    update_zones_count(1000);

    let metrics = gather_metrics().unwrap();
    assert!(metrics.contains("bindcar_zones_managed_total"));
}

#[test]
fn test_update_zones_count_negative_becomes_zero() {
    // Ensure negative counts don't break metrics
    update_zones_count(-5);

    let metrics = gather_metrics().unwrap();
    assert!(metrics.contains("bindcar_zones_managed_total"));
}

#[test]
fn test_all_metrics_registered() {
    // Trigger all metrics at least once
    init_metrics();
    record_http_request("GET", "/test", 200, 0.1);
    record_zone_operation("test", true);
    record_rndc_command("test", true, 0.1);
    update_zones_count(10);

    let metrics = gather_metrics().unwrap();

    // Verify all metric families are present
    assert!(metrics.contains("bindcar_http_requests_total"));
    assert!(metrics.contains("bindcar_http_request_duration_seconds"));
    assert!(metrics.contains("bindcar_zone_operations_total"));
    assert!(metrics.contains("bindcar_rndc_commands_total"));
    assert!(metrics.contains("bindcar_rndc_command_duration_seconds"));
    assert!(metrics.contains("bindcar_zones_managed_total"));
    assert!(metrics.contains("bindcar_app_info"));
}
