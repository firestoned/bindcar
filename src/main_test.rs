// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for main binary helpers

use super::ready_check_label;

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
