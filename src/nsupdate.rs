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
use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info};

use crate::metrics;

/// Maximum length of a TSIG key name (matches the DNS name length limit).
const MAX_TSIG_KEY_NAME_LEN: usize = 253;

/// HMAC algorithms accepted in a TSIG key file.
const ALLOWED_TSIG_ALGORITHMS: &[&str] = &[
    "hmac-md5",
    "hmac-sha1",
    "hmac-sha224",
    "hmac-sha256",
    "hmac-sha384",
    "hmac-sha512",
];

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
    /// Force TCP transport (nsupdate -v); required in environments where
    /// UDP is unreliable, e.g., Docker Desktop on macOS.
    use_tcp: bool,
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

        let use_tcp = std::env::var("NSUPDATE_TCP")
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);

        Ok(Self {
            tsig_key_name,
            tsig_algorithm,
            tsig_secret,
            server,
            port,
            use_tcp,
        })
    }

    /// Create a private (mode `0600`) temporary key file holding the TSIG key,
    /// for use with `nsupdate -k`.
    ///
    /// Returns `None` when TSIG is not configured. The file is deleted when the
    /// returned guard is dropped — callers must keep it alive until nsupdate has
    /// finished reading it.
    ///
    /// Passing the key via `-k <file>` instead of `-y algo:name:secret` keeps the
    /// secret out of the process argument vector, which is world-readable through
    /// `/proc/<pid>/cmdline` and process listings (B-7).
    ///
    /// # Errors
    /// Returns an error if the key material fails validation or the file cannot
    /// be created or written.
    pub(crate) fn create_tsig_key_file(&self) -> Result<Option<tempfile::NamedTempFile>> {
        let (Some(key_name), Some(algorithm), Some(secret)) = (
            self.tsig_key_name.as_deref(),
            self.tsig_algorithm.as_deref(),
            self.tsig_secret.as_deref(),
        ) else {
            return Ok(None);
        };

        let content = build_tsig_key_file_content(key_name, algorithm, secret)?;

        // NamedTempFile is created with mode 0600 on Unix.
        let mut keyfile =
            tempfile::NamedTempFile::new().context("Failed to create temporary TSIG key file")?;
        keyfile
            .write_all(content.as_bytes())
            .context("Failed to write TSIG key file")?;
        keyfile.flush().context("Failed to flush TSIG key file")?;

        debug!("Using TSIG authentication with key: {}", key_name);
        Ok(Some(keyfile))
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

        // TSIG key goes into a 0600 temp file passed via -k, never into argv
        // (B-7). The guard keeps the file alive until nsupdate has completed.
        let keyfile = self.create_tsig_key_file()?;

        let mut cmd = tokio::process::Command::new("nsupdate");
        cmd.args(build_nsupdate_args(
            self.use_tcp,
            keyfile.as_ref().map(tempfile::NamedTempFile::path),
        ));

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

        // nsupdate has exited — remove the key file immediately rather than
        // waiting for scope end.
        drop(keyfile);

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

        // Defense-in-depth: reject control characters before assembling the
        // newline-delimited nsupdate command script.
        reject_injection_chars("zone", zone)?;
        reject_injection_chars("name", name)?;
        reject_injection_chars("value", value)?;

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

        // Defense-in-depth: reject control characters before assembling the
        // newline-delimited nsupdate command script.
        reject_injection_chars("zone", zone)?;
        reject_injection_chars("name", name)?;
        reject_injection_chars("value", value)?;

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

        // Defense-in-depth: reject control characters before assembling the
        // newline-delimited nsupdate command script.
        reject_injection_chars("zone", zone)?;
        reject_injection_chars("name", name)?;
        reject_injection_chars("old_value", old_value)?;
        reject_injection_chars("new_value", new_value)?;

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

/// Build the nsupdate argument vector.
///
/// The TSIG key is referenced by file path (`-k <keyfile>`), never embedded in
/// the arguments — argv is world-readable via `/proc/<pid>/cmdline` (B-7).
pub(crate) fn build_nsupdate_args(use_tcp: bool, keyfile: Option<&Path>) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    if use_tcp {
        args.push("-v".into());
    }
    if let Some(path) = keyfile {
        args.push("-k".into());
        args.push(path.as_os_str().to_owned());
    }
    args
}

/// Render a BIND TSIG key file for `nsupdate -k`.
///
/// Produces:
/// ```text
/// key "<name>" {
///     algorithm <hmac-...>;
///     secret "<base64>";
/// };
/// ```
///
/// All three fields are validated before interpolation since they land in a
/// quoted BIND configuration literal:
/// - `key_name` — safe identifier set (`[A-Za-z0-9._-]`, max 253 chars)
/// - `algorithm` — normalized to lowercase, `hmac-` prefix added if missing,
///   then checked against the known HMAC algorithm list
/// - `secret` — base64 character set only
///
/// # Errors
/// Returns an error when any field is empty or fails validation.
pub(crate) fn build_tsig_key_file_content(
    key_name: &str,
    algorithm: &str,
    secret: &str,
) -> Result<String> {
    if key_name.is_empty() || key_name.len() > MAX_TSIG_KEY_NAME_LEN {
        return Err(anyhow::anyhow!(
            "TSIG key name must be 1-{} characters",
            MAX_TSIG_KEY_NAME_LEN
        ));
    }
    if let Some(bad) = key_name
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')))
    {
        return Err(anyhow::anyhow!(
            "TSIG key name contains invalid character: {:?}",
            bad
        ));
    }

    // Accept "SHA256", "hmac-sha256", "HMAC-SHA256", etc.
    let mut normalized_algorithm = algorithm.to_ascii_lowercase();
    if !normalized_algorithm.starts_with("hmac-") {
        normalized_algorithm = format!("hmac-{}", normalized_algorithm);
    }
    if !ALLOWED_TSIG_ALGORITHMS.contains(&normalized_algorithm.as_str()) {
        return Err(anyhow::anyhow!(
            "Unsupported TSIG algorithm: {} (allowed: {})",
            algorithm,
            ALLOWED_TSIG_ALGORITHMS.join(", ")
        ));
    }

    if secret.is_empty() {
        return Err(anyhow::anyhow!("TSIG secret cannot be empty"));
    }
    if let Some(bad) = secret
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=')))
    {
        return Err(anyhow::anyhow!(
            "TSIG secret contains non-base64 character: {:?}",
            bad
        ));
    }

    Ok(format!(
        "key \"{key_name}\" {{\n    algorithm {normalized_algorithm};\n    secret \"{secret}\";\n}};\n"
    ))
}

/// Reject a DNS update field containing control characters.
///
/// nsupdate reads its command script as newline-delimited lines from stdin, so a
/// `\n`, `\r`, or NUL embedded in a zone, name, or value would be interpreted as a
/// command separator and allow injection of arbitrary update commands (B-2). This
/// is a defense-in-depth check at the sink; the HTTP handlers in `records.rs` also
/// reject these characters and return HTTP 400.
///
/// # Errors
/// Returns an error if `value` contains any control character.
pub(crate) fn reject_injection_chars(field: &str, value: &str) -> Result<()> {
    if let Some(bad) = value.chars().find(|c| c.is_control()) {
        return Err(anyhow::anyhow!(
            "{} contains illegal control character {:?}",
            field,
            bad
        ));
    }

    Ok(())
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
