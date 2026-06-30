// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Middleware for metrics collection

use axum::{extract::MatchedPath, extract::Request, middleware::Next, response::Response};
use std::time::Instant;

use crate::metrics;

/// Label used for requests that do not match any route (404s, scans).
const UNMATCHED_PATH_LABEL: &str = "<unmatched>";

/// Middleware to track HTTP request metrics
pub async fn track_metrics(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    // Use the matched ROUTE TEMPLATE (e.g. "/api/v1/zones/{zone_name}/records")
    // as the metric label, never the raw request path. The raw path is
    // attacker-controlled and high-cardinality: it embeds zone/record names
    // (information disclosure via the public /metrics endpoint) and lets an
    // attacker mint unbounded distinct label values to exhaust memory (A-4).
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| UNMATCHED_PATH_LABEL.to_string());

    // Process the request
    let response = next.run(req).await;

    // Record metrics
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16();

    metrics::record_http_request(&method, &path, status, duration);

    response
}
