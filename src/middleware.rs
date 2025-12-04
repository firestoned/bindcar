// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Middleware for metrics collection

use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;

use crate::metrics;

/// Middleware to track HTTP request metrics
pub async fn track_metrics(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Process the request
    let response = next.run(req).await;

    // Record metrics
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16();

    metrics::record_http_request(&method, &path, status, duration);

    response
}
