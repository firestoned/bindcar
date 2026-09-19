// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! TLS transport for the HTTP API.
//!
//! bindcar's REST API carries a privileged bearer credential on every request —
//! a Kubernetes ServiceAccount token under TokenReview auth, or the shared
//! `BIND_API_TOKEN`. Authentication proves *who* the caller is; it does nothing
//! for confidentiality. Without TLS that credential crosses the pod network in
//! the clear, readable by anything able to observe the link.
//!
//! This module resolves the TLS configuration and builds the [`rustls`] server
//! config. TLS is **opt-in**: with no certificate configured bindcar serves
//! plaintext exactly as before, so existing deployments are unaffected.
//!
//! # Configuration
//!
//! | CLI flag | Environment variable | Meaning |
//! |---|---|---|
//! | `--tls-cert` | `BIND_TLS_CERT` | PEM certificate chain, leaf first |
//! | `--tls-key` | `BIND_TLS_KEY` | PEM private key (PKCS#8, PKCS#1 or SEC1) |
//! | `--tls-client-ca` | `BIND_TLS_CLIENT_CA` | PEM CA bundle; presence turns on mTLS |
//!
//! # Fail-closed rules
//!
//! Misconfiguration is a startup error, never a silent downgrade:
//!
//! - Exactly one of cert/key set → [`TlsError::IncompleteKeyPair`]. Falling back
//!   to plaintext here would serve the API in the clear on a deployment whose
//!   operator believes TLS is on.
//! - A client CA with no key pair → [`TlsError::ClientCaWithoutTls`], rather than
//!   accepting the flag and ignoring it.
//! - Unreadable or unparseable PEM → an error before the listener binds.

#[cfg(feature = "tls")]
use std::sync::{Arc, RwLock};

#[cfg(feature = "tls")]
use tracing::{debug, info, warn};

#[cfg(feature = "tls")]
use rustls::server::WebPkiClientVerifier;
#[cfg(feature = "tls")]
use rustls::{RootCertStore, ServerConfig};
#[cfg(feature = "tls")]
use rustls_pki_types::pem::PemObject;
#[cfg(feature = "tls")]
use rustls_pki_types::{CertificateDer, PrivateKeyDer};

/// ALPN protocols advertised by the TLS listener, preferred order first.
#[cfg(feature = "tls")]
///
/// `h2` before `http/1.1` matches the HTTP versions the hyper auto-builder
/// serves, so a client negotiating either lands on a protocol bindcar speaks.
const ALPN_PROTOCOLS: [&[u8]; 2] = [b"h2", b"http/1.1"];

/// Errors raised while resolving or loading the TLS transport configuration.
///
/// Every variant is fatal at startup — bindcar refuses to bind rather than
/// downgrade to plaintext.
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    /// Exactly one of the certificate/key pair was supplied.
    #[error(
        "TLS is half-configured: {supplied} was set but {missing} was not; \
         set both to enable TLS, or neither to serve plaintext"
    )]
    IncompleteKeyPair {
        /// The option that was supplied.
        supplied: &'static str,
        /// The option that was missing.
        missing: &'static str,
    },

    /// A client CA was supplied without a server certificate and key.
    #[error(
        "a TLS client CA was configured but no server certificate/key was; \
         mutual TLS requires a TLS listener"
    )]
    ClientCaWithoutTls,

    /// The certificate file could not be read or contained no certificates.
    #[error("failed to read TLS certificate from {path}: {reason}")]
    CertRead {
        /// The configured certificate path.
        path: String,
        /// The underlying error, rendered.
        reason: String,
    },

    /// The private key file could not be read.
    #[error("failed to read TLS private key from {path}: {reason}")]
    KeyRead {
        /// The configured key path.
        path: String,
        /// The underlying error, rendered.
        reason: String,
    },

    /// The private key did not match the certificate, or was otherwise rejected.
    #[error("TLS private key from {path} was rejected: {message}")]
    KeyParse {
        /// The configured key path.
        path: String,
        /// Why rustls rejected the key.
        message: String,
    },

    /// The client CA bundle could not be read.
    #[error("failed to read TLS client CA bundle from {path}: {reason}")]
    ClientCaRead {
        /// The configured client CA path.
        path: String,
        /// The underlying error, rendered.
        reason: String,
    },

    /// The client CA bundle parsed but produced no usable trust anchors.
    #[error("TLS client CA bundle from {path} yielded no usable certificates: {message}")]
    ClientCaInvalid {
        /// The configured client CA path.
        path: String,
        /// Why the bundle was rejected.
        message: String,
    },
}

