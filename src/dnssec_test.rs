// Copyright (c) 2026 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for the dnssec module (ADR-0001, roadmap 07).

use super::dnssec::*;

/// Real `rndc dnssec -status` output captured from BIND 9.18.50 for a zone
/// signed with a CSK policy (ADR-0001 live verification, 2026-09-27).
const STATUS_SIGNED_CSK: &str = "dnssec-policy: test-policy
current time:  Sun Sep 27 22:07:11 2026

key: 33306 (ECDSAP256SHA256), CSK
  published:      yes - since Sun Sep 27 22:07:06 2026
  key signing:    yes - since Sun Sep 27 22:07:06 2026
  zone signing:   yes - since Sun Sep 27 22:07:06 2026

  No rollover scheduled
  - goal:           omnipresent
  - dnskey:         rumoured
  - ds:             hidden
  - zone rrsig:     rumoured
  - key rrsig:      rumoured
";

/// Real output captured after switching the same zone to the built-in
/// "insecure" policy (graceful unsigning) once the key had been removed.
const STATUS_INSECURE: &str = "dnssec-policy: insecure
current time:  Sun Sep 27 22:33:23 2026

key: 33306 (ECDSAP256SHA256), CSK
  published:      no
  key signing:    no
  zone signing:   no

  Key has been removed from the zone
  - goal:           hidden
  - dnskey:         unretentive
  - ds:             hidden
  - zone rrsig:     unretentive
  - key rrsig:      unretentive
";

/// Real output for a zone with no dnssec-policy at all.
const STATUS_UNSIGNED: &str = "Zone does not have dnssec-policy";

/// Synthetic split-key (KSK+ZSK) status with a scheduled rollover, in the
/// same line format BIND 9.18 emits.
const STATUS_KSK_ZSK_ROLLOVER: &str = "dnssec-policy: split
current time:  Sun Sep 27 22:07:11 2026

key: 11111 (ECDSAP256SHA256), KSK
  published:      yes - since Sun Sep 27 22:07:06 2026
  key signing:    yes - since Sun Sep 27 22:07:06 2026

  No rollover scheduled
  - goal:           omnipresent
  - dnskey:         omnipresent
  - ds:             omnipresent
  - key rrsig:      omnipresent

key: 22222 (ECDSAP256SHA256), ZSK
  published:      yes - since Sun Sep 27 22:07:06 2026
  zone signing:   yes - since Sun Sep 27 22:07:06 2026

  Next rollover scheduled on Thu Dec 24 15:35:20 2026
  - goal:           omnipresent
  - dnskey:         omnipresent
  - zone rrsig:     omnipresent
";

// ---------------------------------------------------------------------------
// parse_dnssec_status
// ---------------------------------------------------------------------------

#[test]
fn test_parse_status_signed_csk() {
    let status = parse_dnssec_status(STATUS_SIGNED_CSK);
    assert_eq!(status.policy.as_deref(), Some("test-policy"));
    assert!(status.signed);
    assert_eq!(status.keys.len(), 1);

    let key = &status.keys[0];
    assert_eq!(key.tag, 33306);
    assert_eq!(key.algorithm, "ECDSAP256SHA256");
    assert_eq!(key.role, "CSK");
    assert!(key.published);
    assert!(key.key_signing);
    assert!(key.zone_signing);
    assert_eq!(key.published_since.as_deref(), Some("2026-09-27T22:07:06"));
    assert_eq!(
        key.key_signing_since.as_deref(),
        Some("2026-09-27T22:07:06")
    );
    assert_eq!(
        key.zone_signing_since.as_deref(),
        Some("2026-09-27T22:07:06")
    );
    assert_eq!(key.next_rollover, None);
    assert!(!key.removed);
    assert_eq!(key.goal.as_deref(), Some("omnipresent"));
    assert_eq!(key.dnskey_state.as_deref(), Some("rumoured"));
    assert_eq!(key.ds_state.as_deref(), Some("hidden"));
    assert_eq!(key.zone_rrsig_state.as_deref(), Some("rumoured"));
    assert_eq!(key.key_rrsig_state.as_deref(), Some("rumoured"));
}

