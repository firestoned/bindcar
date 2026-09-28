// Copyright (c) 2026 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! DNSSEC observability primitives (ADR-0001, roadmap 07).
//!
//! Two independent concerns live here, both pure and side-effect free:
//!
//! 1. Parsing `rndc dnssec -status <zone>` output into [`DnssecStatus`] —
//!    the source of the zone-status DNSSEC block. Per ADR-0001 this output
//!    (the supported operator interface, reflecting the *running* server) is
//!    parsed instead of BIND's internal key `.state` files.
//! 2. Parsing DNSKEY resource records from public key files
//!    (`K<zone>.+<alg>+<tag>.key`) and computing the DS record (RFC 4034
//!    appendix B key tag, RFC 4509 SHA-256 digest) in-process, so no
//!    `dnssec-dsfromkey` binary is needed in the image.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(feature = "server")]
use utoipa::ToSchema;

/// DS digest type 2 = SHA-256 (RFC 4509). The only digest type bindcar emits.
pub const DS_DIGEST_TYPE_SHA256: u8 = 2;

/// DNSKEY flags bit 15 (SEP, RFC 4034 §2.1.1): set on key-signing keys.
const DNSKEY_FLAG_SEP: u16 = 0x0001;

/// Maximum length of a single DNS label in wire format (RFC 1035 §2.3.4).
const MAX_LABEL_LEN: usize = 63;

/// Maximum length of a DNS name in wire format (RFC 1035 §2.3.4).
const MAX_NAME_LEN: usize = 255;

/// Timestamp format BIND uses in `rndc dnssec -status` output
/// (ctime-style, e.g. `Sun Sep 27 22:07:06 2026`).
const BIND_STATUS_TIME_FORMAT: &str = "%a %b %e %H:%M:%S %Y";

/// ISO 8601 (naive — the server's local clock, no zone claim) format used
/// for timestamps in API responses.
const ISO8601_NAIVE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// Errors from DNSKEY/DS parsing and computation.
#[derive(Debug, thiserror::Error)]
pub enum DnssecError {
    /// The input is not a DNSKEY resource record in textual form.
    #[error("not a DNSKEY resource record: {0}")]
    NotDnskey(String),

    /// A DNSKEY RR field failed to parse.
    #[error("invalid DNSKEY field {field}: {value}")]
    InvalidField {
        /// Which RDATA field was invalid.
        field: &'static str,
        /// The offending value.
        value: String,
    },

    /// The owner name cannot be encoded in DNS wire format.
    #[error("invalid owner name: {0}")]
    InvalidOwnerName(String),
}

/// Per-key signing state parsed from `rndc dnssec -status` output.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct DnssecKeyStatus {
    /// Key tag (RFC 4034 appendix B).
    pub tag: u16,
    /// Algorithm mnemonic as BIND reports it (e.g. `ECDSAP256SHA256`).
    pub algorithm: String,
    /// Key role: `KSK`, `ZSK` or `CSK`.
    pub role: String,
    /// Whether the DNSKEY is published in the zone.
    pub published: bool,
    /// When publication started (ISO 8601, server-local clock), if published.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_since: Option<String>,
    /// Whether the key signs the DNSKEY RRset (KSK duty).
    pub key_signing: bool,
    /// When key-signing started, if active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_signing_since: Option<String>,
    /// Whether the key signs zone data (ZSK duty).
    pub zone_signing: bool,
    /// When zone-signing started, if active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_signing_since: Option<String>,
    /// Next scheduled rollover event, if BIND reports one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_rollover: Option<String>,
    /// True once BIND reports "Key has been removed from the zone".
    pub removed: bool,
    /// Key goal state (`hidden`/`rumoured`/`omnipresent`/`unretentive`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    /// DNSKEY record state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dnskey_state: Option<String>,
    /// DS record state (as far as named knows, via checkds/parental agents).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ds_state: Option<String>,
    /// Zone RRSIG state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_rrsig_state: Option<String>,
    /// Key (DNSKEY RRset) RRSIG state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_rrsig_state: Option<String>,
}

/// Zone DNSSEC status parsed from `rndc dnssec -status` output.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct DnssecStatus {
    /// The `dnssec-policy` in effect on the running zone, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    /// Whether the zone currently serves DNSKEY records (derived: any key
    /// whose DNSKEY state is `rumoured` or `omnipresent`).
    pub signed: bool,
    /// Per-key state.
    pub keys: Vec<DnssecKeyStatus>,
}

