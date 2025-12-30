// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! RNDC output parser
//!
//! This module provides parsers for BIND9 RNDC command outputs using nom.
//!
//! # Examples
//!
//! ```rust
//! use bindcar::rndc_parser::parse_showzone;
//!
//! let output = r#"zone "example.com" { type primary; file "/var/cache/bind/example.com.zone"; };"#;
//! let config = parse_showzone(output).unwrap();
//! assert_eq!(config.zone_name, "example.com");
//! ```

use crate::rndc_types::{DnsClass, PrimarySpec, ZoneConfig, ZoneType};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, multispace0},
    combinator::{map, opt, recognize},
    multi::many0,
    sequence::{delimited, preceded, terminated},
    IResult,
};
use std::net::IpAddr;
use thiserror::Error;

/// RNDC parse errors
#[derive(Debug, Error)]
pub enum RndcParseError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid zone type: {0}")]
    InvalidZoneType(String),

    #[error("Invalid DNS class: {0}")]
    InvalidDnsClass(String),

    #[error("Invalid IP address: {0}")]
    InvalidIpAddress(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Incomplete input")]
    Incomplete,
}

pub type ParseResult<T> = Result<T, RndcParseError>;

// ========== Common Parser Primitives ==========

/// Skip whitespace around a parser
fn ws<'a, F, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
{
    delimited(multispace0, inner, multispace0)
}

/// Parse a semicolon
fn semicolon(input: &str) -> IResult<&str, char> {
    ws(char(';'))(input)
}

/// Parse a quoted string: "example"
pub(crate) fn quoted_string(input: &str) -> IResult<&str, String> {
    let (input, content) = delimited(char('"'), take_until("\""), char('"'))(input)?;
    Ok((input, content.to_string()))
}

/// Parse an identifier (alphanumeric with hyphens and underscores)
fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '-')(input)
}

/// Parse an IP address (IPv4 or IPv6), optionally with CIDR notation
/// Examples: 192.168.1.1, 192.168.1.1/32, 2001:db8::1, 2001:db8::1/128
pub(crate) fn ip_addr(input: &str) -> IResult<&str, IpAddr> {
    // Try to parse as much as possible that looks like an IP address
    let (input, addr_str) = recognize(take_while1(|c: char| {
        c.is_ascii_hexdigit() || c == '.' || c == ':'
    }))(input)?;

    // Try to parse the string as an IP address
    let addr = match addr_str.parse::<IpAddr>() {
        Ok(addr) => addr,
        Err(_) => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Verify,
            )))
        }
    };

    // Check for optional CIDR suffix (e.g., /32 or /128) and consume it
    let (input, _) = opt(preceded(
        char('/'),
        take_while1(|c: char| c.is_numeric()),
    ))(input)?;

    Ok((input, addr))
}

/// Parse IP address with optional port
pub(crate) fn ip_with_port(input: &str) -> IResult<&str, PrimarySpec> {
    let (input, addr) = ws(ip_addr)(input)?;
    let (input, port) = opt(preceded(
        ws(tag("port")),
        map(take_while1(|c: char| c.is_numeric()), |s: &str| {
            s.parse::<u16>().ok()
        }),
    ))(input)?;

    Ok((
        input,
        PrimarySpec {
            address: addr,
            port: port.flatten(),
        },
    ))
}

/// Parse a list of IP addresses: { addr; addr; }
fn ip_list(input: &str) -> IResult<&str, Vec<IpAddr>> {
    delimited(
        ws(char('{')),
        many0(terminated(ws(ip_addr), semicolon)),
        ws(char('}')),
    )(input)
}

/// Parse a list of primary specs: { addr; addr port 5353; }
fn primary_list(input: &str) -> IResult<&str, Vec<PrimarySpec>> {
    delimited(
        ws(char('{')),
        many0(terminated(ip_with_port, semicolon)),
        ws(char('}')),
    )(input)
}

// ========== Zone Configuration Parser ==========

/// Statement types within a zone configuration
#[derive(Debug)]
enum ZoneStatement {
    Type(ZoneType),
    File(String),
    Primaries(Vec<PrimarySpec>),
    AlsoNotify(Vec<IpAddr>),
    AllowTransfer(Vec<IpAddr>),
    AllowUpdate(Vec<IpAddr>),
    AllowUpdateRaw(String), // Raw allow-update directive for key-based updates
}

/// Parse zone type statement: type primary;
fn parse_type_statement(input: &str) -> IResult<&str, ZoneStatement> {
    let (input, _) = ws(tag("type"))(input)?;
    let (input, type_str) = ws(identifier)(input)?;
    let (input, _) = semicolon(input)?;

    let zone_type = ZoneType::parse(type_str).ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        ))
    })?;

    Ok((input, ZoneStatement::Type(zone_type)))
}

/// Parse file statement: file "/path/to/file";
fn parse_file_statement(input: &str) -> IResult<&str, ZoneStatement> {
    let (input, _) = ws(tag("file"))(input)?;
    let (input, file) = ws(quoted_string)(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, ZoneStatement::File(file)))
}

/// Parse primaries statement: primaries { addr; addr port 5353; };
/// Also handles legacy "masters" keyword
fn parse_primaries_statement(input: &str) -> IResult<&str, ZoneStatement> {
    let (input, _) = ws(alt((tag("primaries"), tag("masters"))))(input)?;
    let (input, primaries) = primary_list(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, ZoneStatement::Primaries(primaries)))
}

