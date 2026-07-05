// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Authentication middleware for Kubernetes ServiceAccount tokens
//!
//! This module validates that incoming requests include a valid ServiceAccount token
//! in the Authorization header.
//!
//! ## Token Validation Modes
//!
//! ### Basic Mode (default)
//! - Checks for token presence and format
//! - Suitable for trusted environments or when using external auth (API gateway, service mesh)
//!
//! ### Kubernetes TokenReview Mode (feature: `k8s-token-review`)
//! - Validates tokens against Kubernetes TokenReview API
//! - Verifies token authenticity and expiration
//! - Validates token audience
//! - Restricts to allowed namespaces and service accounts
//! - Requires in-cluster configuration or kubeconfig
//!
//! ## Security Configuration
//!
//! Environment variables for TokenReview mode:
//! - `BIND_TOKEN_AUDIENCES` - Comma-separated list of expected audiences (default: "bindcar")
//! - `BIND_ALLOWED_NAMESPACES` - Comma-separated list of allowed namespaces (empty = allow all)
//! - `BIND_ALLOWED_SERVICE_ACCOUNTS` - Comma-separated list of allowed SA names (empty = allow all)

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde::Serialize;
use tracing::{debug, warn};

#[cfg(feature = "k8s-token-review")]
use k8s_openapi::api::authentication::v1::TokenReview;
#[cfg(feature = "k8s-token-review")]
use kube::{
    config::{
        AuthInfo, Cluster, KubeConfigOptions, Kubeconfig, NamedAuthInfo, NamedCluster, NamedContext,
    },
    Api, Client, Config,
};
#[cfg(feature = "k8s-token-review")]
use std::env;
#[cfg(feature = "k8s-token-review")]
use tracing::error;

/// Error response for authentication failures
#[derive(Serialize)]
pub struct AuthError {
    pub error: String,
}

/// Environment variable holding a shared API token.
///
/// When set to a non-empty value, every request's Bearer token is compared
/// against it in constant time. This provides real authentication for
/// deployments that do not use the Kubernetes TokenReview API (`drone` mode,
/// bare-metal, local testing), closing the "presence-only" gap (B-4) where any
/// non-empty token was accepted.
pub const BIND_API_TOKEN_ENV: &str = "BIND_API_TOKEN";

/// Validate the presented Bearer token against the shared secret in
/// [`BIND_API_TOKEN_ENV`], if configured.
///
/// Returns `Ok(())` when the shared-secret mode is **not** configured (the env
/// var is unset or empty) so this layer is opt-in and composes with the
/// TokenReview layer. When configured, the comparison is constant-time to avoid
/// leaking the secret through response timing.
///
/// # Errors
/// Returns `Err` with a generic message when a shared secret is configured but
/// the presented token does not match.
pub(crate) fn validate_shared_secret(token: &str) -> Result<(), String> {
    let expected = std::env::var(BIND_API_TOKEN_ENV)
        .ok()
        .filter(|value| !value.is_empty());
    compare_shared_secret(token, expected.as_deref())
}

/// Pure constant-time comparison of a presented token against an optional
/// expected shared secret.
///
/// When `expected` is `None` the shared-secret mode is not configured and the
/// check is a no-op (`Ok`). When `Some`, the comparison is constant-time.
///
/// # Errors
/// Returns `Err` when a shared secret is configured but does not match.
pub(crate) fn compare_shared_secret(token: &str, expected: Option<&str>) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use subtle::ConstantTimeEq;

    let Some(expected) = expected else {
        return Ok(());
    };

    // Compare fixed-size SHA-256 digests rather than the raw bytes, so the
    // comparison time does not depend on the secret's length (A15). `ct_eq`
    // short-circuits on unequal lengths, which would otherwise leak whether the
    // presented token matched the secret's length; equal-length digests remove
    // that side channel while keeping the byte comparison constant-time.
    let token_hash = Sha256::digest(token.as_bytes());
    let expected_hash = Sha256::digest(expected.as_bytes());
    let matches: bool = token_hash.as_slice().ct_eq(expected_hash.as_slice()).into();
    if matches {
        Ok(())
    } else {
        Err("Invalid API token".to_string())
    }
}

/// Returns `true` if `host` denotes a loopback-only bind address.
///
/// Used by the startup guard: binding to loopback means the API is not reachable
/// from other pods/hosts, so weaker auth is tolerable for local development.
pub fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<std::net::IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

