// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! RNDC command execution using native RNDC protocol
//!
//! This module communicates with BIND9 using the RNDC protocol directly,
//! rather than shelling out to the rndc binary. This provides:
//! - Better error handling with structured responses
//! - No subprocess overhead
//! - Native async support
//! - Direct access to error messages from BIND9

use anyhow::{Context, Result};
use rndc::RndcClient;
use std::time::Instant;
use tracing::{debug, error, info};

use crate::metrics;

/// RNDC configuration parsed from rndc.conf
#[derive(Clone)]
pub struct RndcConfig {
    pub server: String,
    pub algorithm: String,
    pub secret: String,
}

// Manual `Debug` that redacts the TSIG secret (A4). The derived impl would print
// the plaintext key on any `{:?}` / panic backtrace / `.context()`; redacting at
// the type keeps that credential out of logs and diagnostics entirely.
impl std::fmt::Debug for RndcConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RndcConfig")
            .field("server", &self.server)
            .field("algorithm", &self.algorithm)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

/// HMAC algorithms accepted for the RNDC control channel (SHA-2 family only).
/// HMAC-MD5 / HMAC-SHA1 are deprecated and intentionally excluded (A12).
const ACCEPTED_RNDC_ALGORITHMS: &[&str] = &["sha224", "sha256", "sha384", "sha512"];

/// Maximum length of a DNS zone name (RFC 1035 total name length).
const MAX_ZONE_NAME_LEN: usize = 253;

/// Default RNDC port when the address carries no explicit port.
const DEFAULT_RNDC_PORT: u16 = 953;

/// Re-validate a zone name at the RNDC sink (defense-in-depth, A9).
///
/// Every executor method interpolates a caller-supplied zone name into an RNDC
/// control-channel command line. The HTTP handlers validate first, but this
/// guard-at-the-sink means a missing caller-side check (which has regressed
/// before, see commit `c514ff9`) cannot inject extra RNDC arguments. Only the
/// DNS charset `[A-Za-z0-9._-]` (bounded length, no `..`) is accepted.
///
/// # Errors
/// Returns an error if the name is empty, too long, contains `..`, or has a
/// character outside the permitted set.
pub(crate) fn validate_rndc_zone_name(zone_name: &str) -> Result<()> {
    if zone_name.is_empty() || zone_name.len() > MAX_ZONE_NAME_LEN {
        return Err(anyhow::anyhow!("invalid zone name length"));
    }
    if zone_name.contains("..") {
        return Err(anyhow::anyhow!("zone name must not contain '..'"));
    }
    if let Some(bad) = zone_name
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')))
    {
        return Err(anyhow::anyhow!(
            "zone name contains invalid character: {:?}",
            bad
        ));
    }
    Ok(())
}

/// RNDC command executor using native protocol
pub struct RndcExecutor {
    client: RndcClient,
}