/// Converts a BIND ctime-style timestamp to naive ISO 8601, passing the raw
/// string through when it does not parse (never drop information).
fn convert_bind_timestamp(raw: &str) -> String {
    match chrono::NaiveDateTime::parse_from_str(raw.trim(), BIND_STATUS_TIME_FORMAT) {
        Ok(dt) => dt.format(ISO8601_NAIVE_FORMAT).to_string(),
        Err(_) => raw.trim().to_string(),
    }
}

/// Parses a `published:` / `key signing:` / `zone signing:` value of the form
/// `yes - since <timestamp>` or `no`, returning (active, since).
fn parse_yes_since(value: &str) -> (bool, Option<String>) {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix("yes") {
        let since = rest
            .trim_start()
            .strip_prefix("- since")
            .map(convert_bind_timestamp);
        return (true, since);
    }
    (false, None)
}

/// Parses `rndc dnssec -status <zone>` output.
///
/// Unknown lines are ignored so that new BIND minor versions cannot break the
/// status endpoint; a zone without a policy ("Zone does not have
/// dnssec-policy") yields `policy: None, signed: false, keys: []`.
///
/// # Arguments
/// * `output` - Raw text returned by the rndc channel
pub fn parse_dnssec_status(output: &str) -> DnssecStatus {
    let mut status = DnssecStatus::default();
    let mut current: Option<DnssecKeyStatus> = None;

    for line in output.lines() {
        let trimmed = line.trim();

        if let Some(policy) = trimmed.strip_prefix("dnssec-policy:") {
            status.policy = Some(policy.trim().to_string());
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("key:") {
            // "key: 33306 (ECDSAP256SHA256), CSK"
            if let Some(prev) = current.take() {
                status.keys.push(prev);
            }
            let mut key = DnssecKeyStatus::default();
            let rest = rest.trim();
            let mut parts = rest.split_whitespace();
            key.tag = parts.next().and_then(|t| t.parse().ok()).unwrap_or(0);
            if let (Some(open), Some(close)) = (rest.find('('), rest.find(')')) {
                if open < close {
                    key.algorithm = rest[open + 1..close].to_string();
                }
                key.role = rest[close + 1..].trim_start_matches(',').trim().to_string();
            }
            current = Some(key);
            continue;
        }

        let Some(key) = current.as_mut() else {
            continue;
        };

        if let Some(v) = trimmed.strip_prefix("published:") {
            (key.published, key.published_since) = parse_yes_since(v);
        } else if let Some(v) = trimmed.strip_prefix("key signing:") {
            (key.key_signing, key.key_signing_since) = parse_yes_since(v);
        } else if let Some(v) = trimmed.strip_prefix("zone signing:") {
            (key.zone_signing, key.zone_signing_since) = parse_yes_since(v);
        } else if let Some(ts) = trimmed.strip_prefix("Next rollover scheduled on") {
            key.next_rollover = Some(convert_bind_timestamp(ts));
        } else if trimmed == "Key has been removed from the zone" {
            key.removed = true;
        } else if let Some(v) = trimmed.strip_prefix("- goal:") {
            key.goal = Some(v.trim().to_string());
        } else if let Some(v) = trimmed.strip_prefix("- dnskey:") {
            key.dnskey_state = Some(v.trim().to_string());
        } else if let Some(v) = trimmed.strip_prefix("- ds:") {
            key.ds_state = Some(v.trim().to_string());
        } else if let Some(v) = trimmed.strip_prefix("- zone rrsig:") {
            key.zone_rrsig_state = Some(v.trim().to_string());
        } else if let Some(v) = trimmed.strip_prefix("- key rrsig:") {
            key.key_rrsig_state = Some(v.trim().to_string());
        }
    }

    if let Some(prev) = current.take() {
        status.keys.push(prev);
    }

    status.signed = status.keys.iter().any(|k| {
        matches!(
            k.dnskey_state.as_deref(),
            Some("rumoured") | Some("omnipresent")
        )
    });

    status
}

impl DnssecStatus {
    /// Key tags of keys in a key-signing role (`KSK` or `CSK`) — the keys
    /// whose DNSKEY records DS records are derived from (ADR-0001).
    pub fn ksk_tags(&self) -> std::collections::HashSet<u16> {
        self.keys
            .iter()
            .filter(|k| k.role == "KSK" || k.role == "CSK")
            .map(|k| k.tag)
            .collect()
    }
}

/// DS visibility state reported to named via `rndc dnssec -checkds`
/// (ADR-0001: exposed so the insecure transition can complete without host
/// access when no parental agents are configured).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum CheckdsState {
    /// The DS for the key has been published at the parent.
    Published,
    /// The DS for the key has been removed from the parent.
    Withdrawn,
}