/// Decide whether bindcar is allowed to start given its authentication posture.
///
/// bindcar must not silently expose an unauthenticated (or presence-only) API on
/// a non-loopback interface (B-4). It may start when **any** of the following
/// holds:
/// - real authentication is active (`auth_enabled && has_real_auth`), or
/// - the bind address is loopback-only, or
/// - the operator explicitly accepted the risk (`insecure_override`).
///
/// # Arguments
/// * `auth_enabled` - Whether the auth middleware is applied (`!DISABLE_AUTH`).
/// * `has_real_auth` - Whether a real authenticator is configured (TokenReview
///   feature or a shared secret).
/// * `bind_host` - The host portion of the listen address.
/// * `insecure_override` - Whether the operator passed the explicit insecure override.
///
/// # Errors
/// Returns `Err` with an operator-facing message when the configuration would
/// expose an unauthenticated API on a non-loopback interface.
pub fn check_startup_auth_posture(
    auth_enabled: bool,
    has_real_auth: bool,
    bind_host: &str,
    insecure_override: bool,
) -> Result<(), String> {
    let secure = auth_enabled && has_real_auth;
    if secure || insecure_override || is_loopback_host(bind_host) {
        return Ok(());
    }

    Err(format!(
        "refusing to start: the API is bound to a non-loopback interface ({bind_host}) without \
         real authentication. Enable the Kubernetes TokenReview feature, set {BIND_API_TOKEN_ENV}, \
         bind to loopback, or pass --i-know-this-is-insecure to override."
    ))
}

/// Environment variable that explicitly opts into allow-all authorization when
/// the Kubernetes TokenReview feature is active but no namespace / service-account
/// allowlist is configured.
///
/// See [`check_authorization_posture`] (A2): without an allowlist, every
/// authenticated ServiceAccount in the cluster is authorized for full DNS
/// control, so bindcar refuses to start in that posture unless this is set.
pub const ALLOW_ANY_SERVICE_ACCOUNT_ENV: &str = "BIND_ALLOW_ANY_SERVICEACCOUNT";

/// Decide whether bindcar may start given its TokenReview authorization posture.
///
/// With the Kubernetes TokenReview feature active, an empty namespace /
/// service-account allowlist authorizes **every** authenticated ServiceAccount in
/// the cluster — a confused-deputy where any compromised pod's token grants full
/// create/delete/modify over DNS zones (A2). bindcar refuses to start in that
/// posture unless the operator explicitly accepts it, mirroring the
/// [`check_startup_auth_posture`] pattern.
///
/// # Arguments
/// * `authorization_restricted` - Whether an allowlist is configured
///   ([`TokenReviewConfig::is_authorization_restricted`]).
/// * `allow_any_override` - Whether the operator set [`ALLOW_ANY_SERVICE_ACCOUNT_ENV`].
///
/// # Errors
/// Returns `Err` with an operator-facing message when authorization is
/// unrestricted and the override was not set.
#[cfg(feature = "k8s-token-review")]
pub fn check_authorization_posture(
    authorization_restricted: bool,
    allow_any_override: bool,
) -> Result<(), String> {
    if authorization_restricted || allow_any_override {
        return Ok(());
    }

    Err(format!(
        "refusing to start: the Kubernetes TokenReview feature is enabled but neither \
         BIND_ALLOWED_NAMESPACES nor BIND_ALLOWED_SERVICE_ACCOUNTS is set, so ANY authenticated \
         ServiceAccount in the cluster would be authorized for full DNS control. Configure an \
         allowlist, or set {ALLOW_ANY_SERVICE_ACCOUNT_ENV}=true to explicitly accept allow-all."
    ))
}

/// Returns `true` if a real authenticator is configured at runtime: either the
/// Kubernetes TokenReview feature is compiled in, or a shared secret is set.
pub fn has_real_auth() -> bool {
    cfg!(feature = "k8s-token-review") || shared_secret_configured()
}