#[test]
fn test_parse_status_insecure_policy_not_signed() {
    let status = parse_dnssec_status(STATUS_INSECURE);
    assert_eq!(status.policy.as_deref(), Some("insecure"));
    // The key has been removed from the zone: nothing serves DNSKEY anymore.
    assert!(!status.signed);

    let key = &status.keys[0];
    assert!(!key.published);
    assert!(!key.key_signing);
    assert!(!key.zone_signing);
    assert_eq!(key.published_since, None);
    assert!(key.removed);
    assert_eq!(key.goal.as_deref(), Some("hidden"));
    assert_eq!(key.dnskey_state.as_deref(), Some("unretentive"));
}

#[test]
fn test_parse_status_unsigned_zone() {
    let status = parse_dnssec_status(STATUS_UNSIGNED);
    assert_eq!(status.policy, None);
    assert!(!status.signed);
    assert!(status.keys.is_empty());
}

#[test]
fn test_parse_status_ksk_zsk_with_rollover() {
    let status = parse_dnssec_status(STATUS_KSK_ZSK_ROLLOVER);
    assert_eq!(status.policy.as_deref(), Some("split"));
    assert!(status.signed);
    assert_eq!(status.keys.len(), 2);

    let ksk = &status.keys[0];
    assert_eq!(ksk.tag, 11111);
    assert_eq!(ksk.role, "KSK");
    assert_eq!(ksk.next_rollover, None);
    assert_eq!(ksk.ds_state.as_deref(), Some("omnipresent"));
    // The KSK block has no "zone signing" or "zone rrsig" lines.
    assert!(!ksk.zone_signing);
    assert_eq!(ksk.zone_rrsig_state, None);

    let zsk = &status.keys[1];
    assert_eq!(zsk.tag, 22222);
    assert_eq!(zsk.role, "ZSK");
    assert_eq!(zsk.next_rollover.as_deref(), Some("2026-12-24T15:35:20"));
    assert!(!zsk.key_signing);
}

#[test]
fn test_parse_status_empty_input() {
    let status = parse_dnssec_status("");
    assert_eq!(status.policy, None);
    assert!(!status.signed);
    assert!(status.keys.is_empty());
}

#[test]
fn test_parse_status_unparseable_timestamp_kept_raw() {
    let input = "dnssec-policy: p\n\nkey: 1 (ECDSAP256SHA256), CSK\n  published:      yes - since not a date\n";
    let status = parse_dnssec_status(input);
    let key = &status.keys[0];
    assert!(key.published);
    // A timestamp BIND emits in a format we do not recognize is passed
    // through raw rather than dropped.
    assert_eq!(key.published_since.as_deref(), Some("not a date"));
}

// ---------------------------------------------------------------------------
// DNSKEY parsing / key tag / DS computation (RFC 4034 / RFC 4509 vectors)
// ---------------------------------------------------------------------------

/// RFC 4509 section 2.3 example DNSKEY (key id 60485).
const RFC4509_DNSKEY_B64: &str = "AQOeiiR0GOMYkDshWoSKz9XzfwJr1AYtsmx3TGkJaNXVbfi/2pHm822aJ5iI9BMzNXxeYCmZDRD99WYwYqUSdjMmmAphXdvxegXd/M5+X7OrzKBaMbCVdFLUUh6DhweJBjEVv5f2wwjM9XzcnOf+EPbtG9DMBmADjFDc2w/rljwvFw==";

fn rfc4509_key() -> DnskeyRecord {
    let mut rr = String::new();
    rr.push_str("dskey.example.com. 86400 IN DNSKEY 256 3 5 ");
    rr.push_str(RFC4509_DNSKEY_B64);
    parse_dnskey_rr(&rr).expect("RFC 4509 example DNSKEY must parse")
}

#[test]
fn test_parse_dnskey_rr_fields() {
    let key = rfc4509_key();
    assert_eq!(key.owner, "dskey.example.com.");
    assert_eq!(key.flags, 256);
    assert_eq!(key.protocol, 3);
    assert_eq!(key.algorithm, 5);
    assert!(!key.public_key.is_empty());
    assert!(!key.is_ksk());
}

#[test]
fn test_key_tag_matches_rfc4509_example() {
    assert_eq!(rfc4509_key().key_tag(), 60485);
}

#[test]
fn test_ds_record_matches_rfc4509_example() {
    let ds = rfc4509_key().ds_record().expect("DS computation");
    assert_eq!(ds.key_tag, 60485);
    assert_eq!(ds.algorithm, 5);
    assert_eq!(ds.digest_type, DS_DIGEST_TYPE_SHA256);
    assert_eq!(
        ds.digest,
        "D4B7D520E7BB5F0F67674A0CCEB1E3E0614B93C4F9E99B8383F6A1E4469DA50A"
    );
    assert_eq!(
        ds.rdata(),
        "60485 5 2 D4B7D520E7BB5F0F67674A0CCEB1E3E0614B93C4F9E99B8383F6A1E4469DA50A"
    );
}

