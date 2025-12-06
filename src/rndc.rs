// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! RNDC command execution
//!
//! This module executes rndc commands using the system's rndc binary.
//! The rndc binary must be configured with appropriate keys in /etc/bind/rndc.conf

use anyhow::{Context, Result};
use std::time::Instant;
use tokio::process::Command;
use tracing::{debug, error};

use crate::metrics;

/// RNDC command executor
pub struct RndcExecutor {
    pub(crate) rndc_path: String,
}

impl RndcExecutor {
    /// Create a new RNDC executor
    ///
    /// # Arguments
    /// * `rndc_path` - Path to the rndc binary (default: "/usr/sbin/rndc")
    pub fn new(rndc_path: Option<String>) -> Self {
        Self {
            rndc_path: rndc_path.unwrap_or_else(|| "/usr/sbin/rndc".to_string()),
        }
    }

    /// Execute an rndc command
    ///
    /// # Arguments
    /// * `args` - Command arguments (e.g., &["status"], &["addzone", "example.com", "{ ... }"])
    ///
    /// # Returns
    /// The stdout output from rndc on success
    ///
    /// # Errors
    /// Returns an error if the rndc command fails
    async fn execute(&self, args: &[&str]) -> Result<String> {
        debug!("Executing rndc command: {} {:?}", self.rndc_path, args);

        let start = Instant::now();
        let command_name = args.first().unwrap_or(&"unknown");

        let output = Command::new(&self.rndc_path)
            .args(args)
            .output()
            .await
            .context("Failed to execute rndc command")?;

        let duration = start.elapsed().as_secs_f64();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("RNDC command failed: {}", stderr);
            metrics::record_rndc_command(command_name, false, duration);
            return Err(anyhow::anyhow!("RNDC command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        debug!("RNDC command output: {}", stdout);
        metrics::record_rndc_command(command_name, true, duration);
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

impl Clone for RndcExecutor {
    fn clone(&self) -> Self {
        Self {
            rndc_path: self.rndc_path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rndc_executor_creation() {
        let executor = RndcExecutor::new(None);
        assert_eq!(executor.rndc_path, "/usr/sbin/rndc");

        let executor_custom = RndcExecutor::new(Some("/custom/path/rndc".to_string()));
        assert_eq!(executor_custom.rndc_path, "/custom/path/rndc");
    }

    #[test]
    fn test_rndc_executor_clone() {
        let executor = RndcExecutor::new(Some("/custom/path/rndc".to_string()));
        let cloned = executor.clone();
        assert_eq!(cloned.rndc_path, "/custom/path/rndc");
    }

    // Note: Integration tests that actually execute rndc commands require
    // a running BIND9 instance with rndc configured. These should be in
    // integration tests, not unit tests.
}
