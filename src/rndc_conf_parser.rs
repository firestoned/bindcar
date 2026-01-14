// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! RNDC configuration file parser
//!
//! This module provides parsers for BIND9 rndc.conf files using nom.
//!
//! # Examples
//!
//! ```rust
//! use bindcar::rndc_conf_parser::parse_rndc_conf_str;
//!
//! let conf_str = r#"
//! key "rndc-key" {
//!     algorithm hmac-sha256;
//!     secret "dGVzdC1zZWNyZXQ=";
//! };
//!
//! options {
//!     default-server localhost;
//!     default-key "rndc-key";
//! };
//! "#;
//!
//! let config = parse_rndc_conf_str(conf_str).unwrap();
//! assert_eq!(config.keys.len(), 1);
//! ```

use crate::rndc_conf_types::{
    KeyBlock, OptionsBlock, RndcConfFile, ServerAddress, ServerBlock,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while, take_while1},
    character::complete::{char, digit1, multispace0, multispace1},
    combinator::{map, recognize, value},
    multi::{many0, separated_list0},
    sequence::{delimited, preceded, tuple},
    IResult,
};
use std::collections::HashSet;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// RNDC configuration parse errors
#[derive(Debug, Error)]
pub enum RndcConfParseError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid server address: {0}")]
    InvalidServerAddress(String),

    #[error("Invalid IP address: {0}")]
    InvalidIpAddress(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Circular include detected: {0}")]
    CircularInclude(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Incomplete input")]
    Incomplete,
}

pub type ParseResult<T> = Result<T, RndcConfParseError>;

// ========== Comment and Whitespace Parsers ==========

/// Parse C-style line comment: // comment
fn line_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = tag("//")(input)?;
    let (input, _) = take_while(|c| c != '\n')(input)?;
    Ok((input, ()))
}

/// Parse hash comment: # comment
fn hash_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = char('#')(input)?;
    let (input, _) = take_while(|c| c != '\n')(input)?;
    Ok((input, ()))
}

/// Parse C-style block comment: /* comment */
fn block_comment(input: &str) -> IResult<&str, ()> {
    value((), tuple((tag("/*"), take_until("*/"), tag("*/"))))(input)
}

/// Parse any type of comment
fn comment(input: &str) -> IResult<&str, ()> {
    alt((line_comment, hash_comment, block_comment))(input)
}

/// Skip whitespace and comments
fn ws<'a, F, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
{
    delimited(
        many0(alt((value((), multispace1), comment))),
        inner,
        many0(alt((value((), multispace1), comment))),
    )
}

/// Parse a semicolon with surrounding whitespace
fn semicolon(input: &str) -> IResult<&str, char> {
    ws(char(';'))(input)
}

// ========== String and Identifier Parsers ==========

/// Parse escaped character in quoted string
fn escaped_char(input: &str) -> IResult<&str, char> {
    preceded(
        char('\\'),
        alt((
            value('"', char('"')),
            value('\\', char('\\')),
            value('\n', char('n')),
            value('\r', char('r')),
            value('\t', char('t')),
        )),
    )(input)
}

/// Parse quoted string with escape sequences: "example"
fn quoted_string(input: &str) -> IResult<&str, String> {
    delimited(
        char('"'),
        map(
            many0(alt((
                map(escaped_char, |c| c.to_string()),
                map(take_while1(|c| c != '"' && c != '\\'), |s: &str| {
                    s.to_string()
                }),
            ))),
            |parts| parts.join(""),
        ),
        char('"'),
    )(input)
}

/// Parse an identifier (alphanumeric with hyphens, underscores, dots, and colons)
/// Examples: rndc-key, hmac-sha256, 127.0.0.1, localhost, 2001:db8::1
fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == ':')(
        input,
    )
}

// ========== IP Address and Port Parsers ==========

/// Parse an IPv4 address
fn ipv4_addr(input: &str) -> IResult<&str, IpAddr> {
    let (input, addr_str) = recognize(tuple((
        digit1,
        char('.'),
        digit1,
        char('.'),
        digit1,
        char('.'),
        digit1,
    )))(input)?;

    let addr = match addr_str.parse::<IpAddr>() {
        Ok(addr) => addr,
        Err(_) => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Verify,
            )))
        }
    };

    Ok((input, addr))
}

