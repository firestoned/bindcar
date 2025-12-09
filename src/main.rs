// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! BIND9 RNDC API Server
//!
//! A lightweight HTTP REST API server that manages BIND9 zones by:
//! - Creating zone files in `/var/cache/bind/`
//! - Executing local rndc commands to manage zones
//! - Providing authenticated access via Kubernetes ServiceAccount tokens
//!
//! This server runs as a sidecar container alongside BIND9, sharing
//! the zone file storage volume and rndc configuration.

use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    middleware as axum_middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// Import from the library
use bindcar::{
    auth::authenticate,
    metrics, middleware,
    rndc::RndcExecutor,
    types::{AppState, ErrorResponse},
    zones,
};

/// OpenAPI documentation structure
#[derive(OpenApi)]
#[openapi(
    paths(
        zones::create_zone,
        zones::delete_zone,
        zones::reload_zone,
        zones::zone_status,
        zones::freeze_zone,
        zones::thaw_zone,
        zones::notify_zone,
        zones::server_status,
        zones::list_zones,
        zones::get_zone,
    ),
    components(
        schemas(
            zones::CreateZoneRequest,
            zones::ZoneResponse,
            zones::ServerStatusResponse,
            zones::ZoneInfo,
            zones::ZoneListResponse,
            zones::ZoneConfig,
            zones::SoaRecord,
            zones::DnsRecord,
        )
    ),
    tags(
        (name = "zones", description = "Zone management endpoints"),
        (name = "server", description = "Server status endpoints")
    ),
    info(
        title = "Bindcar API",
        version = "0.1.0",
        description = "HTTP REST API for managing BIND9 zones via RNDC",
        license(name = "MIT")
    )
)]
struct ApiDoc;

/// Server configuration
const DEFAULT_BIND_ZONE_DIR: &str = "/var/cache/bind";
const DEFAULT_API_PORT: u16 = 8080;

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// Readiness check response
#[derive(Serialize)]
struct ReadyResponse {
    ready: bool,
    checks: Vec<String>,
}

/// Health check endpoint
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Metrics endpoint for Prometheus scraping
async fn metrics_handler() -> Response {
    match metrics::gather_metrics() {
        Ok(metrics_text) => (
            StatusCode::OK,
            [("Content-Type", "text/plain; version=0.0.4")],
            metrics_text,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to gather metrics: {}", e),
                details: None,
            }),
        )
            .into_response(),
    }
}

