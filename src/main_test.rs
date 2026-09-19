// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for main binary helpers

use super::{probe_zone_dir, ready_check_label};

/// The unauthenticated /ready response must expose only a coarse ok/error
/// status — never an internal path or backend error string.
#[test]
fn test_ready_check_label_is_non_sensitive() {
    assert_eq!(ready_check_label("zone_dir", true), "zone_dir: ok");
    assert_eq!(ready_check_label("zone_dir", false), "zone_dir: error");
    assert_eq!(ready_check_label("rndc", true), "rndc: ok");
    assert_eq!(ready_check_label("rndc", false), "rndc: error");

    // Guard against regressions that re-introduce leakage: the label must never
    // contain a path separator or error punctuation regardless of outcome.
    for ok in [true, false] {
        let label = ready_check_label("zone_dir", ok);
        assert!(!label.contains('/'), "label leaked a path: {label}");
    }
}

/// `probe_zone_dir` must refuse any path that is not a normalized absolute
/// path, *before* it reaches the `tokio::fs::metadata` sink. The guard is a
/// straight-line early return so that CodeQL's `rust/path-injection` barrier
/// guard recognizes it (see the doc comment on `probe_zone_dir`).
#[tokio::test]
async fn test_probe_zone_dir_rejects_non_normalized_paths() {
    // Relative paths are never acceptable — zone_dir is canonicalized at startup.
    assert!(!probe_zone_dir("relative/zones").await);
    assert!(!probe_zone_dir("").await);

    // Parent-directory references are rejected even when the prefix is absolute.
    assert!(!probe_zone_dir("/var/lib/bind/../../etc").await);
    assert!(!probe_zone_dir("/var/lib/bind/..").await);

    // Current-directory components are rejected too.
    assert!(!probe_zone_dir("/var/lib/./bind").await);
}

/// A normalized absolute path that does not exist must probe as not-ready
/// rather than panicking or being reported ready.
#[tokio::test]
async fn test_probe_zone_dir_rejects_missing_directory() {
    assert!(!probe_zone_dir("/nonexistent-bindcar-zone-dir-for-tests").await);
}

/// A normalized absolute path that resolves to a *file* is not a usable zone
/// directory and must probe as not-ready.
#[tokio::test]
async fn test_probe_zone_dir_rejects_regular_file() {
    let file = tempfile::NamedTempFile::new().expect("create temp file");
    let path = file.path().to_str().expect("temp path is valid utf-8");

    assert!(!probe_zone_dir(path).await);
}

/// The happy path: an existing, normalized, absolute directory probes ready.
#[tokio::test]
async fn test_probe_zone_dir_accepts_existing_directory() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().to_str().expect("temp path is valid utf-8");

    assert!(probe_zone_dir(path).await);
}

/// A binary built without the `tls` feature must refuse to start when TLS was
/// configured, rather than silently serving plaintext on a deployment whose
/// operator believes TLS is on. This is the same fail-closed rule the runtime
/// TLS config follows, extended to compile-time capability.
#[test]
fn test_check_tls_support_rejects_configured_tls_in_a_build_without_it() {
    let err = super::check_tls_support(true, false)
        .expect_err("configured TLS on a non-TLS build must be refused");
    let msg = err.to_string();

    assert!(
        msg.contains("tls"),
        "the error must name the missing feature: {msg}"
    );
    assert!(
        msg.contains("--features tls") || msg.contains("feature"),
        "the error must say how to fix it: {msg}"
    );
}

/// Every other combination starts normally.
#[test]
fn test_check_tls_support_allows_the_supported_combinations() {
    // TLS wanted and compiled in
    assert!(super::check_tls_support(true, true).is_ok());
    // TLS not wanted, compiled in — the common case for a plaintext deployment
    assert!(super::check_tls_support(false, true).is_ok());
    // TLS not wanted, not compiled in — a minimal build serving plaintext
    assert!(super::check_tls_support(false, false).is_ok());
}

/// The constant must track the actual build configuration, so the check above
/// is wired to reality rather than a hardcoded value.
#[test]
fn test_tls_supported_reflects_the_build() {
    assert_eq!(super::TLS_SUPPORTED, cfg!(feature = "tls"));
}