/// Returns `true` when shared-secret authentication is the selected mode — i.e.
/// [`BIND_API_TOKEN_ENV`] is set to a non-empty value.
///
/// Shared-secret and Kubernetes TokenReview are **mutually exclusive**: a single
/// Bearer token cannot be both the shared secret and a valid ServiceAccount
/// token. When a shared secret is configured it is the selected mode, so the
/// TokenReview path is not consulted at request time and the fail-closed
/// TokenReview authorization posture (A2) is not enforced at startup. TokenReview
/// is used only when no shared secret is set (and the feature is compiled in).
pub fn shared_secret_configured() -> bool {
    std::env::var(BIND_API_TOKEN_ENV)
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

/// Configuration for TokenReview security policies
#[cfg(feature = "k8s-token-review")]
#[derive(Debug, Clone)]
pub struct TokenReviewConfig {
    /// Expected audiences for token validation
    pub audiences: Vec<String>,
    /// Allowed namespaces (empty = allow all)
    pub allowed_namespaces: Vec<String>,
    /// Allowed service accounts in format "system:serviceaccount:namespace:name" (empty = allow all)
    pub allowed_service_accounts: Vec<String>,
}

#[cfg(feature = "k8s-token-review")]
impl TokenReviewConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let audiences = match env::var("BIND_TOKEN_AUDIENCES") {
            Ok(val) if !val.trim().is_empty() => val
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            _ => vec!["bindcar".to_string()],
        };

        let allowed_namespaces = env::var("BIND_ALLOWED_NAMESPACES")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let allowed_service_accounts = env::var("BIND_ALLOWED_SERVICE_ACCOUNTS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let config = Self {
            audiences,
            allowed_namespaces,
            allowed_service_accounts,
        };

        debug!("TokenReview config loaded: audiences={:?}, allowed_namespaces={:?}, allowed_service_accounts={:?}",
            config.audiences, config.allowed_namespaces, config.allowed_service_accounts);

        config
    }

    /// Check if a namespace is allowed
    pub(crate) fn is_namespace_allowed(&self, namespace: &str) -> bool {
        // Empty list means allow all
        if self.allowed_namespaces.is_empty() {
            return true;
        }
        self.allowed_namespaces.contains(&namespace.to_string())
    }

    /// Check if a service account is allowed
    pub(crate) fn is_service_account_allowed(&self, username: &str) -> bool {
        // Empty list means allow all
        if self.allowed_service_accounts.is_empty() {
            return true;
        }
        self.allowed_service_accounts
            .contains(&username.to_string())
    }

    /// Returns `true` if this configuration restricts authorization to an explicit
    /// allowlist (at least one of the namespace / service-account lists is
    /// non-empty).
    ///
    /// When this returns `false` every authenticated ServiceAccount in the cluster
    /// is authorized (allow-all) — the over-broad default that the startup guard
    /// [`check_authorization_posture`] refuses unless explicitly overridden (A2).
    pub fn is_authorization_restricted(&self) -> bool {
        !self.allowed_namespaces.is_empty() || !self.allowed_service_accounts.is_empty()
    }

    /// Extract namespace from service account username
    /// Format: "system:serviceaccount:namespace:name"
    pub(crate) fn extract_namespace(username: &str) -> Option<String> {
        let parts: Vec<&str> = username.split(':').collect();
        if parts.len() != 4 || parts[0] != "system" || parts[1] != "serviceaccount" {
            return None;
        }
        Some(parts[2].to_string())
    }
}

/// Describes how the Kubernetes client will be authenticated when performing TokenReview calls.
///
/// The mode is determined by environment variables at startup. When all three explicit
/// vars (`KUBE_API_SERVER`, `KUBE_TOKEN_PATH`, `KUBE_CA_CERT_PATH`) are present,
/// bindcar builds the client directly from those files. Otherwise it falls back to
/// `kube::Client::try_default()`, which checks `KUBECONFIG`, `~/.kube/config`, and then
/// the in-cluster ServiceAccount mount in order.
#[cfg(feature = "k8s-token-review")]
#[derive(Debug)]
pub enum KubeAuthMode {
    /// All three explicit env vars are set; use them to build the client.
    Explicit {
        server: String,
        token_path: String,
        ca_cert_path: String,
    },
    /// Fall back to `kube::Client::try_default()`.
    Default,
}