/// Parse also-notify statement: also-notify { addr; addr; };
fn parse_also_notify_statement(input: &str) -> IResult<&str, ZoneStatement> {
    let (input, _) = ws(tag("also-notify"))(input)?;
    let (input, addrs) = ip_list(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, ZoneStatement::AlsoNotify(addrs)))
}

/// Parse allow-transfer statement: allow-transfer { addr; addr; };
fn parse_allow_transfer_statement(input: &str) -> IResult<&str, ZoneStatement> {
    let (input, _) = ws(tag("allow-transfer"))(input)?;
    let (input, addrs) = ip_list(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, ZoneStatement::AllowTransfer(addrs)))
}

/// Parse allow-update statement: allow-update { addr; addr; }; or allow-update { key "name"; };
/// Captures both IP addresses and raw directive for key-based updates
fn parse_allow_update_statement(input: &str) -> IResult<&str, ZoneStatement> {
    let (input, _) = ws(tag("allow-update"))(input)?;

    // Capture the start position to extract raw content
    let start_input = input;

    // Parse the content between braces
    let (input, _) = ws(char('{'))(input)?;

    // Collect IP addresses and check for key references
    let mut addrs = Vec::new();
    let mut has_key_ref = false;
    let mut remaining = input;

    loop {
        // Skip whitespace
        let (input, _) = multispace0(remaining)?;

        // Check if we've reached the closing brace
        if let Ok((input, _)) = char::<_, nom::error::Error<&str>>('}')(input) {
            remaining = input;
            break;
        }

        // Check for "key" keyword
        if let Ok((input, _)) = ws(tag("key"))(input) {
            has_key_ref = true;
            // Skip to the next semicolon
            let (input, _) = take_until(";")(input)?;
            let (input, _) = char(';')(input)?;
            remaining = input;
        } else if let Ok((input, addr)) = ip_addr(input) {
            // Try to parse an IP address
            addrs.push(addr);
            // Consume the semicolon
            let (input, _) = semicolon(input)?;
            remaining = input;
        } else {
            // Skip to the next semicolon (handles other unknown statements)
            let (input, _) = take_until(";")(input)?;
            let (input, _) = char(';')(input)?;
            remaining = input;
        }
    }

    let (input, _) = semicolon(remaining)?;

    // If we found key references, capture the raw directive
    if has_key_ref {
        // Extract raw content from start to end
        let raw_len = start_input.len() - input.len();
        let raw_content = &start_input[..raw_len];
        Ok((input, ZoneStatement::AllowUpdateRaw(raw_content.to_string())))
    } else {
        Ok((input, ZoneStatement::AllowUpdate(addrs)))
    }
}

/// Parse any zone statement
fn parse_zone_statement(input: &str) -> IResult<&str, ZoneStatement> {
    alt((
        parse_type_statement,
        parse_file_statement,
        parse_primaries_statement,
        parse_also_notify_statement,
        parse_allow_transfer_statement,
        parse_allow_update_statement,
    ))(input)
}

/// Parse complete zone configuration from showzone output
///
/// Parses output from `rndc showzone <zonename>` which has the format:
/// ```text
/// zone "example.com" { type primary; file "/path"; };
/// ```
///
/// Or with optional class:
/// ```text
/// zone "example.com" IN { type primary; file "/path"; };
/// ```
fn parse_zone_config_internal(input: &str) -> IResult<&str, ZoneConfig> {
    // Parse: zone "name" [class] { statements };
    let (input, _) = ws(tag("zone"))(input)?;
    let (input, zone_name) = ws(quoted_string)(input)?;

    // Optional class (IN, CH, HS)
    let (input, class) = opt(ws(alt((tag("IN"), tag("CH"), tag("HS")))))(input)?;
    let class = match class {
        Some("IN") => DnsClass::IN,
        Some("CH") => DnsClass::CH,
        Some("HS") => DnsClass::HS,
        _ => DnsClass::IN, // Default to IN
    };

    // Parse zone block
    let (input, statements) = delimited(
        ws(char('{')),
        many0(parse_zone_statement),
        ws(tag("};"))
    )(input)?;

    // Build ZoneConfig from statements
    let mut config = ZoneConfig::new(zone_name, ZoneType::Primary); // Default type
    config.class = class;

    for stmt in statements {
        match stmt {
            ZoneStatement::Type(t) => config.zone_type = t,
            ZoneStatement::File(f) => config.file = Some(f),
            ZoneStatement::Primaries(p) => config.primaries = Some(p),
            ZoneStatement::AlsoNotify(a) => config.also_notify = Some(a),
            ZoneStatement::AllowTransfer(a) => config.allow_transfer = Some(a),
            ZoneStatement::AllowUpdate(a) => config.allow_update = Some(a),
            ZoneStatement::AllowUpdateRaw(raw) => config.allow_update_raw = Some(raw),
        }
    }

    Ok((input, config))
}

/// Parse `rndc showzone` output
///
/// # Examples
///
/// ```rust
/// use bindcar::rndc_parser::parse_showzone;
///
/// let output = r#"zone "example.com" { type primary; file "/var/cache/bind/example.com.zone"; };"#;
/// let config = parse_showzone(output).unwrap();
/// assert_eq!(config.zone_name, "example.com");
/// ```
pub fn parse_showzone(input: &str) -> ParseResult<ZoneConfig> {
    match parse_zone_config_internal(input.trim()) {
        Ok((_, config)) => Ok(config),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            Err(RndcParseError::ParseError(format!("Parse failed: {:?}", e)))
        }
        Err(nom::Err::Incomplete(_)) => Err(RndcParseError::Incomplete),
    }
}