/// Parse an IPv6 address
fn ipv6_addr(input: &str) -> IResult<&str, IpAddr> {
    let (input, addr_str) = recognize(take_while1(|c: char| {
        c.is_ascii_hexdigit() || c == ':'
    }))(input)?;

    // Must contain at least two colons to be valid IPv6
    if !addr_str.contains("::") && addr_str.matches(':').count() < 2 {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        )));
    }

    let addr = match addr_str.parse::<IpAddr>() {
        Ok(addr) => addr,
        Err(_) => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Verify,
            )))
        }
    };

    Ok((input, addr))
}

/// Parse an IP address (IPv4 or IPv6)
fn ip_addr(input: &str) -> IResult<&str, IpAddr> {
    alt((ipv6_addr, ipv4_addr))(input)
}

/// Parse a port number
fn port_number(input: &str) -> IResult<&str, u16> {
    map(digit1, |s: &str| s.parse::<u16>().unwrap_or(953))(input)
}

/// Parse a server address (hostname or IP)
fn server_address(input: &str) -> IResult<&str, ServerAddress> {
    // Try IP address first, then fall back to hostname
    alt((
        map(ip_addr, ServerAddress::IpAddr),
        map(identifier, |s: &str| ServerAddress::Hostname(s.to_string())),
    ))(input)
}

// ========== Key Block Parser ==========

/// Key block field types
#[derive(Debug)]
enum KeyField {
    Algorithm(String),
    Secret(String),
}

/// Parse algorithm field: algorithm hmac-sha256;
fn parse_algorithm_field(input: &str) -> IResult<&str, KeyField> {
    let (input, _) = ws(tag("algorithm"))(input)?;
    let (input, algo) = ws(identifier)(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, KeyField::Algorithm(algo.to_string())))
}

/// Parse secret field: secret "base64string";
fn parse_secret_field(input: &str) -> IResult<&str, KeyField> {
    let (input, _) = ws(tag("secret"))(input)?;
    let (input, secret) = ws(quoted_string)(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, KeyField::Secret(secret)))
}

/// Parse key field
fn parse_key_field(input: &str) -> IResult<&str, KeyField> {
    alt((parse_algorithm_field, parse_secret_field))(input)
}

/// Parse key block: key "name" { algorithm ...; secret "..."; };
fn parse_key_block(input: &str) -> IResult<&str, (String, KeyBlock)> {
    let (input, _) = ws(tag("key"))(input)?;
    let (input, name) = ws(quoted_string)(input)?;
    let (input, fields) = delimited(
        ws(char('{')),
        many0(parse_key_field),
        ws(tag("};")),
    )(input)?;

    let mut algorithm = None;
    let mut secret = None;

    for field in fields {
        match field {
            KeyField::Algorithm(a) => algorithm = Some(a),
            KeyField::Secret(s) => secret = Some(s),
        }
    }

    let key_block = KeyBlock {
        name: name.clone(),
        algorithm: algorithm.unwrap_or_else(|| "hmac-sha256".to_string()),
        secret: secret.unwrap_or_default(),
    };

    Ok((input, (name, key_block)))
}

// ========== Server Block Parser ==========

/// Server block field types
#[derive(Debug)]
enum ServerField {
    Key(String),
    Port(u16),
    Addresses(Vec<IpAddr>),
}

/// Parse key field: key "keyname";
fn parse_server_key_field(input: &str) -> IResult<&str, ServerField> {
    let (input, _) = ws(tag("key"))(input)?;
    let (input, key) = ws(quoted_string)(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, ServerField::Key(key)))
}

/// Parse port field: port 953;
fn parse_server_port_field(input: &str) -> IResult<&str, ServerField> {
    let (input, _) = ws(tag("port"))(input)?;
    let (input, port) = ws(port_number)(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, ServerField::Port(port)))
}

/// Parse addresses field: addresses { ip; ip; };
fn parse_server_addresses_field(input: &str) -> IResult<&str, ServerField> {
    let (input, _) = ws(tag("addresses"))(input)?;
    let (input, addrs) = delimited(
        ws(char('{')),
        separated_list0(semicolon, ws(ip_addr)),
        ws(tag("};")),
    )(input)?;
    Ok((input, ServerField::Addresses(addrs)))
}

