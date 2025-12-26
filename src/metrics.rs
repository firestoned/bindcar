// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Prometheus metrics for bindcar
//!
//! This module provides comprehensive metrics for monitoring the BIND9 RNDC API server:
//! - HTTP request metrics (count, duration, status codes)
//! - Zone operation metrics (creates, deletes, reloads, etc.)
//! - RNDC command execution metrics
//! - System health metrics

use lazy_static::lazy_static;
use prometheus::{
    opts, register_counter_vec, register_gauge, register_histogram_vec, CounterVec, Encoder, Gauge,
    HistogramVec, TextEncoder,
};

lazy_static! {
    /// HTTP request counter by method, path, and status code
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        opts!(
            "bindcar_http_requests_total",
            "Total number of HTTP requests processed"
        ),
        &["method", "path", "status"]
    )
    .expect("Failed to create HTTP_REQUESTS_TOTAL metric");

    /// HTTP request duration histogram
    pub static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "bindcar_http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "path"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Failed to create HTTP_REQUEST_DURATION_SECONDS metric");

    /// Zone operations counter by operation type and result
    pub static ref ZONE_OPERATIONS_TOTAL: CounterVec = register_counter_vec!(
        opts!(
            "bindcar_zone_operations_total",
            "Total number of zone operations"
        ),
        &["operation", "result"]
    )
    .expect("Failed to create ZONE_OPERATIONS_TOTAL metric");

    /// RNDC command counter by command and result
    pub static ref RNDC_COMMANDS_TOTAL: CounterVec = register_counter_vec!(
        opts!(
            "bindcar_rndc_commands_total",
            "Total number of RNDC commands executed"
        ),
        &["command", "result"]
    )
    .expect("Failed to create RNDC_COMMANDS_TOTAL metric");

    /// RNDC command duration histogram
    pub static ref RNDC_COMMAND_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "bindcar_rndc_command_duration_seconds",
        "RNDC command execution duration in seconds",
        &["command"],
        vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Failed to create RNDC_COMMAND_DURATION_SECONDS metric");

    /// Total number of zones managed
    pub static ref ZONES_MANAGED_TOTAL: Gauge = register_gauge!(
        opts!(
            "bindcar_zones_managed_total",
            "Total number of zones currently managed"
        )
    )
    .expect("Failed to create ZONES_MANAGED_TOTAL metric");

    /// Application info metric
    pub static ref APP_INFO: CounterVec = register_counter_vec!(
        opts!(
            "bindcar_app_info",
            "Application information"
        ),
        &["version"]
    )
    .expect("Failed to create APP_INFO metric");

    /// Rate limit counter by result
    pub static ref RATE_LIMIT_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        opts!(
            "bindcar_rate_limit_requests_total",
            "Total number of rate limit checks"
        ),
        &["result"]
    )
    .expect("Failed to create RATE_LIMIT_REQUESTS_TOTAL metric");
}

/// Initialize metrics with application info
pub fn init_metrics() {
    APP_INFO
        .with_label_values(&[env!("CARGO_PKG_VERSION")])
        .inc();
}

/// Generate metrics output in Prometheus format
pub fn gather_metrics() -> Result<String, Box<dyn std::error::Error>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}

/// Record an HTTP request
pub fn record_http_request(method: &str, path: &str, status: u16, duration: f64) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, path, &status.to_string()])
        .inc();
    HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[method, path])
        .observe(duration);
}

/// Record a zone operation
pub fn record_zone_operation(operation: &str, success: bool) {
    let result = if success { "success" } else { "error" };
    ZONE_OPERATIONS_TOTAL
        .with_label_values(&[operation, result])
        .inc();
}

/// Record an RNDC command execution
pub fn record_rndc_command(command: &str, success: bool, duration: f64) {
    let result = if success { "success" } else { "error" };
    RNDC_COMMANDS_TOTAL
        .with_label_values(&[command, result])
        .inc();
    RNDC_COMMAND_DURATION_SECONDS
        .with_label_values(&[command])
        .observe(duration);
}

/// Update the total number of managed zones
pub fn update_zones_count(count: i64) {
    ZONES_MANAGED_TOTAL.set(count as f64);
}

/// Record a rate limit check
pub fn record_rate_limit(allowed: bool) {
    let result = if allowed { "allowed" } else { "rejected" };
    RATE_LIMIT_REQUESTS_TOTAL
        .with_label_values(&[result])
        .inc();
}