impl CheckdsState {
    /// The literal argument `rndc dnssec -checkds` expects.
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckdsState::Published => "published",
            CheckdsState::Withdrawn => "withdrawn",
        }
    }
}

/// A DNSKEY resource record parsed from textual (zone file / key file) form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnskeyRecord {
    /// Owner name exactly as it appeared in the record (trailing dot kept).
    pub owner: String,
    /// DNSKEY flags field (256 = ZSK, 257 = ZSK+SEP i.e. KSK/CSK).
    pub flags: u16,
    /// DNSKEY protocol field (always 3 per RFC 4034).
    pub protocol: u8,
    /// DNSSEC algorithm number.
    pub algorithm: u8,
    /// Decoded public key material.
    pub public_key: Vec<u8>,
}

/// A computed DS record ready to publish at the parent zone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsRecord {
    /// Key tag of the DNSKEY this DS refers to.
    pub key_tag: u16,
    /// DNSSEC algorithm number of the DNSKEY.
    pub algorithm: u8,
    /// DS digest type (always 2 = SHA-256).
    pub digest_type: u8,
    /// Uppercase hex digest.
    pub digest: String,
}

impl DsRecord {
    /// The DS RDATA in presentation format: `<tag> <alg> <digest type> <digest>`.
    pub fn rdata(&self) -> String {
        format!(
            "{} {} {} {}",
            self.key_tag, self.algorithm, self.digest_type, self.digest
        )
    }
}

impl DnskeyRecord {
    /// True when the SEP (key-signing) flag bit is set.
    pub fn is_ksk(&self) -> bool {
        self.flags & DNSKEY_FLAG_SEP != 0
    }

    /// DNSKEY RDATA in wire format: flags, protocol, algorithm, key material.
    fn rdata_wire(&self) -> Vec<u8> {
        let mut wire = Vec::with_capacity(self.public_key.len() + 4);
        wire.extend_from_slice(&self.flags.to_be_bytes());
        wire.push(self.protocol);
        wire.push(self.algorithm);
        wire.extend_from_slice(&self.public_key);
        wire
    }

    /// Computes the key tag (RFC 4034 appendix B).
    pub fn key_tag(&self) -> u16 {
        let wire = self.rdata_wire();
        let mut acc: u32 = 0;
        for (i, byte) in wire.iter().enumerate() {
            if i % 2 == 0 {
                acc += u32::from(*byte) << 8;
            } else {
                acc += u32::from(*byte);
            }
        }
        acc += acc >> 16;
        (acc & 0xFFFF) as u16
    }

    /// Computes the SHA-256 DS record for this DNSKEY (RFC 4509).
    ///
    /// The digest is `SHA-256(canonical owner name wire format || DNSKEY
    /// RDATA wire format)`; the owner name is lowercased per RFC 4034 §6.2.
    ///
    /// # Errors
    /// Returns [`DnssecError::InvalidOwnerName`] if the owner name cannot be
    /// encoded in wire format (empty/oversized labels, name too long).
    pub fn ds_record(&self) -> Result<DsRecord, DnssecError> {
        let owner_wire = name_to_canonical_wire(&self.owner)?;

        let mut hasher = Sha256::new();
        hasher.update(&owner_wire);
        hasher.update(self.rdata_wire());
        let digest = hasher.finalize();

        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest {
            hex.push_str(&format!("{byte:02X}"));
        }

        Ok(DsRecord {
            key_tag: self.key_tag(),
            algorithm: self.algorithm,
            digest_type: DS_DIGEST_TYPE_SHA256,
            digest: hex,
        })
    }
}

/// Encodes a textual DNS name in canonical (lowercase) wire format.
fn name_to_canonical_wire(name: &str) -> Result<Vec<u8>, DnssecError> {
    let name = name.trim_end_matches('.');
    let mut wire = Vec::with_capacity(name.len() + 2);

    if !name.is_empty() {
        for label in name.split('.') {
            if label.is_empty() || label.len() > MAX_LABEL_LEN {
                return Err(DnssecError::InvalidOwnerName(format!(
                    "label {label:?} is empty or longer than {MAX_LABEL_LEN} octets"
                )));
            }
            wire.push(label.len() as u8);
            wire.extend(label.bytes().map(|b| b.to_ascii_lowercase()));
        }
    }
    wire.push(0);

    if wire.len() > MAX_NAME_LEN {
        return Err(DnssecError::InvalidOwnerName(format!(
            "name {name:?} exceeds {MAX_NAME_LEN} octets in wire format"
        )));
    }
    Ok(wire)
}

