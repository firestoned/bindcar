// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! RNDC command execution
//!
//! This module executes rndc commands locally using the shell.
//! The rndc configuration and key files are mounted into the container
//! at `/etc/bind/rndc.conf` and `/etc/bind/rndc.key`.

use anyhow::{Context, Result};
use tokio::process::Command;
use tracing::{debug, error};

/// Path to rndc binary
const RNDC_BIN: &str = "/usr/sbin/rndc";

/// RNDC command executor
pub struct RndcExecutor;

impl RndcExecutor {
    /// Create a new RNDC executor
    pub fn new() -> Self {
        Self
    }

    /// Execute an rndc command
    ///
    /// # Arguments
    /// * `args` - Command arguments (e.g., ["status"], ["addzone", "example.com", "{ ... }"])
    ///
    /// # Returns
    /// The stdout output from rndc on success
    ///
    /// # Errors
    /// Returns an error if the rndc command fails or returns non-zero exit code
    pub async fn execute(&self, args: &[&str]) -> Result<String> {
        debug!("Executing rndc command: {:?}", args);

        let output = Command::new(RNDC_BIN)
            .args(args)
            .output()
            .await
            .context("Failed to execute rndc command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            error!(
                "RNDC command failed: {:?}\nstdout: {}\nstderr: {}",
                args, stdout, stderr
            );
            return Err(anyhow::anyhow!(
                "RNDC command failed: {}",
                if !stderr.is_empty() {
                    stderr.as_ref()
                } else {
                    stdout.as_ref()
                }
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        debug!("RNDC command output: {}", stdout);

        Ok(stdout)
    }

    /// Get server status
    pub async fn status(&self) -> Result<String> {
        self.execute(&["status"]).await
    }

    /// Add a zone
    ///
    /// # Arguments
    /// * `zone_name` - Name of the zone (e.g., "example.com")
    /// * `zone_config` - Zone configuration block (e.g., "{ type master; file \"/var/cache/bind/example.com.zone\"; }")
    pub async fn addzone(&self, zone_name: &str, zone_config: &str) -> Result<String> {
        self.execute(&["addzone", zone_name, zone_config]).await
    }

    /// Delete a zone
    pub async fn delzone(&self, zone_name: &str) -> Result<String> {
        self.execute(&["delzone", zone_name]).await
    }

    /// Reload a zone
    pub async fn reload(&self, zone_name: &str) -> Result<String> {
        self.execute(&["reload", zone_name]).await
    }

    /// Get zone status
    pub async fn zonestatus(&self, zone_name: &str) -> Result<String> {
        self.execute(&["zonestatus", zone_name]).await
    }

    /// Freeze a zone (disable dynamic updates)
    pub async fn freeze(&self, zone_name: &str) -> Result<String> {
        self.execute(&["freeze", zone_name]).await
    }

    /// Thaw a zone (enable dynamic updates)
    pub async fn thaw(&self, zone_name: &str) -> Result<String> {
        self.execute(&["thaw", zone_name]).await
    }

    /// Notify secondaries about zone changes
    pub async fn notify(&self, zone_name: &str) -> Result<String> {
        self.execute(&["notify", zone_name]).await
    }
}

impl Default for RndcExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rndc_executor_creation() {
        let executor = RndcExecutor::new();
        // Verify executor can be created
        assert!(std::mem::size_of_val(&executor) == 0); // Zero-sized type
    }

    #[test]
    fn test_rndc_executor_default() {
        let executor = RndcExecutor::default();
        assert!(std::mem::size_of_val(&executor) == 0);
    }

    // Note: Integration tests that actually execute rndc commands require
    // a running BIND9 instance with rndc configured. These should be in
    // integration tests, not unit tests.
}