#[test]
fn test_ds_owner_name_is_canonicalized_lowercase() {
    // RFC 4034: the owner name is hashed in canonical (lowercase) wire form,
    // so case differences must not change the digest.
    let rr_upper = format!("DSKEY.EXAMPLE.COM. 86400 IN DNSKEY 256 3 5 {RFC4509_DNSKEY_B64}");
    let key = parse_dnskey_rr(&rr_upper).expect("uppercase owner must parse");
    assert_eq!(
        key.ds_record().expect("DS computation").digest,
        "D4B7D520E7BB5F0F67674A0CCEB1E3E0614B93C4F9E99B8383F6A1E4469DA50A"
    );
}

#[test]
fn test_parse_dnskey_rr_with_split_base64() {
    // Key files and zone files may split the base64 blob across whitespace.
    let mut rr = String::from("dskey.example.com. 86400 IN DNSKEY 256 3 5 ");
    let (a, b) = RFC4509_DNSKEY_B64.split_at(40);
    rr.push_str(a);
    rr.push(' ');
    rr.push_str(b);
    let key = parse_dnskey_rr(&rr).expect("split base64 must parse");
    assert_eq!(key.key_tag(), 60485);
}

#[test]
fn test_parse_dnskey_rr_ksk_flag() {
    let rr = format!("example.com. 300 IN DNSKEY 257 3 13 {RFC4509_DNSKEY_B64}");
    let key = parse_dnskey_rr(&rr).expect("KSK flags must parse");
    assert_eq!(key.flags, 257);
    assert!(key.is_ksk());
}

#[test]
fn test_parse_dnskey_rr_rejects_garbage() {
    assert!(parse_dnskey_rr("").is_err());
    assert!(parse_dnskey_rr("example.com. 300 IN A 192.0.2.1").is_err());
    assert!(parse_dnskey_rr("example.com. 300 IN DNSKEY 256 3").is_err());
    assert!(parse_dnskey_rr("example.com. 300 IN DNSKEY 256 3 5 !!!notbase64!!!").is_err());
}

#[test]
fn test_parse_key_file_skips_comments() {
    // Shape of a BIND public key file (K<zone>.+<alg>+<tag>.key).
    let contents = format!(
        "; This is a key-signing key, keyid 60485, for dskey.example.com.\n\
         ; Created: 20260927220706 (Sun Sep 27 22:07:06 2026)\n\
         ; Publish: 20260927220706 (Sun Sep 27 22:07:06 2026)\n\
         ; Activate: 20260927220706 (Sun Sep 27 22:07:06 2026)\n\
         dskey.example.com. 86400 IN DNSKEY 256 3 5 {RFC4509_DNSKEY_B64}\n"
    );
    let key = parse_key_file(&contents).expect("key file must parse");
    assert_eq!(key.key_tag(), 60485);
}

#[test]
fn test_parse_key_file_without_dnskey_line_fails() {
    assert!(parse_key_file("; only a comment\n").is_err());
}

#[test]
fn test_owner_name_too_long_rejected() {
    // A label over 63 octets is invalid and must not silently truncate.
    let label = "a".repeat(64);
    let rr = format!("{label}.example.com. 300 IN DNSKEY 256 3 5 {RFC4509_DNSKEY_B64}");
    let key = parse_dnskey_rr(&rr).expect("parse keeps the textual owner");
    assert!(key.ds_record().is_err());
}

// ---------------------------------------------------------------------------
// KSK selection for DS computation
// ---------------------------------------------------------------------------

#[test]
fn test_ksk_tags_selects_ksk_and_csk_roles() {
    let status = parse_dnssec_status(STATUS_KSK_ZSK_ROLLOVER);
    let tags = status.ksk_tags();
    assert!(tags.contains(&11111), "KSK must be selected");
    assert!(!tags.contains(&22222), "ZSK must not be selected");

    let csk = parse_dnssec_status(STATUS_SIGNED_CSK);
    assert!(csk.ksk_tags().contains(&33306), "CSK must be selected");
}

