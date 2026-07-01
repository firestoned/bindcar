// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! BIND9 RNDC API Server
//!
//! Supports two operating modes selected via subcommand:
//!
//! - `bindcar run` (default) — sidecar mode alongside a local BIND9 instance
//! - `bindcar drone` — standalone mode managing a remote BIND9 instance

use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    middleware as axum_middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// Import from the library
use bindcar::{
    auth::authenticate,
    cli::{Cli, Commands},
    metrics, middleware,
    rate_limit::RateLimitConfig,
    rndc::RndcExecutor,
    types::{AppState, ErrorResponse},
    zones,
};
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::PeerIpKeyExtractor, GovernorLayer,
};

/// OpenAPI documentation structure
#[derive(OpenApi)]
#[openapi(
    paths(
        zones::create_zone,
        zones::delete_zone,
        zones::modify_zone,
        zones::reload_zone,
        zones::zone_status,
        zones::freeze_zone,
        zones::thaw_zone,
        zones::notify_zone,
        zones::retransfer_zone,
        zones::server_status,
        zones::list_zones,
        zones::get_zone,
        bindcar::records::add_record,
        bindcar::records::remove_record,
        bindcar::records::update_record,
    ),
    components(
        schemas(
            zones::CreateZoneRequest,
            zones::ModifyZoneRequest,
            zones::ZoneResponse,
            zones::ServerStatusResponse,
            zones::ZoneInfo,
            zones::ZoneListResponse,
            zones::ZoneConfig,
            zones::SoaRecord,
            zones::DnsRecord,
            bindcar::records::AddRecordRequest,
            bindcar::records::RemoveRecordRequest,
            bindcar::records::UpdateRecordRequest,
            bindcar::records::RecordResponse,
        )
    ),
    tags(
        (name = "zones", description = "Zone management endpoints"),
        (name = "records", description = "DNS record management endpoints"),
        (name = "server", description = "Server status endpoints")
    ),
    info(
        title = "Bindcar API",
        version = "0.1.0",
        description = "HTTP REST API for managing BIND9 zones and DNS records via RNDC and nsupdate",
        license(name = "MIT")
    )
)]
struct ApiDoc;

/// Server configuration
const DEFAULT_BIND_ZONE_DIR: &str = "/var/cache/bind";
const DEFAULT_API_PORT: u16 = 8080;
const DEFAULT_BIND_API_ADDRESS: &str = "0.0.0.0";

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

/// Map a readiness sub-check outcome to a non-sensitive status token.
///
/// `/health`, `/ready`, and `/metrics` are intentionally unauthenticated (for
/// kubelet probes and Prometheus scraping), so their responses must never carry
/// internal detail. This emits only `"<name>: ok"` / `"<name>: error"` — the
/// underlying path or backend error text is logged server-side instead, never
/// returned to the caller.
fn ready_check_label(name: &str, ok: bool) -> String {
    format!("{}: {}", name, if ok { "ok" } else { "error" })
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
        Err(e) => {
            // Log the detail; return a generic message (this endpoint is
            // unauthenticated, so the body must not leak internals).
            error!("failed to gather metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to gather metrics".to_string(),
                    details: None,
                }),
            )
                .into_response()
        }
    }
}

/// Readiness check endpoint
async fn ready_check(State(state): State<AppState>) -> Json<ReadyResponse> {
    let mut checks = Vec::new();
    let mut ready = true;

    // Check the zone directory. The response carries only ok/error; the path and
    // any IO error are logged server-side, never returned (the endpoint is
    // unauthenticated and must not disclose the filesystem layout).
    //
    // Defense-in-depth barrier: `zone_dir` is canonicalized once at startup
    // (`zones::resolve_zone_dir`), but it reaches this handler through the axum
    // `State` extractor, which static analysis models as untrusted. Re-assert the
    // startup invariant (absolute, no `..`/`.` components) immediately before the
    // filesystem sink so we never `metadata()` an unexpected path.
    let zone_dir_ok = if !zones::is_normalized_zone_dir(&state.zone_dir) {
        warn!(
            "zone directory {:?} is not a normalized absolute path; refusing to probe it",
            state.zone_dir
        );
        false
    } else {
        match tokio::fs::metadata(&state.zone_dir).await {
            Ok(metadata) => {
                if metadata.is_dir() {
                    true
                } else {
                    warn!("zone directory {:?} is not a directory", state.zone_dir);
                    false
                }
            }
            Err(e) => {
                warn!("zone directory {:?} not accessible: {}", state.zone_dir, e);
                false
            }
        }
    };
    ready &= zone_dir_ok;
    checks.push(ready_check_label("zone_dir", zone_dir_ok));

    // Check if rndc is available. The backend error is logged, not returned.
    let rndc_ok = match state.rndc.status().await {
        Ok(_) => true,
        Err(e) => {
            warn!("RNDC not ready: {}", e);
            false
        }
    };
    ready &= rndc_ok;
    checks.push(ready_check_label("rndc", rndc_ok));

    Json(ReadyResponse { ready, checks })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.debug);

    // The insecure-auth override may come from the CLI flag or an env var (the
    // latter is convenient for container deployments without changing args).
    let insecure_override = cli.i_know_this_is_insecure
        || std::env::var("BINDCAR_ALLOW_INSECURE_AUTH")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

    start_server(cli.resolved_command(), insecure_override).await
}