/// A resolved, syntactically valid TLS configuration.
///
/// Holding one of these means the operator asked for TLS and the combination of
/// options is coherent. It does **not** mean the files exist or parse — that is
/// [`build_server_config`]'s job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsSettings {
    /// Path to the PEM certificate chain, leaf first.
    pub cert_path: String,
    /// Path to the PEM private key.
    pub key_path: String,
    /// Path to a PEM CA bundle used to verify client certificates, when mTLS is on.
    pub client_ca_path: Option<String>,
}

impl TlsSettings {
    /// Whether this configuration demands a client certificate (mutual TLS).
    #[must_use]
    pub fn requires_client_auth(&self) -> bool {
        self.client_ca_path.is_some()
    }
}

/// Normalize an option, treating an empty string as absent.
///
/// An unset environment variable that expands to `""` in a shell wrapper or a
/// Kubernetes manifest must not half-configure the listener.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

/// Resolve TLS settings from the certificate, key and client-CA options.
///
/// # Arguments
/// * `cert_path` - PEM certificate chain path, if configured
/// * `key_path` - PEM private key path, if configured
/// * `client_ca_path` - PEM client CA bundle path, if configured
///
/// # Returns
/// `Ok(None)` when no TLS material is configured — bindcar serves plaintext.
/// `Ok(Some(settings))` when the combination is coherent.
///
/// # Errors
/// Returns [`TlsError::IncompleteKeyPair`] when exactly one of the certificate
/// and key is set, and [`TlsError::ClientCaWithoutTls`] when a client CA is set
/// without a server key pair. Both fail closed rather than serving plaintext.
pub fn resolve_tls_settings(
    cert_path: Option<String>,
    key_path: Option<String>,
    client_ca_path: Option<String>,
) -> Result<Option<TlsSettings>, TlsError> {
    let cert_path = non_empty(cert_path);
    let key_path = non_empty(key_path);
    let client_ca_path = non_empty(client_ca_path);

    let (cert_path, key_path) = match (cert_path, key_path) {
        (Some(cert), Some(key)) => (cert, key),
        (Some(_), None) => {
            return Err(TlsError::IncompleteKeyPair {
                supplied: "--tls-cert / BIND_TLS_CERT",
                missing: "--tls-key / BIND_TLS_KEY",
            })
        }
        (None, Some(_)) => {
            return Err(TlsError::IncompleteKeyPair {
                supplied: "--tls-key / BIND_TLS_KEY",
                missing: "--tls-cert / BIND_TLS_CERT",
            })
        }
        (None, None) if client_ca_path.is_some() => return Err(TlsError::ClientCaWithoutTls),
        (None, None) => return Ok(None),
    };

    Ok(Some(TlsSettings {
        cert_path,
        key_path,
        client_ca_path,
    }))
}

/// The URL scheme bindcar serves under, for log lines and the advertised docs URL.
///
/// # Arguments
/// * `settings` - the resolved TLS settings, or `None` for plaintext
#[must_use]
pub fn scheme_for(settings: Option<&TlsSettings>) -> &'static str {
    if settings.is_some() {
        return "https";
    }
    "http"
}

#[cfg(feature = "tls")]
/// Load the client CA bundle and build a verifier demanding a valid client certificate.
///
/// # Errors
/// Returns [`TlsError::ClientCaRead`] if the bundle cannot be read and
/// [`TlsError::ClientCaInvalid`] if it yields no usable trust anchors.
pub(crate) fn build_client_verifier(
    path: &str,
) -> Result<Arc<dyn rustls::server::danger::ClientCertVerifier>, TlsError> {
    let mut roots = RootCertStore::empty();

    for cert in CertificateDer::pem_file_iter(path).map_err(|e| TlsError::ClientCaRead {
        path: path.to_string(),
        reason: e.to_string(),
    })? {
        let cert = cert.map_err(|e| TlsError::ClientCaRead {
            path: path.to_string(),
            reason: e.to_string(),
        })?;
        roots.add(cert).map_err(|e| TlsError::ClientCaInvalid {
            path: path.to_string(),
            message: e.to_string(),
        })?;
    }

    if roots.is_empty() {
        return Err(TlsError::ClientCaInvalid {
            path: path.to_string(),
            message: "bundle contained no certificates".to_string(),
        });
    }

    WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .map_err(|e| TlsError::ClientCaInvalid {
            path: path.to_string(),
            message: e.to_string(),
        })
}

