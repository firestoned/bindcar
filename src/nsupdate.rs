// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! nsupdate command executor for dynamic DNS updates
//!
//! This module provides programmatic access to BIND9's nsupdate utility for
//! performing dynamic DNS record updates using the DNS UPDATE protocol (RFC 2136).
//!
//! # Features
//!
//! - TSIG authentication support
//! - Add, remove, and update individual DNS records
//! - Async command execution with tokio
//! - Comprehensive error handling and parsing

use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info};

use crate::metrics;

/// nsupdate command executor
///
/// Manages dynamic DNS updates via the nsupdate command-line tool.
/// Supports TSIG authentication for secure updates.
#[derive(Clone)]
pub struct NsupdateExecutor {
    /// Optional TSIG key name for authentication
    tsig_key_name: Option<String>,
    /// Optional TSIG algorithm (e.g., "HMAC-SHA256")
    tsig_algorithm: Option<String>,
    /// Optional TSIG secret (base64-encoded)
    tsig_secret: Option<String>,
    /// DNS server address
    server: String,
    /// DNS server port
    port: u16,
}

impl NsupdateExecutor {
    /// Create a new nsupdate executor
    ///
    /// # Arguments
    ///
    /// * `server` - DNS server address (e.g., "127.0.0.1")
    /// * `port` - DNS server port (typically 53)
    /// * `tsig_key_name` - Optional TSIG key name
    /// * `tsig_algorithm` - Optional TSIG algorithm
    /// * `tsig_secret` - Optional TSIG secret (base64-encoded)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use bindcar::nsupdate::NsupdateExecutor;
    ///
    /// let executor = NsupdateExecutor::new(
    ///     "127.0.0.1".to_string(),
    ///     53,
    ///     Some("update-key".to_string()),
    ///     Some("HMAC-SHA256".to_string()),
    ///     Some("dGVzdC1zZWNyZXQ=".to_string()),
    /// )?;
    /// ```
    pub fn new(
        server: String,
        port: u16,
        tsig_key_name: Option<String>,
        tsig_algorithm: Option<String>,
        tsig_secret: Option<String>,
    ) -> Result<Self> {
        info!(
            "Creating nsupdate executor for {}:{} with TSIG: {}",
            server,
            port,
            tsig_key_name.is_some()
        );

        Ok(Self {
            tsig_key_name,
            tsig_algorithm,
            tsig_secret,
            server,
            port,
        })
    }

    /// Execute nsupdate commands
    ///
    /// # Arguments
    ///
    /// * `commands` - nsupdate commands as a string (one command per line)
    ///
    /// # Returns
    ///
    /// Success or error message from nsupdate
    async fn execute(&self, commands: &str) -> Result<String> {
        let start = Instant::now();

        debug!("Executing nsupdate commands:\n{}", commands);

        let mut cmd = tokio::process::Command::new("nsupdate");

        // Add TSIG authentication if configured
        if let (Some(ref key_name), Some(ref algorithm), Some(ref secret)) =
            (&self.tsig_key_name, &self.tsig_algorithm, &self.tsig_secret)
        {
            // nsupdate -y format: algorithm:keyname:secret
            let auth = format!("{}:{}:{}", algorithm, key_name, secret);
            cmd.arg("-y").arg(&auth);
            debug!("Using TSIG authentication with key: {}", key_name);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().context("Failed to spawn nsupdate process")?;

        // Write commands to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(commands.as_bytes())
                .await
                .context("Failed to write to nsupdate stdin")?;
            stdin.flush().await.context("Failed to flush stdin")?;
        }

        // Wait for completion and capture output
        let output = child
            .wait_with_output()
            .await
            .context("Failed to wait for nsupdate")?;

        let duration = start.elapsed().as_secs_f64();

        // Handle errors
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error_msg = parse_nsupdate_error(&stderr);
            error!("nsupdate failed: {}", error_msg);
            metrics::record_nsupdate_command("update", false, duration);
            return Err(anyhow::anyhow!("nsupdate failed: {}", error_msg));
        }

        metrics::record_nsupdate_command("update", true, duration);

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        debug!("nsupdate completed successfully in {:.3}s", duration);