/// Inspect environment variables and return which Kubernetes auth mode should be used.
///
/// Returns `KubeAuthMode::Explicit` only when **all three** of `KUBE_API_SERVER`,
/// `KUBE_TOKEN_PATH`, and `KUBE_CA_CERT_PATH` are present. If only some are set a
/// warning is logged and `KubeAuthMode::Default` is returned so that existing
/// in-cluster and kubeconfig deployments are unaffected.
#[cfg(feature = "k8s-token-review")]
pub fn detect_kube_auth_mode() -> KubeAuthMode {
    let api_server = env::var("KUBE_API_SERVER").ok();
    let token_path = env::var("KUBE_TOKEN_PATH").ok();
    let ca_cert_path = env::var("KUBE_CA_CERT_PATH").ok();

    // Warn on partial configuration so operators notice misconfiguration early.
    let set_count = [&api_server, &token_path, &ca_cert_path]
        .iter()
        .filter(|v| v.is_some())
        .count();

    if set_count > 0 && set_count < 3 {
        warn!(
            "Partial KUBE_* env vars set ({}/3 present): \
             KUBE_API_SERVER={}, KUBE_TOKEN_PATH={}, KUBE_CA_CERT_PATH={}. \
             All three must be set for explicit auth. Falling back to try_default().",
            set_count,
            api_server.as_deref().unwrap_or("(not set)"),
            token_path.as_deref().unwrap_or("(not set)"),
            ca_cert_path.as_deref().unwrap_or("(not set)"),
        );
    }

    match (api_server, token_path, ca_cert_path) {
        (Some(server), Some(token_path), Some(ca_cert_path)) => KubeAuthMode::Explicit {
            server,
            token_path,
            ca_cert_path,
        },
        _ => KubeAuthMode::Default,
    }
}