#[cfg(feature = "tls")]
/// Build the [`rustls::ServerConfig`] for the API listener.
///
/// Loads the certificate chain and private key from disk, and — when a client
/// CA is configured — a verifier that makes a valid client certificate
/// mandatory.
///
/// # Arguments
/// * `settings` - resolved TLS settings from [`resolve_tls_settings`]
///
/// # Errors
/// Returns [`TlsError::CertRead`], [`TlsError::KeyRead`], [`TlsError::KeyParse`],
/// [`TlsError::ClientCaRead`] or [`TlsError::ClientCaInvalid`] depending on which
/// piece of material failed to load. All are fatal at startup.
pub fn build_server_config(settings: &TlsSettings) -> Result<ServerConfig, TlsError> {
    let certs = CertificateDer::pem_file_iter(&settings.cert_path)
        .map_err(|e| TlsError::CertRead {
            path: settings.cert_path.clone(),
            reason: e.to_string(),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TlsError::CertRead {
            path: settings.cert_path.clone(),
            reason: e.to_string(),
        })?;

    let key = PrivateKeyDer::from_pem_file(&settings.key_path).map_err(|e| TlsError::KeyRead {
        path: settings.key_path.clone(),
        reason: e.to_string(),
    })?;

    let builder = ServerConfig::builder();

    let mut config = match &settings.client_ca_path {
        Some(ca_path) => builder
            .with_client_cert_verifier(build_client_verifier(ca_path)?)
            .with_single_cert(certs, key),
        None => builder.with_no_client_auth().with_single_cert(certs, key),
    }
    .map_err(|e| TlsError::KeyParse {
        path: settings.key_path.clone(),
        message: e.to_string(),
    })?;

    config.alpn_protocols = ALPN_PROTOCOLS.iter().map(|p| p.to_vec()).collect();

    Ok(config)
}

// ---------------------------------------------------------------------------
// Hot-reload (roadmap 06)
// ---------------------------------------------------------------------------

/// How often the certificate files are re-examined, in seconds.
///
/// Three file reads and three hashes a minute is negligible next to the cost of
/// a rotation that silently never takes effect.
pub const DEFAULT_RELOAD_INTERVAL_SECS: u64 = 60;

#[cfg(feature = "tls")]
/// Fingerprint the TLS material currently on disk.
///
/// Kubernetes `Secret` and projected volumes do not rewrite files in place —
/// they swap a `..data` symlink — so mtime and inode are both unreliable across
/// a renewal. Hashing the bytes is what actually detects the change.
///
/// The client CA is included so mTLS trust is part of the reloadable surface,
/// not just the server certificate.
///
/// # Arguments
/// * `settings` - the resolved TLS settings naming the files to hash
///
/// # Returns
/// A SHA-256 digest over the certificate, key and (when configured) client CA.
///
/// # Errors
/// Returns [`TlsError::CertRead`], [`TlsError::KeyRead`] or
/// [`TlsError::ClientCaRead`] if a file cannot be read. Callers polling for
/// changes should treat an error as "no change" and retry: a file is routinely
/// unreadable for a moment during a renewal.
pub fn fingerprint(settings: &TlsSettings) -> Result<[u8; 32], TlsError> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();

    let cert = std::fs::read(&settings.cert_path).map_err(|e| TlsError::CertRead {
        path: settings.cert_path.clone(),
        reason: e.to_string(),
    })?;
    hasher.update(&cert);

    let key = std::fs::read(&settings.key_path).map_err(|e| TlsError::KeyRead {
        path: settings.key_path.clone(),
        reason: e.to_string(),
    })?;
    hasher.update(&key);

    if let Some(ca_path) = &settings.client_ca_path {
        let ca = std::fs::read(ca_path).map_err(|e| TlsError::ClientCaRead {
            path: ca_path.clone(),
            reason: e.to_string(),
        })?;
        hasher.update(&ca);
    }

    Ok(hasher.finalize().into())
}