/// Decodes base64, tolerating embedded whitespace (key files split the blob).
fn decode_base64(input: &str) -> Result<Vec<u8>, DnssecError> {
    let compact: String = input.split_whitespace().collect();
    base64_decode(&compact).ok_or_else(|| DnssecError::InvalidField {
        field: "public key",
        value: input.to_string(),
    })
}

/// Minimal strict base64 (standard alphabet, `=` padding) decoder.
///
/// Kept local so the always-on library half does not grow a dependency for
/// one call site; the RFC 4509 test vector pins its correctness.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const INVALID: u8 = u8::MAX;
    fn value(byte: u8) -> u8 {
        match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => INVALID,
        }
    }

    let bytes = input.as_bytes();
    let stripped = match bytes {
        [rest @ .., b'=', b'='] => rest,
        [rest @ .., b'='] => rest,
        rest => rest,
    };

    let mut out = Vec::with_capacity(stripped.len() * 3 / 4 + 3);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in stripped {
        let v = value(byte);
        if v == INVALID {
            return None;
        }
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    // Leftover bits must be zero padding only.
    if bits > 0 && acc & ((1 << bits) - 1) != 0 {
        return None;
    }
    Some(out)
}

/// Parses a DNSKEY resource record in presentation format:
/// `<owner> [<ttl>] [<class>] DNSKEY <flags> <protocol> <algorithm> <base64…>`.
///
/// # Errors
/// Returns [`DnssecError::NotDnskey`] when the RR type is not DNSKEY, and
/// [`DnssecError::InvalidField`] when an RDATA field fails to parse.
pub fn parse_dnskey_rr(text: &str) -> Result<DnskeyRecord, DnssecError> {
    let mut tokens = text.split_whitespace().peekable();

    let owner = tokens
        .next()
        .ok_or_else(|| DnssecError::NotDnskey("empty input".to_string()))?
        .to_string();

    // Skip optional TTL and class, then require the DNSKEY type token.
    let mut saw_dnskey = false;
    for token in tokens.by_ref() {
        if token.eq_ignore_ascii_case("DNSKEY") {
            saw_dnskey = true;
            break;
        }
        let is_ttl = token.chars().all(|c| c.is_ascii_digit());
        let is_class = token.eq_ignore_ascii_case("IN");
        if !is_ttl && !is_class {
            return Err(DnssecError::NotDnskey(format!(
                "unexpected token {token:?} before DNSKEY type"
            )));
        }
    }
    if !saw_dnskey {
        return Err(DnssecError::NotDnskey(
            "no DNSKEY type token found".to_string(),
        ));
    }

    let flags_token = tokens.next().ok_or(DnssecError::InvalidField {
        field: "flags",
        value: "missing".to_string(),
    })?;
    let flags: u16 = flags_token.parse().map_err(|_| DnssecError::InvalidField {
        field: "flags",
        value: flags_token.to_string(),
    })?;

    let protocol_token = tokens.next().ok_or(DnssecError::InvalidField {
        field: "protocol",
        value: "missing".to_string(),
    })?;
    let protocol: u8 = protocol_token
        .parse()
        .map_err(|_| DnssecError::InvalidField {
            field: "protocol",
            value: protocol_token.to_string(),
        })?;

    let algorithm_token = tokens.next().ok_or(DnssecError::InvalidField {
        field: "algorithm",
        value: "missing".to_string(),
    })?;
    let algorithm: u8 = algorithm_token
        .parse()
        .map_err(|_| DnssecError::InvalidField {
            field: "algorithm",
            value: algorithm_token.to_string(),
        })?;

    let key_b64: String = tokens.collect::<Vec<_>>().join(" ");
    if key_b64.is_empty() {
        return Err(DnssecError::InvalidField {
            field: "public key",
            value: "missing".to_string(),
        });
    }
    let public_key = decode_base64(&key_b64)?;

    Ok(DnskeyRecord {
        owner,
        flags,
        protocol,
        algorithm,
        public_key,
    })
}

/// Parses a BIND public key file (`K<zone>.+<alg>+<tag>.key`): comment lines
/// start with `;`, the remaining non-empty line is the DNSKEY RR.
///
/// # Errors
/// Returns [`DnssecError::NotDnskey`] if no DNSKEY record line is present,
/// or the underlying [`parse_dnskey_rr`] error for a malformed record line.
pub fn parse_key_file(contents: &str) -> Result<DnskeyRecord, DnssecError> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }
        return parse_dnskey_rr(trimmed);
    }
    Err(DnssecError::NotDnskey(
        "no DNSKEY record line in key file".to_string(),
    ))
}