/// Parse server field
fn parse_server_field(input: &str) -> IResult<&str, ServerField> {
    alt((
        parse_server_key_field,
        parse_server_port_field,
        parse_server_addresses_field,
    ))(input)
}

/// Parse server block: server address { key "..."; port 953; };
fn parse_server_block(input: &str) -> IResult<&str, (String, ServerBlock)> {
    let (input, _) = ws(tag("server"))(input)?;
    let (input, addr) = ws(server_address)(input)?;
    let (input, fields) = delimited(
        ws(char('{')),
        many0(parse_server_field),
        ws(tag("};")),
    )(input)?;

    let mut server = ServerBlock::new(addr.clone());

    for field in fields {
        match field {
            ServerField::Key(k) => server.key = Some(k),
            ServerField::Port(p) => server.port = Some(p),
            ServerField::Addresses(a) => server.addresses = Some(a),
        }
    }

    Ok((input, (addr.to_string(), server)))
}

// ========== Options Block Parser ==========

/// Options block field types
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum OptionField {
    DefaultServer(String),
    DefaultKey(String),
    DefaultPort(u16),
}

/// Parse default-server field: default-server localhost;
fn parse_default_server_field(input: &str) -> IResult<&str, OptionField> {
    let (input, _) = ws(tag("default-server"))(input)?;
    let (input, server) = ws(identifier)(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, OptionField::DefaultServer(server.to_string())))
}

/// Parse default-key field: default-key "keyname";
fn parse_default_key_field(input: &str) -> IResult<&str, OptionField> {
    let (input, _) = ws(tag("default-key"))(input)?;
    let (input, key) = ws(quoted_string)(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, OptionField::DefaultKey(key)))
}

/// Parse default-port field: default-port 953;
fn parse_default_port_field(input: &str) -> IResult<&str, OptionField> {
    let (input, _) = ws(tag("default-port"))(input)?;
    let (input, port) = ws(port_number)(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, OptionField::DefaultPort(port)))
}

/// Parse options field
fn parse_option_field(input: &str) -> IResult<&str, OptionField> {
    alt((
        parse_default_server_field,
        parse_default_key_field,
        parse_default_port_field,
    ))(input)
}

/// Parse options block: options { default-server localhost; };
fn parse_options_block(input: &str) -> IResult<&str, OptionsBlock> {
    let (input, _) = ws(tag("options"))(input)?;
    let (input, fields) = delimited(
        ws(char('{')),
        many0(parse_option_field),
        ws(tag("};")),
    )(input)?;

    let mut options = OptionsBlock::new();

    for field in fields {
        match field {
            OptionField::DefaultServer(s) => options.default_server = Some(s),
            OptionField::DefaultKey(k) => options.default_key = Some(k),
            OptionField::DefaultPort(p) => options.default_port = Some(p),
        }
    }

    Ok((input, options))
}

// ========== Include Statement Parser ==========

/// Parse include statement: include "/path/to/file";
fn parse_include_stmt(input: &str) -> IResult<&str, PathBuf> {
    let (input, _) = ws(tag("include"))(input)?;
    let (input, path) = ws(quoted_string)(input)?;
    let (input, _) = semicolon(input)?;
    Ok((input, PathBuf::from(path)))
}

// ========== Statement Parser ==========

/// Statement types in rndc.conf
#[derive(Debug)]
enum Statement {
    Include(PathBuf),
    Key(String, KeyBlock),
    Server(String, ServerBlock),
    Options(OptionsBlock),
}

/// Parse any statement
fn parse_statement(input: &str) -> IResult<&str, Statement> {
    alt((
        map(parse_include_stmt, Statement::Include),
        map(parse_key_block, |(name, key)| Statement::Key(name, key)),
        map(parse_server_block, |(addr, srv)| {
            Statement::Server(addr, srv)
        }),
        map(parse_options_block, Statement::Options),
    ))(input)
}

// ========== File Parser ==========