impl RndcExecutor {
    /// Create a new RNDC executor
    ///
    /// # Arguments
    /// * `server` - RNDC server address (e.g., "127.0.0.1:953")
    /// * `algorithm` - HMAC algorithm, accepts both formats:
    ///   - With prefix: "hmac-md5", "hmac-sha1", "hmac-sha224", "hmac-sha256", "hmac-sha384", "hmac-sha512"
    ///   - Without prefix: "md5", "sha1", "sha224", "sha256", "sha384", "sha512"
    /// * `secret` - Base64-encoded RNDC secret key
    ///
    /// # Returns
    /// A new RndcExecutor instance
    pub fn new(server: String, algorithm: String, secret: String) -> Result<Self> {
        // Trim whitespace from all parameters to handle environment variable issues
        let server = server.trim();
        let mut algorithm = algorithm.trim().to_string();
        let secret = secret.trim();

        // The rndc crate v0.1.3 only accepts algorithms WITHOUT the "hmac-" prefix
        // Strip it if present (rndc.conf files typically use "hmac-sha256" format)
        if algorithm.starts_with("hmac-") {
            algorithm = algorithm.trim_start_matches("hmac-").to_string();
        }

        debug!("Using algorithm: {} for server: {}", algorithm, server);

        // Validate algorithm. HMAC-MD5 and HMAC-SHA1 are deprecated and rejected
        // for the RNDC control channel in this zero-trust posture (A12); only the
        // SHA-2 family is accepted.
        if !ACCEPTED_RNDC_ALGORITHMS.contains(&algorithm.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid or deprecated algorithm '{}'. Accepted algorithms (without 'hmac-' prefix): {:?}",
                algorithm,
                ACCEPTED_RNDC_ALGORITHMS
            ));
        }

        let client = RndcClient::new(server, &algorithm, secret)?;

        Ok(Self { client })
    }

    /// Execute an RNDC command
    ///
    /// # Arguments
    /// * `command` - Command string (e.g., "status", "reload example.com")
    ///
    /// # Returns
    /// The response text from RNDC on success
    ///
    /// # Errors
    /// Returns an error if the RNDC command fails, including detailed error
    /// information from the BIND9 server
    async fn execute(&self, command: &str) -> Result<String> {
        debug!("Executing RNDC command: {}", command);

        let start = Instant::now();
        let command_name = command.split_whitespace().next().unwrap_or("unknown");

        // Execute RNDC command using native protocol
        let result = tokio::task::spawn_blocking({
            let client = self.client.clone();
            let command = command.to_string();
            move || client.rndc_command(&command)
        })
        .await
        .with_context(|| {
            format!(
                "Failed to execute RNDC command '{}': task join error",
                command_name
            )
        })?;

        let duration = start.elapsed().as_secs_f64();

        let rndc_result = result.map_err(|e| {
            let error_msg = format!("RNDC command '{}' failed: {}", command_name, e);
            error!("{}", error_msg);
            metrics::record_rndc_command(command_name, false, duration);
            anyhow::anyhow!("{}", error_msg)
        })?;

        // Check if the RNDC result contains an error
        if let Some(err) = &rndc_result.err {
            let error_msg = format!("RNDC command '{}' failed: {}", command_name, err);
            error!("{}", error_msg);
            metrics::record_rndc_command(command_name, false, duration);
            return Err(anyhow::anyhow!("{}", error_msg));
        }

        // Success - return the text response
        let response = rndc_result.text.unwrap_or_default();
        debug!("RNDC command '{}' succeeded: {}", command_name, response);
        metrics::record_rndc_command(command_name, true, duration);
        Ok(response)
    }

    /// Get server status
    pub async fn status(&self) -> Result<String> {
        self.execute("status").await
    }

    /// Add a zone
    ///
    /// # Arguments
    /// * `zone_name` - Name of the zone (e.g., "example.com")
    /// * `zone_config` - Zone configuration block (e.g., "{ type primary; file \"/var/cache/bind/example.com.zone\"; }")
    pub async fn addzone(&self, zone_name: &str, zone_config: &str) -> Result<String> {
        validate_rndc_zone_name(zone_name)?;
        let command = format!("addzone {} {}", zone_name, zone_config);
        self.execute(&command).await
    }

    /// Delete a zone
    pub async fn delzone(&self, zone_name: &str) -> Result<String> {
        validate_rndc_zone_name(zone_name)?;
        let command = format!("delzone {}", zone_name);
        self.execute(&command).await
    }

    /// Reload a zone
    pub async fn reload(&self, zone_name: &str) -> Result<String> {
        validate_rndc_zone_name(zone_name)?;
        let command = format!("reload {}", zone_name);
        self.execute(&command).await
    }

    /// Get zone status
    pub async fn zonestatus(&self, zone_name: &str) -> Result<String> {
        validate_rndc_zone_name(zone_name)?;
        let command = format!("zonestatus {}", zone_name);
        self.execute(&command).await
    }

    /// Freeze a zone (disable dynamic updates)
    pub async fn freeze(&self, zone_name: &str) -> Result<String> {
        validate_rndc_zone_name(zone_name)?;
        let command = format!("freeze {}", zone_name);
        self.execute(&command).await
    }

    /// Thaw a zone (enable dynamic updates)
    pub async fn thaw(&self, zone_name: &str) -> Result<String> {
        validate_rndc_zone_name(zone_name)?;
        let command = format!("thaw {}", zone_name);
        self.execute(&command).await
    }

    /// Notify secondaries about zone changes
    pub async fn notify(&self, zone_name: &str) -> Result<String> {
        validate_rndc_zone_name(zone_name)?;
        let command = format!("notify {}", zone_name);
        self.execute(&command).await
    }

    /// Force a zone retransfer from primary
    ///
    /// This command is used on secondary zones to discard the current zone data
    /// and initiate a fresh transfer from the primary server.
    pub async fn retransfer(&self, zone_name: &str) -> Result<String> {
        validate_rndc_zone_name(zone_name)?;
        let command = format!("retransfer {}", zone_name);
        self.execute(&command).await
    }

    /// Modify a zone configuration
    ///
    /// # Arguments
    /// * `zone_name` - Name of the zone (e.g., "example.com")
    /// * `zone_config` - Zone configuration block (e.g., "{ also-notify { 10.0.0.1; }; allow-transfer { 10.0.0.2; }; }")
    pub async fn modzone(&self, zone_name: &str, zone_config: &str) -> Result<String> {
        validate_rndc_zone_name(zone_name)?;
        let command = format!("modzone {} {}", zone_name, zone_config);
        self.execute(&command).await
    }

    /// Show zone configuration
    ///
    /// # Arguments
    /// * `zone_name` - Name of the zone (e.g., "example.com")
    ///
    /// # Returns
    /// The zone configuration in BIND9 format
    pub async fn showzone(&self, zone_name: &str) -> Result<String> {
        validate_rndc_zone_name(zone_name)?;
        let command = format!("showzone {}", zone_name);
        self.execute(&command).await
    }
}

