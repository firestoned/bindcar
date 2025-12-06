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
use tracing::{debug, error, info};

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
    /// * `algorithm` - HMAC algorithm (e.g., "sha256", "md5", "sha1", "sha224", "sha384", "sha512")
    /// * `secret` - Base64-encoded RNDC secret key
    ///
    /// # Returns
    /// A new RndcExecutor instance
    pub fn new(server: String, algorithm: String, secret: String) -> Result<Self> {
        debug!(
            "Creating RNDC client for server {} with algorithm {}",
            server, algorithm
        );

        let client = RndcClient::new(&server, &algorithm, &secret);

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

        match result {
            Ok(rndc_result) => {
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
            Err(e) => {
                let error_msg = format!("RNDC command '{}' failed: {}", command_name, e);
                error!("{}", error_msg);
                metrics::record_rndc_command(command_name, false, duration);
                Err(anyhow::anyhow!("{}", error_msg))
            }
        }
    }

    /// Get server status
    pub async fn status(&self) -> Result<String> {
        self.execute("status").await
    }

    /// Add a zone
    ///
    /// # Arguments
    /// * `zone_name` - Name of the zone (e.g., "example.com")
    /// * `zone_config` - Zone configuration block (e.g., "{ type master; file \"/var/cache/bind/example.com.zone\"; }")
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

    // Simple parser for rndc.conf format
    // Format: key "name" { algorithm "hmac-sha256"; secret "base64string"; };
    let mut algorithm = None;
    let mut secret = None;
    let mut server = "127.0.0.1:953".to_string(); // Default server
    let mut default_key = None;

    // Parse key blocks
    for line in content.lines() {
        let line = line.trim();

        // Parse algorithm
        if line.contains("algorithm") {
            if let Some(algo) = extract_quoted_value(line) {
                // Remove "hmac-" prefix if present for compatibility
                algorithm = Some(algo.trim_start_matches("hmac-").to_string());
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
                // Add default port if not specified
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

    let algorithm = algorithm.ok_or_else(|| anyhow::anyhow!("No algorithm found in {}", path))?;
    let secret = secret.ok_or_else(|| anyhow::anyhow!("No secret found in {}", path))?;

    info!(
        "Parsed rndc.conf: server={}, algorithm={}, key={}",
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
