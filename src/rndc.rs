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
use std::fs;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use crate::metrics;

/// RNDC configuration parsed from rndc.conf
#[derive(Debug, Clone)]
pub struct RndcConfig {
    pub server: String,
    pub algorithm: String,
    pub secret: String,
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

        // Validate algorithm - rndc crate v0.1.3 only supports these values
        let valid_algorithms = ["md5", "sha1", "sha224", "sha256", "sha384", "sha512"];

        if !valid_algorithms.contains(&algorithm.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid algorithm '{}'. Valid algorithms (without 'hmac-' prefix): {:?}",
                algorithm,
                valid_algorithms
            ));
        }

        let client = RndcClient::new(server, &algorithm, secret);

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
        let command = format!("addzone {} {}", zone_name, zone_config);
        self.execute(&command).await
    }

    /// Delete a zone
    pub async fn delzone(&self, zone_name: &str) -> Result<String> {
        let command = format!("delzone {}", zone_name);
        self.execute(&command).await
    }

    /// Reload a zone
    pub async fn reload(&self, zone_name: &str) -> Result<String> {
        let command = format!("reload {}", zone_name);
        self.execute(&command).await
    }

    /// Get zone status
    pub async fn zonestatus(&self, zone_name: &str) -> Result<String> {
        let command = format!("zonestatus {}", zone_name);
        self.execute(&command).await
    }

    /// Freeze a zone (disable dynamic updates)
    pub async fn freeze(&self, zone_name: &str) -> Result<String> {
        let command = format!("freeze {}", zone_name);
        self.execute(&command).await
    }

    /// Thaw a zone (enable dynamic updates)
    pub async fn thaw(&self, zone_name: &str) -> Result<String> {
        let command = format!("thaw {}", zone_name);
        self.execute(&command).await
    }

    /// Notify secondaries about zone changes
    pub async fn notify(&self, zone_name: &str) -> Result<String> {
        let command = format!("notify {}", zone_name);
        self.execute(&command).await
    }

    /// Force a zone retransfer from primary
    ///
    /// This command is used on secondary zones to discard the current zone data
    /// and initiate a fresh transfer from the primary server.
    pub async fn retransfer(&self, zone_name: &str) -> Result<String> {
        let command = format!("retransfer {}", zone_name);
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
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read rndc.conf from {}", path))?;

    info!("Parsing rndc.conf from {}", path);

    let mut algorithm = None;
    let mut secret = None;
    let mut server = "127.0.0.1:953".to_string();
    let mut default_key = None;
    let mut include_files = Vec::new();

    // First pass: parse main config and collect include directives
    for line in content.lines() {
        let line = line.trim();

        // Handle include directive (e.g., include "/etc/bind/rndc.key";)
        if line.starts_with("include") {
            if let Some(include_path) = extract_quoted_value(line) {
                debug!("Found include directive: {}", include_path);
                include_files.push(include_path);
            }
        }

        // Parse algorithm
        if line.contains("algorithm") {
            if let Some(algo) = extract_quoted_value(line) {
                algorithm = Some(algo);
            }
        }

        // Parse secret
        if line.contains("secret") {
            if let Some(sec) = extract_quoted_value(line) {
                secret = Some(sec);
            }
        }

        // Parse default-server from options block
        if line.contains("default-server") {
            if let Some(srv) = extract_value_after_whitespace(line) {
                server = if srv.contains(':') {
                    srv
                } else {
                    format!("{}:953", srv)
                };
            }
        }

        // Parse default-key from options block
        if line.contains("default-key") {
            if let Some(key) = extract_value_after_whitespace(line) {
                default_key = Some(key);
            }
        }
    }

    // If we don't have algorithm/secret yet, try parsing included files
    if algorithm.is_none() || secret.is_none() {
        for include_path in &include_files {
            info!("Parsing included file: {}", include_path);

            let include_content = match fs::read_to_string(include_path) {
                Ok(content) => content,
                Err(e) => {
                    warn!("Failed to read included file {}: {}", include_path, e);
                    continue;
                }
            };

            for line in include_content.lines() {
                let line = line.trim();

                if algorithm.is_none() && line.contains("algorithm") {
                    if let Some(algo) = extract_quoted_value(line) {
                        debug!("Found algorithm in {}: {}", include_path, algo);
                        algorithm = Some(algo);
                    }
                }

                if secret.is_none() && line.contains("secret") {
                    if let Some(sec) = extract_quoted_value(line) {
                        debug!("Found secret in {}", include_path);
                        secret = Some(sec);
                    }
                }

                // Also check for key name in included file
                if default_key.is_none() && line.starts_with("key") {
                    // Extract key name from: key "keyname" {
                    if let Some(key_name) = extract_quoted_value(line) {
                        default_key = Some(key_name);
                    }
                }
            }
        }
    }

    let algorithm = algorithm
        .ok_or_else(|| anyhow::anyhow!("No algorithm found in {} or included files", path))?;
    let secret =
        secret.ok_or_else(|| anyhow::anyhow!("No secret found in {} or included files", path))?;

    info!(
        "Parsed rndc configuration: server={}, algorithm={}, key={}",
        server,
        algorithm,
        default_key.unwrap_or_else(|| "unnamed".to_string())
    );

    Ok(RndcConfig {
        server,
        algorithm,
        secret,
    })
}

/// Extract quoted value from a line like: algorithm "hmac-sha256";
fn extract_quoted_value(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split('"').collect();
    if parts.len() >= 2 {
        Some(parts[1].to_string())
    } else {
        None
    }
}

/// Extract value after whitespace from a line like: default-server 127.0.0.1;
fn extract_value_after_whitespace(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        // Remove trailing semicolon if present
        Some(parts[1].trim_end_matches(';').to_string())
    } else {
        None
    }
}