        Ok(stdout)
    }

    /// Add a DNS record
    ///
    /// # Arguments
    ///
    /// * `zone` - Zone name (e.g., "example.com")
    /// * `name` - Record name (FQDN, e.g., "www.example.com.")
    /// * `ttl` - Time-to-live in seconds
    /// * `record_type` - Record type (e.g., "A", "AAAA", "CNAME")
    /// * `value` - Record value (e.g., "192.0.2.1")
    ///
    /// # Example
    ///
    /// ```ignore
    /// executor.add_record(
    ///     "example.com",
    ///     "www.example.com.",
    ///     3600,
    ///     "A",
    ///     "192.0.2.1"
    /// ).await?;
    /// ```
    pub async fn add_record(
        &self,
        zone: &str,
        name: &str,
        ttl: u32,
        record_type: &str,
        value: &str,
    ) -> Result<String> {
        info!(
            "Adding {} record: {} -> {} (TTL: {})",
            record_type, name, value, ttl
        );

        let commands = format!(
            "server {} {}\nzone {}\nupdate add {} {} IN {} {}\nsend\n",
            self.server, self.port, zone, name, ttl, record_type, value
        );

        self.execute(&commands).await
    }

    /// Remove a DNS record
    ///
    /// # Arguments
    ///
    /// * `zone` - Zone name (e.g., "example.com")
    /// * `name` - Record name (FQDN, e.g., "www.example.com.")
    /// * `record_type` - Record type (e.g., "A")
    /// * `value` - Record value to remove (e.g., "192.0.2.1"). If empty, removes all records of this type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Remove specific record
    /// executor.remove_record(
    ///     "example.com",
    ///     "www.example.com.",
    ///     "A",
    ///     "192.0.2.1"
    /// ).await?;
    ///
    /// // Remove all A records for www
    /// executor.remove_record(
    ///     "example.com",
    ///     "www.example.com.",
    ///     "A",
    ///     ""
    /// ).await?;
    /// ```
    pub async fn remove_record(
        &self,
        zone: &str,
        name: &str,
        record_type: &str,
        value: &str,
    ) -> Result<String> {
        info!(
            "Removing {} record: {} {} {}",
            record_type,
            name,
            if value.is_empty() { "(all)" } else { value },
            ""
        );

        // Build delete command - with or without value
        let delete_cmd = if value.is_empty() {
            // Remove all records of this type
            format!("update delete {} {}", name, record_type)
        } else {
            // Remove specific record
            format!("update delete {} {} {}", name, record_type, value)
        };

        let commands = format!(
            "server {} {}\nzone {}\n{}\nsend\n",
            self.server, self.port, zone, delete_cmd
        );

        self.execute(&commands).await
    }

    /// Update a DNS record (atomic delete + add)
    ///
    /// # Arguments
    ///
    /// * `zone` - Zone name (e.g., "example.com")
    /// * `name` - Record name (FQDN, e.g., "www.example.com.")
    /// * `ttl` - New time-to-live in seconds
    /// * `record_type` - Record type (e.g., "A")
    /// * `old_value` - Current record value (e.g., "192.0.2.1")
    /// * `new_value` - New record value (e.g., "192.0.2.2")
    ///
    /// # Example
    ///
    /// ```ignore
    /// executor.update_record(
    ///     "example.com",
    ///     "www.example.com.",
    ///     3600,
    ///     "A",
    ///     "192.0.2.1",
    ///     "192.0.2.2"
    /// ).await?;
    /// ```
    pub async fn update_record(
        &self,
        zone: &str,
        name: &str,
        ttl: u32,
        record_type: &str,
        old_value: &str,
        new_value: &str,
    ) -> Result<String> {
        info!(
            "Updating {} record: {} from {} to {} (TTL: {})",
            record_type, name, old_value, new_value, ttl
        );

        // Atomic update: delete old, add new in single transaction
        let commands = format!(
            "server {} {}\nzone {}\nupdate delete {} {} {}\nupdate add {} {} IN {} {}\nsend\n",
            self.server,
            self.port,
            zone,
            name,
            record_type,
            old_value,
            name,
            ttl,
            record_type,
            new_value
        );

        self.execute(&commands).await
    }
}

/// Parse nsupdate error messages into human-readable format
///
/// Maps common nsupdate error codes to helpful messages
fn parse_nsupdate_error(stderr: &str) -> String {
    if stderr.contains("REFUSED") {
        "Zone refused the update (check allow-update configuration)".to_string()
    } else if stderr.contains("NOTAUTH") {
        "Not authorized (check TSIG key configuration)".to_string()
    } else if stderr.contains("SERVFAIL") {
        "Server failure (check BIND9 logs)".to_string()
    } else if stderr.contains("NOTZONE") {
        "Zone not found on server".to_string()
    } else if stderr.contains("FORMERR") {
        "Format error (check record syntax)".to_string()
    } else if stderr.contains("NXDOMAIN") {
        "Domain name does not exist".to_string()
    } else {
        // Return raw stderr if no specific error matched
        stderr.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nsupdate_error_refused() {
        let stderr = "update failed: REFUSED\n";
        assert_eq!(
            parse_nsupdate_error(stderr),
            "Zone refused the update (check allow-update configuration)"
        );
    }

    #[test]
    fn test_parse_nsupdate_error_notauth() {
        let stderr = "update failed: NOTAUTH\n";
        assert_eq!(
            parse_nsupdate_error(stderr),
            "Not authorized (check TSIG key configuration)"
        );
    }

    #[test]
    fn test_parse_nsupdate_error_servfail() {
        let stderr = "update failed: SERVFAIL\n";
        assert_eq!(
            parse_nsupdate_error(stderr),
            "Server failure (check BIND9 logs)"
        );
    }

    #[test]
    fn test_parse_nsupdate_error_notzone() {
        let stderr = "update failed: NOTZONE\n";
        assert_eq!(parse_nsupdate_error(stderr), "Zone not found on server");
    }

    #[test]
    fn test_parse_nsupdate_error_unknown() {
        let stderr = "some other error\n";
        assert_eq!(parse_nsupdate_error(stderr), "some other error");
    }

    #[test]
    fn test_new_executor_with_tsig() {
        let executor = NsupdateExecutor::new(
            "127.0.0.1".to_string(),
            53,
            Some("test-key".to_string()),
            Some("HMAC-SHA256".to_string()),
            Some("dGVzdA==".to_string()),
        );
        assert!(executor.is_ok());
    }

    #[test]
    fn test_new_executor_without_tsig() {
        let executor = NsupdateExecutor::new("127.0.0.1".to_string(), 53, None, None, None);
        assert!(executor.is_ok());
    }
}