/// Build a `kube::Client` from explicit file-based credentials.
///
/// Reads the token from `token_path` and the CA certificate from `ca_cert_path`,
/// then constructs an in-memory kubeconfig pointing at `server`.
///
/// # Errors
/// Returns an error string if either file cannot be read, the CA certificate is not
/// valid PEM, or the kube client cannot be initialized.
#[cfg(feature = "k8s-token-review")]
pub(crate) async fn build_explicit_kube_client(
    server: String,
    token_path: String,
    ca_cert_path: String,
) -> Result<Client, String> {
    // Validate token file is readable first so callers get a clear error message.
    // The content is not used here — kube will re-read the file on each API call via
    // token_file in the kubeconfig, which is correct for rotating SA tokens.
    tokio::fs::read_to_string(&token_path)
        .await
        .map_err(|e| format!("failed to read token file '{}': {}", token_path, e))?;

    // Validate CA certificate file is readable and contains a PEM certificate block.
    // kube defers TLS setup to the first API call, so we validate eagerly here to
    // surface misconfiguration at startup rather than on the first request.
    let ca_bytes = tokio::fs::read(&ca_cert_path).await.map_err(|e| {
        format!(
            "failed to read CA certificate file '{}': {}",
            ca_cert_path, e
        )
    })?;
    let ca_pem = String::from_utf8_lossy(&ca_bytes);
    if !ca_pem.contains("-----BEGIN CERTIFICATE-----") {
        return Err(format!(
            "CA certificate file '{}' does not contain a valid PEM certificate block",
            ca_cert_path
        ));
    }

    // Build an in-memory kubeconfig using file paths. kube will re-read the token
    // file on each API call (correct for short-lived, rotating SA tokens) and will
    // parse the CA cert PEM when building the TLS stack.
    let kubeconfig = Kubeconfig {
        clusters: vec![NamedCluster {
            name: "standalone".to_string(),
            cluster: Some(Cluster {
                server: Some(server),
                certificate_authority: Some(ca_cert_path),
                ..Default::default()
            }),
            // kube 4.0 added `other` (flattened unknown kubeconfig fields) to the
            // Named* wrappers; default it so we stay forward-compatible.
            ..Default::default()
        }],
        auth_infos: vec![NamedAuthInfo {
            name: "standalone".to_string(),
            auth_info: Some(AuthInfo {
                // Use token_file so the token is re-read on rotation, not embedded.
                token_file: Some(token_path),
                ..Default::default()
            }),
            ..Default::default()
        }],
        contexts: vec![NamedContext {
            name: "standalone".to_string(),
            context: Some(kube::config::Context {
                cluster: "standalone".to_string(),
                user: Some("standalone".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }],
        current_context: Some("standalone".to_string()),
        ..Default::default()
    };

    let config = Config::from_custom_kubeconfig(kubeconfig, &KubeConfigOptions::default())
        .await
        .map_err(|e| format!("failed to build Kubernetes client config: {}", e))?;

    Client::try_from(config).map_err(|e| format!("failed to create Kubernetes client: {}", e))
}

/// Process-wide cached `kube::Client`, built once on first use.
///
/// Rebuilding the client on every request re-read the token/CA files and
/// reconstructed the whole TLS stack, amplifying auth-path load onto the API
/// server (A8). `kube::Client` is cheaply cloneable (an `Arc` internally), so a
/// single shared instance is reused. A failed first init is not cached, so a
/// transient startup error can still be retried on the next request.
#[cfg(feature = "k8s-token-review")]
static KUBE_CLIENT: tokio::sync::OnceCell<Client> = tokio::sync::OnceCell::const_new();

/// Return the shared `kube::Client`, building it once on first call (A8).
#[cfg(feature = "k8s-token-review")]
async fn cached_kube_client() -> Result<Client, String> {
    KUBE_CLIENT
        .get_or_try_init(build_kube_client)
        .await
        .cloned()
}

/// Build a `kube::Client` using the resolved auth mode.
///
/// Priority order:
/// 1. Explicit env vars (`KUBE_API_SERVER` + `KUBE_TOKEN_PATH` + `KUBE_CA_CERT_PATH`)
/// 2. `KUBECONFIG` env / `~/.kube/config` / in-cluster SA mount (via `try_default`)
#[cfg(feature = "k8s-token-review")]
async fn build_kube_client() -> Result<Client, String> {
    match detect_kube_auth_mode() {
        KubeAuthMode::Explicit {
            server,
            token_path,
            ca_cert_path,
        } => {
            debug!(
                "Kubernetes auth mode: explicit (KUBE_API_SERVER={})",
                server
            );
            build_explicit_kube_client(server, token_path, ca_cert_path).await
        }
        KubeAuthMode::Default => {
            debug!("Kubernetes auth mode: try_default (KUBECONFIG / ~/.kube/config / in-cluster)");
            Client::try_default()
                .await
                .map_err(|e| format!("Failed to create Kubernetes client: {}", e))
        }
    }
}

/// Authentication middleware
///
/// Validates that the request includes a Bearer token in the Authorization header.
/// This token should be a Kubernetes ServiceAccount token.
///
/// # Headers
/// - `Authorization: Bearer <token>` - Required
///
/// # Errors
/// Returns 401 Unauthorized if:
/// - No Authorization header is present
/// - Authorization header is malformed
/// - Token is invalid (future implementation)
pub async fn authenticate(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<AuthError>)> {
    // Extract Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing Authorization header");
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "Missing Authorization header".to_string(),
                }),
            )
        })?;

    // Check Bearer token format
    if !auth_header.starts_with("Bearer ") {
        warn!("Invalid Authorization header format");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(AuthError {
                error: "Invalid Authorization header format. Expected: Bearer <token>".to_string(),
            }),
        ));
    }

    let token = &auth_header[7..]; // Skip "Bearer "

    // Basic validation: token should not be empty
    if token.is_empty() {
        warn!("Empty token in Authorization header");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(AuthError {
                error: "Empty token".to_string(),
            }),
        ));
    }

    // Shared-secret validation (constant-time). No-op unless BIND_API_TOKEN is
    // set, in which case the presented token MUST match — this closes the
    // presence-only gap (B-4) for non-TokenReview deployments.
    if let Err(e) = validate_shared_secret(token) {
        warn!("Shared-secret token validation failed");
        return Err((StatusCode::UNAUTHORIZED, Json(AuthError { error: e })));
    }

    // Validate token with Kubernetes TokenReview API — but only when TokenReview
    // is the active mode. A configured shared secret (BIND_API_TOKEN) selects
    // shared-secret auth, which is mutually exclusive with TokenReview (see
    // `shared_secret_configured`), so we do not also require the Bearer token to
    // be a valid ServiceAccount token.
    //
    // The detailed reason (namespace/SA/api error) is logged server-side only; the
    // client gets a single generic "Unauthorized" so it cannot distinguish
    // "valid token, wrong namespace" from "invalid token" from "apiserver
    // unreachable" — an authorization/identity-enumeration oracle (A7).
    #[cfg(feature = "k8s-token-review")]
    if !shared_secret_configured() {
        if let Err(e) = validate_token_with_k8s(token).await {
            warn!("Token validation failed: {}", e);
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "Unauthorized".to_string(),
                }),
            ));
        }
        debug!("Token validated with Kubernetes TokenReview API");
    }

    #[cfg(not(feature = "k8s-token-review"))]
    debug!("Token validation: basic mode (presence check only)");

    Ok(next.run(request).await)
}