impl Clone for RndcExecutor {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

/// Parse RNDC configuration from rndc.conf file
///
/// # Arguments
/// * `path` - Path to rndc.conf file (typically /etc/bind/rndc.conf or /etc/rndc.conf)
///
/// # Returns
/// RndcConfig with server, algorithm, and secret
///
/// # Errors
/// Returns an error if the file cannot be read or parsed
pub fn parse_rndc_conf(path: &str) -> Result<RndcConfig> {
    use crate::rndc_conf_parser::parse_rndc_conf_file;
    use std::path::Path;

    info!("Parsing rndc.conf from {}", path);

    // Use new parser
    let conf_file = parse_rndc_conf_file(Path::new(path))
        .map_err(|e| anyhow::anyhow!("Failed to parse rndc.conf: {}", e))?;

    // Select the key. Prefer an explicit `default-key`; otherwise pick
    // deterministically and refuse to guess when the choice is ambiguous (A18) —
    // the previous `keys().next()` returned an arbitrary HashMap entry, so with
    // multiple keys the client could authenticate with an unintended one.
    let default_key_name = match conf_file.options.default_key.clone() {
        Some(name) => Some(name),
        None => {
            if conf_file.keys.len() > 1 {
                return Err(anyhow::anyhow!(
                    "rndc.conf defines {} keys but no `default-key`; specify one explicitly",
                    conf_file.keys.len()
                ));
            }
            let mut names: Vec<&String> = conf_file.keys.keys().collect();
            names.sort();
            names.first().map(|name| (*name).clone())
        }
    };

    let key_block = if let Some(ref key_name) = default_key_name {
        conf_file
            .keys
            .get(key_name)
            .ok_or_else(|| anyhow::anyhow!("Default key '{}' not found", key_name))?
    } else {
        return Err(anyhow::anyhow!("No keys found in configuration"));
    };

    // Reject an empty secret (A19): a key block with a missing/blank secret would
    // otherwise flow into RndcClient with no credential, masking a
    // misconfiguration that must fail fast.
    if key_block.secret.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "key '{}' has an empty secret",
            default_key_name.as_deref().unwrap_or("unnamed")
        ));
    }

    // Extract server address from options or use default
    let server = conf_file
        .options
        .default_server
        .clone()
        .unwrap_or_else(|| "127.0.0.1".to_string());

    // Add port if not present
    let server = if server.contains(':') {
        server
    } else {
        let port = conf_file.options.default_port.unwrap_or(DEFAULT_RNDC_PORT);
        format!("{}:{}", server, port)
    };

    info!(
        "Parsed rndc configuration: server={}, algorithm={}, key={}",
        server,
        key_block.algorithm,
        default_key_name.unwrap_or_else(|| "unnamed".to_string())
    );

    Ok(RndcConfig {
        server,
        algorithm: key_block.algorithm.clone(),
        secret: key_block.secret.clone(),
    })
}