#[cfg(feature = "tls")]
/// Holds the live [`rustls::ServerConfig`] and swaps it when the files change.
///
/// The accept loop reads [`TlsReloader::current`] once per connection, so a swap
/// affects new connections only — established ones finish under the certificate
/// they started with, which is what makes rotation non-disruptive.
///
/// # Failure behaviour
///
/// A reload that cannot be built is a **non-event**: the error is logged and the
/// existing configuration keeps serving. This is deliberate and differs from
/// startup, where bad TLS material is fatal. At startup, continuing would mean
/// silently serving plaintext; here there is already a working configuration in
/// memory, so retaining it is strictly safer than either failing or downgrading.
///
/// It also makes non-atomic renewals safe. A poll can easily observe a new
/// certificate alongside a not-yet-replaced key; that pair does not build, the
/// swap is skipped, and the next poll picks up the completed write.
#[derive(Debug)]
pub struct TlsReloader {
    settings: TlsSettings,
    config: RwLock<Arc<ServerConfig>>,
    last_fingerprint: RwLock<[u8; 32]>,
}

#[cfg(feature = "tls")]
impl TlsReloader {
    /// Build the initial configuration and capture its fingerprint.
    ///
    /// # Arguments
    /// * `settings` - resolved TLS settings
    ///
    /// # Errors
    /// Propagates any [`TlsError`] from [`build_server_config`]. Unlike a later
    /// reload, a failure here **is** fatal: there is no known-good config to
    /// fall back to.
    pub fn new(settings: TlsSettings) -> Result<Self, TlsError> {
        let config = Arc::new(build_server_config(&settings)?);
        // A fingerprint failure at startup cannot happen in practice — the files
        // were just read successfully — but treat it as "unknown" rather than
        // failing the server, so the first poll simply reloads.
        let fp = fingerprint(&settings).unwrap_or([0u8; 32]);

        Ok(Self {
            settings,
            config: RwLock::new(config),
            last_fingerprint: RwLock::new(fp),
        })
    }

    /// The configuration new connections should use.
    ///
    /// Cheap: clones an `Arc` while holding the read lock only for that clone,
    /// never across an `await`.
    #[must_use]
    pub fn current(&self) -> Arc<ServerConfig> {
        self.config
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Whether a reload interval enables reloading at all.
    ///
    /// Zero restores the startup-only behaviour of earlier releases.
    #[must_use]
    pub fn reloading_enabled(interval_secs: u64) -> bool {
        interval_secs > 0
    }

    /// Re-read the files and swap the live config if they changed and still build.
    ///
    /// # Returns
    /// `true` only when a new configuration was actually installed. An unchanged
    /// fingerprint, an unreadable file and a material that fails to build all
    /// return `false` and leave the current configuration serving.
    pub fn reload_if_changed(&self) -> bool {
        let current_fp = match fingerprint(&self.settings) {
            Ok(fp) => fp,
            Err(e) => {
                // Routine during a renewal; the next poll retries.
                debug!("TLS material not readable while polling for changes: {}", e);
                return false;
            }
        };

        {
            let last = self
                .last_fingerprint
                .read()
                .unwrap_or_else(|p| p.into_inner());
            if *last == current_fp {
                return false;
            }
        }

        let rebuilt = match build_server_config(&self.settings) {
            Ok(config) => config,
            Err(e) => {
                warn!(
                    "TLS material changed but the new configuration is not usable, \
continuing with the previous certificate: {}",
                    e
                );
                // Deliberately do NOT record the fingerprint: the write may still
                // be in progress, and the next poll must try again.
                return false;
            }
        };

        *self.config.write().unwrap_or_else(|p| p.into_inner()) = Arc::new(rebuilt);
        *self
            .last_fingerprint
            .write()
            .unwrap_or_else(|p| p.into_inner()) = current_fp;

        info!("reloaded TLS certificate material; new connections will use it");
        true
    }
}