/// Parse rndc.conf file content (internal)
fn parse_rndc_conf_internal(input: &str) -> IResult<&str, RndcConfFile> {
    let (input, statements) = many0(ws(parse_statement))(input)?;
    let (input, _) = multispace0(input)?;

    let mut conf = RndcConfFile::new();

    for stmt in statements {
        match stmt {
            Statement::Include(path) => conf.includes.push(path),
            Statement::Key(name, key) => {
                conf.keys.insert(name, key);
            }
            Statement::Server(addr, server) => {
                conf.servers.insert(addr, server);
            }
            Statement::Options(opts) => {
                // Merge options (last one wins)
                if opts.default_server.is_some() {
                    conf.options.default_server = opts.default_server;
                }
                if opts.default_key.is_some() {
                    conf.options.default_key = opts.default_key;
                }
                if opts.default_port.is_some() {
                    conf.options.default_port = opts.default_port;
                }
            }
        }
    }

    Ok((input, conf))
}

/// Parse rndc.conf from string
///
/// # Examples
///
/// ```rust
/// use bindcar::rndc_conf_parser::parse_rndc_conf_str;
///
/// let conf_str = r#"
/// key "rndc-key" {
///     algorithm hmac-sha256;
///     secret "dGVzdC1zZWNyZXQ=";
/// };
/// "#;
///
/// let config = parse_rndc_conf_str(conf_str).unwrap();
/// assert_eq!(config.keys.len(), 1);
/// ```
pub fn parse_rndc_conf_str(input: &str) -> ParseResult<RndcConfFile> {
    match parse_rndc_conf_internal(input) {
        Ok((_, conf)) => Ok(conf),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            Err(RndcConfParseError::ParseError(format!("{:?}", e)))
        }
        Err(nom::Err::Incomplete(_)) => Err(RndcConfParseError::Incomplete),
    }
}

/// Parse rndc.conf from file with include resolution
///
/// Handles include directives and detects circular includes.
///
/// # Examples
///
/// ```rust,no_run
/// use bindcar::rndc_conf_parser::parse_rndc_conf_file;
/// use std::path::Path;
///
/// let config = parse_rndc_conf_file(Path::new("/etc/bind/rndc.conf")).unwrap();
/// ```
pub fn parse_rndc_conf_file(path: &Path) -> ParseResult<RndcConfFile> {
    let mut visited = HashSet::new();
    parse_rndc_conf_file_recursive(path, &mut visited)
}

/// Recursively parse rndc.conf file with include resolution
fn parse_rndc_conf_file_recursive(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
) -> ParseResult<RndcConfFile> {
    // Check for circular includes
    let canonical_path = path
        .canonicalize()
        .map_err(|_| RndcConfParseError::FileNotFound(path.display().to_string()))?;

    if visited.contains(&canonical_path) {
        return Err(RndcConfParseError::CircularInclude(
            canonical_path.display().to_string(),
        ));
    }

    visited.insert(canonical_path.clone());

    // Read and parse main file
    let content = std::fs::read_to_string(path)?;
    let mut conf = parse_rndc_conf_str(&content)?;

    // Resolve includes
    let includes = conf.includes.clone();
    conf.includes.clear();

    for include_path in includes {
        // Resolve relative paths
        let resolved_path = if include_path.is_absolute() {
            include_path
        } else {
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(include_path)
        };

        // Parse included file
        let included_conf = parse_rndc_conf_file_recursive(&resolved_path, visited)?;

        // Merge configurations
        for (name, key) in included_conf.keys {
            conf.keys.entry(name).or_insert(key);
        }

        for (addr, server) in included_conf.servers {
            conf.servers.entry(addr).or_insert(server);
        }

        // Merge options (main file takes precedence)
        if conf.options.default_server.is_none() {
            conf.options.default_server = included_conf.options.default_server;
        }
        if conf.options.default_key.is_none() {
            conf.options.default_key = included_conf.options.default_key;
        }
        if conf.options.default_port.is_none() {
            conf.options.default_port = included_conf.options.default_port;
        }

        conf.includes.push(resolved_path);
    }

    Ok(conf)
}

#[cfg(test)]
#[path = "rndc_conf_parser_tests.rs"]
mod tests;