/// Readiness check endpoint
async fn ready_check(State(state): State<AppState>) -> Json<ReadyResponse> {
    let mut checks = Vec::new();
    let mut ready = true;

    // Check if zone directory is writable
    match tokio::fs::metadata(&state.zone_dir).await {
        Ok(metadata) if metadata.is_dir() => {
            checks.push(format!("zone_dir_accessible: {}", state.zone_dir));
        }
        Ok(_) => {
            ready = false;
            checks.push(format!("zone_dir_not_directory: {}", state.zone_dir));
        }
        Err(e) => {
            ready = false;
            checks.push(format!("zone_dir_error: {}", e));
        }
    }

    // Check if rndc is available
    match state.rndc.status().await {
        Ok(_) => {
            checks.push("rndc_available: true".to_string());
        }
        Err(e) => {
            warn!("RNDC not ready: {}", e);
            ready = false;
            checks.push(format!("rndc_error: {}", e));
        }
    }

    Json(ReadyResponse { ready, checks })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    info!(
        "starting bind9 rndc api server v{}",
        env!("CARGO_PKG_VERSION")
    );

    // initialize metrics
    metrics::init_metrics();

    // get configuration from environment
    let zone_dir =
        std::env::var("BIND_ZONE_DIR").unwrap_or_else(|_| DEFAULT_BIND_ZONE_DIR.to_string());
    let api_port = std::env::var("API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_API_PORT);
    let disable_auth = std::env::var("DISABLE_AUTH")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    info!("zone directory: {}", zone_dir);
    info!("api port: {}", api_port);
    if disable_auth {
        warn!("⚠️  authentication is disabled - api endpoints are unprotected!");
        warn!("⚠️  this should only be used in trusted environments (e.g., linkerd service mesh)");
    } else {
        info!("authentication is enabled");
    }

    // get rndc configuration from environment or fallback to rndc.conf
    let (rndc_server, rndc_algorithm, rndc_secret) = match (
        std::env::var("RNDC_SECRET").ok(),
        std::env::var("RNDC_SERVER").ok(),
        std::env::var("RNDC_ALGORITHM").ok(),
    ) {
        (Some(secret), server, algorithm) => {
            // environment variables provided
            let server = server.unwrap_or_else(|| "127.0.0.1:953".to_string());
            let algorithm = algorithm.unwrap_or_else(|| "sha256".to_string());
            info!("using rndc configuration from environment variables");
            info!("rndc server: {}", server);
            info!("rndc algorithm: {}", algorithm);
            (server, algorithm, secret)
        }
        (None, _, _) => {
            // try to parse from rndc.conf files
            info!("RNDC_SECRET not set, attempting to parse rndc.conf");

            let config_paths = vec!["/etc/bind/rndc.conf", "/etc/rndc.conf"];
            let mut config = None;

            for path in &config_paths {
                match bindcar::rndc::parse_rndc_conf(path) {
                    Ok(cfg) => {
                        info!("successfully parsed rndc configuration from {}", path);
                        config = Some(cfg);
                        break;
                    }
                    Err(e) => {
                        debug!("failed to parse {}: {}", path, e);
                    }
                }
            }

            match config {
                Some(cfg) => {
                    info!("rndc server: {}", cfg.server);
                    info!("rndc algorithm: {}", cfg.algorithm);
                    (cfg.server, cfg.algorithm, cfg.secret)
                }
                None => {
                    error!("rndc configuration not found!");
                    error!("either set RNDC_SECRET environment variable or ensure /etc/bind/rndc.conf exists");
                    return Err(anyhow::anyhow!(
                        "rndc configuration required: set RNDC_SECRET env var or create /etc/bind/rndc.conf"
                    ));
                }
            }
        }
    };

    // verify zone directory exists
    if !tokio::fs::metadata(&zone_dir).await?.is_dir() {
        error!("zone directory does not exist: {}", zone_dir);
        return Err(anyhow::anyhow!("zone directory not found"));
    }

    // create rndc executor
    let rndc = Arc::new(
        RndcExecutor::new(rndc_server, rndc_algorithm, rndc_secret)
            .context("failed to create rndc client")?,
    );

    // create application state
    let state = AppState {
        rndc,
        zone_dir: zone_dir.clone(),
    };

    // build api routes
    let api_routes = Router::new()
        .route("/zones", post(zones::create_zone).get(zones::list_zones))
        .route(
            "/zones/{name}",
            get(zones::get_zone).delete(zones::delete_zone),
        )
        .route("/zones/{name}/reload", post(zones::reload_zone))
        .route("/zones/{name}/status", get(zones::zone_status))
        .route("/zones/{name}/freeze", post(zones::freeze_zone))
        .route("/zones/{name}/thaw", post(zones::thaw_zone))
        .route("/zones/{name}/notify", post(zones::notify_zone))
        .route("/server/status", get(zones::server_status))
        .with_state(state.clone());

    // conditionally apply authentication middleware
    let api_routes = if disable_auth {
        api_routes
    } else {
        api_routes.layer(axum_middleware::from_fn(authenticate))
    };

    // build main router
    let app = Router::new()
        .merge(SwaggerUi::new("/api/v1/docs").url("/api/v1/openapi.json", ApiDoc::openapi()))
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/ready", get(ready_check))
        .route("/metrics", get(metrics_handler))
        .nest("/api/v1", api_routes)
        .with_state(state)
        .layer(axum_middleware::from_fn(middleware::track_metrics))
        .layer(TraceLayer::new_for_http());

    // start server
    let addr = format!("0.0.0.0:{}", api_port);

    info!("bind9 rndc api server listening on {}", addr);
    info!("swagger ui available at http://{}/api/v1/docs", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(listener, app.into_make_service())
        .await
        .context("server error")?;

    Ok(())
}