/// Returns `true` if the audiences echoed back by a TokenReview
/// (`status.audiences`) are compatible with the audiences bindcar requested
/// (`spec.audiences`) — i.e. their intersection is non-empty.
///
/// The Kubernetes TokenReview contract states that a client which sets
/// `spec.audiences` **must** verify that a compatible identifier is returned in
/// `status.audiences`; otherwise it cannot tell whether the authenticator is
/// audience-aware. Trusting `status.authenticated == true` alone (A1) lets a
/// token minted for a *different* audience through — notably the
/// apiserver-audience ServiceAccount token auto-mounted into every pod, which
/// would otherwise defeat the `BIND_TOKEN_AUDIENCES` scoping entirely
/// (confused-deputy). An empty returned set is therefore treated as incompatible.
#[cfg(feature = "k8s-token-review")]
pub(crate) fn audiences_compatible(requested: &[String], returned: &[String]) -> bool {
    requested.iter().any(|a| returned.contains(a))
}

/// Validate a token using Kubernetes TokenReview API
///
/// This function sends the token to the Kubernetes API server for validation.
/// It verifies that the token is authentic, not expired, and belongs to a valid
/// service account.
///
/// Additionally validates:
/// - Token audience matches expected audiences
/// - Service account namespace is in allowed list (if configured)
/// - Service account name is in allowed list (if configured)
///
/// # Arguments
/// * `token` - The bearer token to validate
///
/// # Returns
/// * `Ok(())` if the token is valid and authorized
/// * `Err(String)` if validation fails
#[cfg(feature = "k8s-token-review")]
pub(crate) async fn validate_token_with_k8s(token: &str) -> Result<(), String> {
    // Load security configuration
    let config = TokenReviewConfig::from_env();

    // Reuse the process-wide cached client (built once) rather than rebuilding
    // the TLS stack on every request (A8).
    let client = cached_kube_client().await?;

    // Create TokenReview API client
    let token_reviews: Api<TokenReview> = Api::all(client);

    // Build TokenReview request with audience validation
    let audiences = if !config.audiences.is_empty() {
        Some(config.audiences.clone())
    } else {
        None
    };

    let token_review = TokenReview {
        metadata: Default::default(),
        spec: k8s_openapi::api::authentication::v1::TokenReviewSpec {
            token: Some(token.to_string()),
            audiences,
        },
        status: None,
    };

    // Submit TokenReview request
    let result = token_reviews
        .create(&Default::default(), &token_review)
        .await
        .map_err(|e| {
            error!("TokenReview API call failed: {}", e);
            format!("Failed to validate token with Kubernetes API: {}", e)
        })?;

    // Check if token is authenticated
    let status = result
        .status
        .ok_or_else(|| "TokenReview status not available".to_string())?;

    if status.authenticated != Some(true) {
        let error_msg = status
            .error
            .unwrap_or_else(|| "Token not authenticated".to_string());
        warn!("Token authentication failed: {}", error_msg);
        return Err(error_msg);
    }

    // Enforce audience binding (A1). Because we set `spec.audiences`, the
    // TokenReview contract requires confirming that a compatible audience is
    // echoed back in `status.audiences`; relying on `authenticated == true` alone
    // would accept a token minted for another audience (e.g. any pod's default
    // ServiceAccount token) and silently defeat BIND_TOKEN_AUDIENCES scoping.
    let returned_audiences = status.audiences.clone().unwrap_or_default();
    if !audiences_compatible(&config.audiences, &returned_audiences) {
        warn!(
            "TokenReview returned incompatible audiences (requested={:?}, returned={:?})",
            config.audiences, returned_audiences
        );
        return Err("Token audience is not valid for bindcar".to_string());
    }

    debug!("Token authenticated successfully");

    // Validate user information and authorization
    let user = status.user.ok_or_else(|| {
        warn!("TokenReview succeeded but no user information returned");
        "No user information in TokenReview response".to_string()
    })?;

    let username = user.username.as_deref().unwrap_or("");
    debug!("Authenticated user: {}", username);

    // Validate namespace restriction
    if let Some(namespace) = TokenReviewConfig::extract_namespace(username) {
        if !config.is_namespace_allowed(&namespace) {
            warn!(
                "ServiceAccount from unauthorized namespace: {} (from {})",
                namespace, username
            );
            return Err(format!(
                "ServiceAccount from unauthorized namespace: {}",
                namespace
            ));
        }
        debug!("Namespace {} is allowed", namespace);
    }

    // Validate service account allowlist
    if !config.is_service_account_allowed(username) {
        warn!("ServiceAccount not in allowlist: {}", username);
        return Err(format!("ServiceAccount not authorized: {}", username));
    }

    debug!("ServiceAccount {} is allowed", username);
    Ok(())
}