#[test]
fn test_ksk_tags_empty_for_unsigned_zone() {
    let status = parse_dnssec_status(STATUS_UNSIGNED);
    assert!(status.ksk_tags().is_empty());
}

// ---------------------------------------------------------------------------
// Algorithm agility (roadmap 09 phase 4): bindcar must pass through DNSSEC
// algorithms it has never seen. When BIND one day reports a post-quantum
// algorithm, nothing here may reject or mangle it.
// ---------------------------------------------------------------------------

/// A DNSSEC algorithm number no IANA registry entry uses today, standing in
/// for a future post-quantum assignment.
const FUTURE_ALGORITHM_NUMBER: u8 = 248;

/// A mnemonic BIND does not emit today, standing in for a future
/// post-quantum signing algorithm.
const FUTURE_ALGORITHM_MNEMONIC: &str = "ML-DSA-44";

#[test]
fn test_parse_dnskey_rr_unknown_algorithm_number_passes_through() {
    let rr = format!(
        "dskey.example.com. 86400 IN DNSKEY 256 3 {FUTURE_ALGORITHM_NUMBER} {RFC4509_DNSKEY_B64}"
    );
    let key = parse_dnskey_rr(&rr).expect("unknown algorithm number must parse");
    assert_eq!(key.algorithm, FUTURE_ALGORITHM_NUMBER);
}

#[test]
fn test_ds_computation_is_algorithm_agnostic() {
    // The SHA-256 digest covers the DNSKEY RDATA including the algorithm
    // byte, so an unknown algorithm must compute (not error) and must yield
    // a different digest than the same key material under algorithm 5.
    let rr = format!(
        "dskey.example.com. 86400 IN DNSKEY 256 3 {FUTURE_ALGORITHM_NUMBER} {RFC4509_DNSKEY_B64}"
    );
    let key = parse_dnskey_rr(&rr).expect("parse");
    let ds = key.ds_record().expect("DS must compute for any algorithm");
    assert_eq!(ds.algorithm, FUTURE_ALGORITHM_NUMBER);
    assert_eq!(ds.digest_type, DS_DIGEST_TYPE_SHA256);
    assert_eq!(ds.digest.len(), 64, "SHA-256 digest is 32 bytes of hex");
    assert_ne!(
        ds.digest, "D4B7D520E7BB5F0F67674A0CCEB1E3E0614B93C4F9E99B8383F6A1E4469DA50A",
        "algorithm byte must participate in the digest"
    );
}

#[test]
fn test_parse_status_unknown_algorithm_mnemonic_round_trips() {
    let status_output = format!(
        "dnssec-policy: pqc-policy\ncurrent time:  Sun Sep 27 22:07:11 2026\n\n\
         key: 12345 ({FUTURE_ALGORITHM_MNEMONIC}), CSK\n\
         \x20 published:      yes - since Sun Sep 27 22:07:06 2026\n\
         \x20 key signing:    yes - since Sun Sep 27 22:07:06 2026\n\
         \x20 zone signing:   yes - since Sun Sep 27 22:07:06 2026\n\n\
         \x20 No rollover scheduled\n\
         \x20 - goal:           omnipresent\n\
         \x20 - dnskey:         omnipresent\n\
         \x20 - ds:             hidden\n\
         \x20 - zone rrsig:     omnipresent\n\
         \x20 - key rrsig:      omnipresent\n"
    );
    let status = parse_dnssec_status(&status_output);
    assert_eq!(status.keys.len(), 1);
    assert_eq!(status.keys[0].tag, 12345);
    assert_eq!(status.keys[0].algorithm, FUTURE_ALGORITHM_MNEMONIC);
    assert!(status.signed);
}

#[test]
fn test_zone_status_dnssec_block_serializes_unknown_algorithm() {
    // The typed `dnssec` block on the zone-status response serializes the
    // mnemonic as an opaque string: no allowlist, no normalization.
    let status_output = format!(
        "dnssec-policy: pqc-policy\ncurrent time:  Sun Sep 27 22:07:11 2026\n\n\
         key: 12345 ({FUTURE_ALGORITHM_MNEMONIC}), CSK\n\
         \x20 published:      yes - since Sun Sep 27 22:07:06 2026\n"
    );
    let status = parse_dnssec_status(&status_output);
    let json = serde_json::to_value(&status).expect("serialize");
    assert_eq!(
        json["keys"][0]["algorithm"],
        serde_json::json!(FUTURE_ALGORITHM_MNEMONIC)
    );
}