fn init_tracing(debug: bool) {
    let filter = if debug {
        tracing_subscriber::EnvFilter::new("debug")
    } else {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .init();
}

async fn start_server(command: &Commands, insecure_override: bool) -> anyhow::Result<()> {
    match command {
        Commands::Run => info!(
            "starting bindcar v{} [sidecar mode]",
            env!("CARGO_PKG_VERSION")
        ),
        Commands::Drone => info!(
            "starting bindcar v{} [drone mode] - standalone, managing remote BIND9",
            env!("CARGO_PKG_VERSION")
        ),
    }

    // initialize metrics
    metrics::init_metrics();

    // get configuration from environment
    let zone_dir =
        std::env::var("BIND_ZONE_DIR").unwrap_or_else(|_| DEFAULT_BIND_ZONE_DIR.to_string());
    let api_port = std::env::var("API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_API_PORT);
    let bind_host =
        std::env::var("BIND_API_ADDRESS").unwrap_or_else(|_| DEFAULT_BIND_API_ADDRESS.to_string());
    let disable_auth = std::env::var("DISABLE_AUTH")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    info!("zone directory: {}", zone_dir);
    info!("api address: {}:{}", bind_host, api_port);
    // Startup guard (B-4): never silently expose an unauthenticated or
    // presence-only API on a non-loopback interface. Requires real auth
    // (TokenReview feature or BIND_API_TOKEN), a loopback bind, or an explicit
    // operator override.
    if let Err(e) = bindcar::auth::check_startup_auth_posture(
        !disable_auth,
        bindcar::auth::has_real_auth(),
        &bind_host,
        insecure_override,
    ) {
        error!("{}", e);
        return Err(anyhow::anyhow!(e));
    }

    if disable_auth {
        warn!("⚠️  authentication is disabled - api endpoints are unprotected!");
        warn!("⚠️  this should only be used in trusted environments (e.g., linkerd service mesh)");
    } else {
        info!("authentication is enabled");
        if std::env::var(bindcar::auth::BIND_API_TOKEN_ENV)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            info!("shared-secret API token authentication is active (BIND_API_TOKEN)");
        }

        #[cfg(feature = "k8s-token-review")]
        {
            use bindcar::auth::{
                check_authorization_posture, detect_kube_auth_mode, KubeAuthMode,
                TokenReviewConfig, ALLOW_ANY_SERVICE_ACCOUNT_ENV,
            };

            // Fail-closed authorization posture (A2): with TokenReview active but
            // no namespace/service-account allowlist, any authenticated
            // ServiceAccount in the cluster would be authorized. Refuse to start
            // unless the operator configured an allowlist or explicitly opted in.
            let tr_config = TokenReviewConfig::from_env();
            let allow_any = std::env::var(ALLOW_ANY_SERVICE_ACCOUNT_ENV)
                .ok()
                .and_then(|v| v.parse::<bool>().ok())
                .unwrap_or(false);
            if let Err(e) =
                check_authorization_posture(tr_config.is_authorization_restricted(), allow_any)
            {
                error!("{}", e);
                return Err(anyhow::anyhow!(e));
            }
            if allow_any && !tr_config.is_authorization_restricted() {
                warn!("⚠️  BIND_ALLOW_ANY_SERVICEACCOUNT is set: every authenticated ServiceAccount in the cluster is authorized");
            }

            match detect_kube_auth_mode() {
                KubeAuthMode::Explicit { ref server, .. } => {
                    info!(
                        "kubernetes auth mode: explicit (KUBE_API_SERVER={})",
                        server
                    );
                }
                KubeAuthMode::Default => {
                    info!("kubernetes auth mode: try_default (KUBECONFIG / ~/.kube/config / in-cluster)");
                }
            }
        }
    }

    // load rate limiting configuration
    let rate_limit_config = RateLimitConfig::from_env();
    if let Err(e) = rate_limit_config.validate() {
        error!("invalid rate limit configuration: {}", e);
        return Err(anyhow::anyhow!("invalid rate limit configuration: {}", e));
    }

    if rate_limit_config.enabled {
        info!(
            "rate limiting enabled: {} requests per {} seconds (burst: {})",
            rate_limit_config.requests_per_period,
            rate_limit_config.period_secs,
            rate_limit_config.burst_size
        );
    } else {
        warn!("⚠️  rate limiting is disabled");
    }

    // get rndc configuration from environment or fallback to rndc.conf
    // Each parameter is checked independently - env var takes priority, then rndc.conf

    // Try to parse rndc.conf file first to have fallback values
    let config_paths = vec!["/etc/bind/rndc.conf", "/etc/rndc.conf"];
    let mut parsed_config = None;

    for path in &config_paths {
        match bindcar::rndc::parse_rndc_conf(path) {
            Ok(cfg) => {
                info!("successfully parsed rndc configuration from {}", path);
                parsed_config = Some(cfg);
                break;
            }
            Err(e) => {
                debug!("failed to parse {}: {}", path, e);
            }
        }
    }

    // Check each parameter independently: env var first, then rndc.conf, then hardcoded default
    let rndc_server = if let Ok(server) = std::env::var("RNDC_SERVER") {
        info!("using RNDC_SERVER from environment: {}", server);
        server
    } else if let Some(ref cfg) = parsed_config {
        info!("using server from rndc.conf: {}", cfg.server);
        cfg.server.clone()
    } else {
        let default = "127.0.0.1:953".to_string();
        warn!("using default RNDC_SERVER: {}", default);
        default
    };

    let rndc_algorithm = if let Ok(algorithm) = std::env::var("RNDC_ALGORITHM") {
        info!("using RNDC_ALGORITHM from environment: {}", algorithm);
        algorithm
    } else if let Some(ref cfg) = parsed_config {
        info!("using algorithm from rndc.conf: {}", cfg.algorithm);
        cfg.algorithm.clone()
    } else {
        let default = "sha256".to_string();
        warn!("using default RNDC_ALGORITHM: {}", default);
        default
    };

    let rndc_secret = if let Ok(secret) = std::env::var("RNDC_SECRET") {
        info!("using RNDC_SECRET from environment");
        secret
    } else if let Some(ref cfg) = parsed_config {
        info!("using secret from rndc.conf");
        cfg.secret.clone()
    } else {
        error!("rndc configuration not found!");
        error!("either set RNDC_SECRET environment variable or ensure /etc/bind/rndc.conf exists");
        return Err(anyhow::anyhow!(
            "rndc configuration required: set RNDC_SECRET env var or create /etc/bind/rndc.conf"
        ));
    };

    // Resolve and validate the zone directory once at startup. Canonicalizing the
    // operator-provided BIND_ZONE_DIR resolves symlinks and `..`, rejects a missing
    // or non-directory path up front, and breaks the env->filesystem taint flow
    // before the value is reused by the read_dir/metadata handlers (B-1).
    let zone_dir = zones::resolve_zone_dir(&zone_dir)
        .map_err(|e| anyhow::anyhow!("zone directory not usable: {}", e))?;
    info!("resolved zone directory: {}", zone_dir);

    // create rndc executor
    let rndc = Arc::new(
        RndcExecutor::new(
            rndc_server.clone(),
            rndc_algorithm.clone(),
            rndc_secret.clone(),
        )
        .context("failed to create rndc client")?,
    );

    // Configure nsupdate executor (hybrid approach: env vars → rndc credentials)
    let nsupdate_key_name = std::env::var("NSUPDATE_KEY_NAME")
        .ok()
        .or_else(|| std::env::var("RNDC_KEY_NAME").ok())
        .or(Some("rndc-key".to_string()));

    let nsupdate_algorithm = std::env::var("NSUPDATE_ALGORITHM")
        .ok()
        .or(Some(rndc_algorithm.clone()));

    let nsupdate_secret = std::env::var("NSUPDATE_SECRET")
        .ok()
        .or(Some(rndc_secret.clone()));

    let nsupdate_server = std::env::var("NSUPDATE_SERVER")
        .ok()
        .unwrap_or_else(|| "127.0.0.1".to_string());

    let nsupdate_port = std::env::var("NSUPDATE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(53);

    info!("nsupdate executor configuration:");
    info!("  server: {}:{}", nsupdate_server, nsupdate_port);
    info!("  TSIG key: {:?}", nsupdate_key_name);

    // create nsupdate executor
    let nsupdate = Arc::new(
        bindcar::nsupdate::NsupdateExecutor::new(
            nsupdate_server,
            nsupdate_port,
            nsupdate_key_name,
            nsupdate_algorithm,
            nsupdate_secret,
        )
        .context("failed to create nsupdate executor")?,
    );

    // create application state
    let state = AppState {
        rndc,
        nsupdate,
        zone_dir: zone_dir.clone(),
    };

    // build api routes
    let api_routes = Router::new()
        .route("/zones", post(zones::create_zone).get(zones::list_zones))
        .route(
            "/zones/{name}",
            get(zones::get_zone)
                .delete(zones::delete_zone)
                .patch(zones::modify_zone),
        )
        .route("/zones/{name}/reload", post(zones::reload_zone))
        .route("/zones/{name}/status", get(zones::zone_status))
        .route("/zones/{name}/freeze", post(zones::freeze_zone))
        .route("/zones/{name}/thaw", post(zones::thaw_zone))
        .route("/zones/{name}/notify", post(zones::notify_zone))
        .route("/zones/{name}/retransfer", post(zones::retransfer_zone))
        .route(
            "/zones/{name}/records",
            post(bindcar::records::add_record)
                .delete(bindcar::records::remove_record)
                .put(bindcar::records::update_record),
        )
        .route("/server/status", get(zones::server_status))
        .with_state(state.clone());

    // conditionally apply authentication middleware
    let api_routes = if !disable_auth {
        api_routes.layer(axum_middleware::from_fn(authenticate))
    } else {
        api_routes
    };

    // conditionally apply rate limiting layer
    let api_routes = if rate_limit_config.enabled {
        // Calculate requests per second from period
        let per_second =
            rate_limit_config.requests_per_period / rate_limit_config.period_secs.max(1) as u32;
        let per_second = per_second.max(1); // Ensure at least 1 request per second

        // Build governor configuration
        // A-1: key on the real TCP peer IP, NOT the spoofable X-Forwarded-For /
        // X-Real-IP / Forwarded headers (SmartIpKeyExtractor). bindcar is reached
        // directly by the operator's pods (no trusted L7 proxy in front), so
        // trusting client-supplied forwarding headers would let any caller evade
        // the limit (rotate the header) or exhaust another client's bucket
        // (spoof the victim's IP).
        let governor_conf = Arc::new(
            GovernorConfigBuilder::default()
                .key_extractor(PeerIpKeyExtractor)
                .per_second(per_second.into())
                .burst_size(rate_limit_config.burst_size)
                .finish()
                .expect("Failed to create governor config"),
        );

        api_routes.layer(GovernorLayer::new(governor_conf))
    } else {
        api_routes
    };

    // The Swagger UI and OpenAPI spec are served UNAUTHENTICATED and disclose the
    // full mutate-API surface, so they are OFF by default and only mounted when
    // BIND_ENABLE_DOCS is truthy (A13). Enable them for local development only.
    let enable_docs = std::env::var("BIND_ENABLE_DOCS")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    // build main router
    let mut app = Router::new();
    if enable_docs {
        info!("serving unauthenticated API docs at /api/v1/docs (BIND_ENABLE_DOCS=true)");
        app = app
            .merge(SwaggerUi::new("/api/v1/docs").url("/api/v1/openapi.json", ApiDoc::openapi()));
    }
    let app = app
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/ready", get(ready_check))
        .route("/metrics", get(metrics_handler))
        .nest("/api/v1", api_routes)
        .with_state(state)
        .layer(axum_middleware::from_fn(middleware::track_metrics))
        .layer(TraceLayer::new_for_http());

    // start server
    let addr = format!("{}:{}", bind_host, api_port);

    info!("bind9 rndc api server listening on {}", addr);
    info!("swagger ui available at http://{}/api/v1/docs", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("server error")?;

    Ok(())
}

#[cfg(test)]
mod main_test;
