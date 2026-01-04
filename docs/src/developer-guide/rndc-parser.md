# RNDC Parser

The RNDC parser provides structured parsing of BIND9 RNDC command outputs using the `nom` parser combinator library. This enables reliable parsing and manipulation of zone configurations.

## Overview

bindcar includes parsers for RNDC command outputs to enable structured zone configuration management:

- **Parse** - Convert RNDC text output to structured Rust types
- **Modify** - Update zone configuration programmatically
- **Serialize** - Convert back to RNDC format for execution
- **Validate** - Ensure configuration correctness

## Architecture

```mermaid
graph TD
    A[RNDC showzone Output] --> B[nom Parser]
    B --> C[ZoneConfig struct]
    C --> D[Modify fields]
    D --> E[Serialize to RNDC format]
    E --> F[Execute rndc modzone]

    G[Real BIND9 Output] --> H[Parser Features]
    H --> I[CIDR Notation]
    H --> J[Key References]
    H --> K[Legacy Terminology]

    style B fill:#e1f5ff
    style C fill:#c8e6c9
    style H fill:#fff3e0
```

## Supported Commands

### showzone

Parse `rndc showzone <zone>` output:

```rust
use bindcar::rndc_parser::parse_showzone;

let output = r#"zone "example.com" {
    type primary;
    file "/var/cache/bind/example.com.zone";
    allow-transfer { 10.0.0.1/32; 10.0.0.2/32; };
    also-notify { 10.0.0.1; 10.0.0.2; };
};"#;

let config = parse_showzone(output)?;
println!("Zone: {}", config.zone_name); // example.com
println!("Type: {:?}", config.zone_type); // Primary
println!("Transfer ACL: {:?}", config.allow_transfer); // [10.0.0.1, 10.0.0.2]
```

## Data Structures

### ZoneConfig

Primary data structure representing a BIND9 zone configuration:

```rust
pub struct ZoneConfig {
    // Core fields
    pub zone_name: String,
    pub class: DnsClass,
    pub zone_type: ZoneType,
    pub file: Option<String>,

    // Primary/Secondary options
    pub primaries: Option<Vec<PrimarySpec>>,
    pub also_notify: Option<Vec<IpAddr>>,
    pub notify: Option<NotifyMode>,

    // Access Control options
    pub allow_query: Option<Vec<IpAddr>>,
    pub allow_transfer: Option<Vec<IpAddr>>,
    pub allow_update: Option<Vec<IpAddr>>,
    pub allow_update_raw: Option<String>,      // Raw directive for key-based updates
    pub allow_update_forwarding: Option<Vec<IpAddr>>,
    pub allow_notify: Option<Vec<IpAddr>>,

    // Transfer Control options
    pub max_transfer_time_in: Option<u32>,
    pub max_transfer_time_out: Option<u32>,
    pub transfer_source: Option<IpAddr>,
    pub transfer_source_v6: Option<IpAddr>,
    pub notify_source: Option<IpAddr>,
    pub notify_source_v6: Option<IpAddr>,
    // ... (and more transfer control options)

    // Dynamic Update options
    pub update_policy: Option<String>,
    pub journal: Option<String>,
    pub ixfr_from_differences: Option<bool>,

    // DNSSEC options
    pub inline_signing: Option<bool>,
    pub auto_dnssec: Option<AutoDnssecMode>,
    pub key_directory: Option<String>,
    // ... (and more DNSSEC options)

    // Forwarding options
    pub forward: Option<ForwardMode>,
    pub forwarders: Option<Vec<ForwarderSpec>>,

    // Zone Maintenance options
    pub check_names: Option<CheckNamesMode>,
    pub check_mx: Option<CheckNamesMode>,
    pub masterfile_format: Option<MasterfileFormat>,
    pub max_zone_ttl: Option<u32>,

    // Refresh/Retry options
    pub max_refresh_time: Option<u32>,
    pub min_refresh_time: Option<u32>,
    pub max_retry_time: Option<u32>,
    pub min_retry_time: Option<u32>,

    // Miscellaneous options
    pub multi_master: Option<bool>,
    pub request_ixfr: Option<bool>,
    pub request_expire: Option<bool>,

    // Generic catch-all for unrecognized options
    pub raw_options: HashMap<String, String>,
}
```

**Key Features:**

- **30+ Structured Fields**: Supports common BIND9 zone options with proper typing
- **Catch-All HashMap**: `raw_options` preserves unknown/custom BIND9 options
- **Full Round-Trip**: All options preserved during parse → modify → serialize cycle
- **Backward Compatible**: All new fields are `Option<T>`

### ZoneType

Supported zone types:

```rust
pub enum ZoneType {
    Primary,      // Authoritative primary zone (formerly "master")
    Secondary,    // Secondary zone (formerly "slave")
    Stub,         // Stub zone
    Forward,      // Forward zone
    Hint,         // Hint zone (root servers)
    Mirror,       // Mirror zone
    Delegation,   // Delegation-only zone
    Redirect,     // Redirect zone
}
```

### PrimarySpec

Primary server specification for secondary zones:

```rust
pub struct PrimarySpec {
    pub address: IpAddr,      // Primary server IP
    pub port: Option<u16>,    // Custom port (default: 53)
}
```

## Parser Features

### CIDR Notation Handling

BIND9 outputs CIDR notation in ACLs, which the parser automatically strips:

```rust
// BIND9 output includes CIDR
let input = r#"zone "internal.local" {
    allow-transfer { 10.244.1.18/32; 10.244.1.21/32; };
};"#;

let config = parse_showzone(input)?;

// Parser extracts IP addresses only
assert_eq!(config.allow_transfer, Some(vec![
    "10.244.1.18".parse()?,
    "10.244.1.21".parse()?,
]));
```

**Why strip CIDR?**
- bindcar manages IP addresses, not subnet masks
- CIDR information not needed for zone modification
- Simplifies zone configuration updates
- BIND9 automatically adds `/32` (IPv4) or `/128` (IPv6) when missing

### Key-Based Access Control

The parser handles TSIG key references in `allow-update`:

```rust
let input = r#"zone "example.com" {
    allow-update { key "update-key"; };
};"#;

let config = parse_showzone(input)?;

// Key references ignored - only IP addresses extracted
assert_eq!(config.allow_update, Some(vec![])); // Empty
```

**Rationale**:
- TSIG keys managed separately from zone config
- bindcar API focuses on IP-based ACLs
- Prevents accidental modification of key-based permissions
- Key-based updates use BIND9's native TSIG infrastructure

### Legacy Terminology Support

Parser accepts both modern and legacy BIND9 zone type names:

```rust
// Modern terminology (BIND 9.16+)
parse_showzone(r#"zone "a.com" { type primary; };"#)?;
parse_showzone(r#"zone "b.com" { type secondary; };"#)?;

// Legacy terminology (BIND 9.15 and earlier)
parse_showzone(r#"zone "c.com" { type master; };"#)?;
parse_showzone(r#"zone "d.com" { type slave; };"#)?;
```

| Modern Term | Legacy Term | Enum Value |
|-------------|-------------|------------|
| `primary` | `master` | `ZoneType::Primary` |
| `secondary` | `slave` | `ZoneType::Secondary` |

**Benefits**:
- Works with all BIND9 versions
- Gradual migration to modern terminology
- No breaking changes when upgrading BIND9

### Port Number Support

Primary server specifications can include custom ports:

```rust
let input = r#"zone "example.org" {
    type secondary;
    primaries { 192.0.2.1; 192.0.2.2 port 5353; };
};"#;

let config = parse_showzone(input)?;

assert_eq!(config.primaries, Some(vec![
    PrimarySpec { address: "192.0.2.1".parse()?, port: None },
    PrimarySpec { address: "192.0.2.2".parse()?, port: Some(5353) },
]));
```

## Round-Trip Serialization

Parse, modify, and serialize zone configurations:

```rust
// 1. Parse current configuration
let output = rndc_executor.showzone("example.com").await?;
let mut config = parse_showzone(&output)?;

// 2. Modify fields
config.also_notify = Some(vec![
    "10.0.0.3".parse()?,
    "10.0.0.4".parse()?,
]);

config.allow_transfer = Some(vec![
    "10.0.0.3".parse()?,
    "10.0.0.4".parse()?,
]);

// 3. Serialize to RNDC format
let rndc_block = config.to_rndc_block();
// Result: "{ type primary; file \"...\"; also-notify { 10.0.0.3; 10.0.0.4; }; allow-transfer { 10.0.0.3; 10.0.0.4; }; };"

// 4. Apply changes via RNDC
rndc_executor.modzone("example.com", &rndc_block).await?;
```

## Error Handling

### Parser Errors

The parser provides structured error types:

```rust
pub enum RndcParseError {
    ParseError(String),           // General parse failure
    InvalidZoneType(String),      // Unknown zone type
    InvalidDnsClass(String),      // Unknown DNS class
    InvalidIpAddress(String),     // Invalid IP address format
    MissingField(String),         // Required field missing
    Incomplete,                   // Incomplete input
}
```

### Error Messages

```rust
use bindcar::rndc_parser::{parse_showzone, RndcParseError};

match parse_showzone(input) {
    Ok(config) => {
        println!("Parsed zone: {}", config.zone_name);
    },
    Err(RndcParseError::ParseError(msg)) => {
        eprintln!("Parse failed: {}", msg);
    },
    Err(RndcParseError::InvalidZoneType(type_str)) => {
        eprintln!("Unknown zone type: {}", type_str);
    },
    Err(RndcParseError::InvalidIpAddress(addr)) => {
        eprintln!("Invalid IP: {}", addr);
    },
    Err(e) => {
        eprintln!("Parser error: {}", e);
    },
}
```

## Use Cases

### Zone Modification API

The PATCH endpoint uses the parser to update zone configurations:

```rust
// src/zones.rs - modify_zone() function

// Get current configuration from BIND9
let showzone_output = state.rndc.showzone(&zone_name).await?;
let mut zone_config = parse_showzone(&showzone_output)?;

// Update fields from API request
if let Some(also_notify) = &request.also_notify {
    zone_config.also_notify = Some(also_notify.clone());
}

if let Some(allow_transfer) = &request.allow_transfer {
    zone_config.allow_transfer = Some(allow_transfer.clone());
}

// Serialize and apply changes
let rndc_block = zone_config.to_rndc_block();
state.rndc.modzone(&zone_name, &rndc_block).await?;
```

### Zone Inspection

```rust
// Fetch and parse zone configuration
let output = rndc_executor.showzone("example.com").await?;
let config = parse_showzone(&output)?;

// Inspect configuration
println!("Zone: {}", config.zone_name);
println!("Type: {:?}", config.zone_type);
println!("File: {:?}", config.file);
println!("Class: {:?}", config.class);

if let Some(primaries) = &config.primaries {
    println!("Primary servers:");
    for primary in primaries {
        match primary.port {
            Some(port) => println!("  {} port {}", primary.address, port),
            None => println!("  {}", primary.address),
        }
    }
}
```

## Testing

### Unit Tests

The parser includes comprehensive test coverage:

```rust
#[test]
fn test_parse_ip_addr_with_cidr() {
    // Test CIDR notation stripping
    assert_eq!(
        ip_addr("192.168.1.1/32").unwrap().1,
        "192.168.1.1".parse::<IpAddr>().unwrap()
    );
}

#[test]
fn test_parse_exact_production_output() {
    // Real production output
    let input = r#"zone "internal.local" {
        type primary;
        file "/var/cache/bind/internal.local.zone";
        allow-transfer { 10.244.1.18/32; 10.244.1.21/32; };
        allow-update { key "bindy-operator"; };
        also-notify { 10.244.1.18; 10.244.1.21; };
    };"#;

    let config = parse_showzone(input).unwrap();

    assert_eq!(config.zone_name, "internal.local");
    assert_eq!(config.zone_type, ZoneType::Primary);
    assert_eq!(config.allow_transfer.unwrap().len(), 2);
    assert_eq!(config.also_notify.unwrap().len(), 2);
}

#[test]
fn test_roundtrip() {
    // Parse → modify → serialize → parse again
    let input = r#"zone "example.com" {
        type primary;
        file "/var/cache/bind/example.com.zone";
        also-notify { 10.0.0.1; };
    };"#;

    let config = parse_showzone(input).unwrap();
    let serialized = format!("zone \"{}\" {}", config.zone_name, config.to_rndc_block());
    let config2 = parse_showzone(&serialized).unwrap();

    assert_eq!(config.zone_type, config2.zone_type);
    assert_eq!(config.also_notify, config2.also_notify);
}
```

### Running Tests

```bash
# Run all parser tests
cargo test rndc_parser --lib

# Run with output
cargo test rndc_parser --lib -- --nocapture

# Run specific test
cargo test test_parse_exact_production_output --lib
```

## Enhanced Features (v0.6.0+)

### Unknown Option Preservation

The parser now includes a catch-all mechanism that preserves all unknown BIND9 options:

```rust
let input = r#"zone "example.com" {
    type primary;
    file "/var/cache/bind/example.com.zone";
    zone-statistics full;
    check-names warn;
    custom-option { custom value; };
};"#;

let config = parse_showzone(input)?;

// Unknown options preserved in raw_options HashMap
assert_eq!(config.raw_options.get("zone-statistics"), Some(&"full".to_string()));
assert_eq!(config.raw_options.get("check-names"), Some(&"warn".to_string()));
assert_eq!(config.raw_options.get("custom-option"), Some(&"{ custom value; }".to_string()));

// Serialization preserves all options
let serialized = config.to_rndc_block();
assert!(serialized.contains("zone-statistics full"));
assert!(serialized.contains("check-names warn"));
assert!(serialized.contains("custom-option { custom value; }"));
```

**Benefits:**

- **Future-Proof**: New BIND9 options automatically supported
- **No Data Loss**: Complete round-trip preservation
- **Custom Options**: Support for non-standard BIND9 configurations
- **Gradual Migration**: Add structured parsing for popular options over time

### Key-Based Access Control

Enhanced handling of TSIG key references in `allow-update`:

```rust
let input = r#"zone "example.com" {
    allow-update { key "bindy-operator"; };
};"#;

let config = parse_showzone(input)?;

// Raw directive preserved
assert_eq!(config.allow_update_raw, Some("{ key \"bindy-operator\"; };".to_string()));
assert_eq!(config.allow_update, None);

// Serialization preserves key reference
let serialized = config.to_rndc_block();
assert!(serialized.contains("allow-update { key \"bindy-operator\"; }"));
```

**Key Features:**

- Key references preserved in `allow_update_raw` field
- PATCH operations preserve keys when modifying other fields
- Explicit IP setting clears raw directive
- No accidental modification of key-based permissions

## Limitations

### Not Currently Supported (Structured Parsing)

While all options are preserved via `raw_options`, structured parsing is not yet implemented for:

- **ACL Names**: `allow-transfer { "trusted"; };` (preserved as raw)
- **Complex ACLs**: `{ !10.0.0.1; any; };` (preserved as raw)
- **Update Policy**: Complex grammar (preserved as raw string in `update_policy`)
- **Views**: View-specific zone configurations
- **Forwarders with TLS**: `forwarders { 10.1.1.1 tls tls-config; };` (structured type exists, parser pending)

**Note**: All these options are preserved and round-trip correctly through `raw_options` or dedicated raw fields (`allow_update_raw`, `update_policy`).

### Future Enhancements

See the [BIND9 Full Zone Config Support Roadmap](../../roadmaps/bind9-full-zone-config-support.md) for details:

- Structured parsers for common options (notify, forwarders, transfer timeouts)
- ACL name resolution
- View-aware zone configurations
- Parser for `rndc zonestatus` output
- Parser for `rndc status` output

## Implementation Details

### Parser Combinators

The parser uses `nom` combinators for robust, composable parsing:

```rust
// Whitespace handling
fn ws<F>(inner: F) -> impl FnMut(&str) -> IResult<&str, O>

// Quoted strings
fn quoted_string(input: &str) -> IResult<&str, String>

// IP addresses with optional CIDR
fn ip_addr(input: &str) -> IResult<&str, IpAddr>

// IP addresses with optional port
fn ip_with_port(input: &str) -> IResult<&str, PrimarySpec>

// IP address lists
fn ip_list(input: &str) -> IResult<&str, Vec<IpAddr>>

// Zone statements
fn parse_type_statement(input: &str) -> IResult<&str, ZoneStatement>
fn parse_file_statement(input: &str) -> IResult<&str, ZoneStatement>
fn parse_primaries_statement(input: &str) -> IResult<&str, ZoneStatement>
fn parse_also_notify_statement(input: &str) -> IResult<&str, ZoneStatement>
fn parse_allow_transfer_statement(input: &str) -> IResult<&str, ZoneStatement>
fn parse_allow_update_statement(input: &str) -> IResult<&str, ZoneStatement>
```

### Grammar

Simplified BNF grammar for zone configuration:

```
zone_config     ::= "zone" quoted_string [class] "{" statement* "};"
statement       ::= type_stmt | file_stmt | primaries_stmt | notify_stmt | transfer_stmt | update_stmt
type_stmt       ::= "type" identifier ";"
file_stmt       ::= "file" quoted_string ";"
primaries_stmt  ::= ("primaries" | "masters") "{" primary_spec* "};"
notify_stmt     ::= "also-notify" "{" ip_list "};"
transfer_stmt   ::= "allow-transfer" "{" ip_list "};"
update_stmt     ::= "allow-update" "{" (ip_addr | key_ref)* "};"

primary_spec    ::= ip_addr ["port" number] ";"
ip_list         ::= (ip_addr [cidr] ";")*
cidr            ::= "/" number
key_ref         ::= "key" quoted_string ";"
class           ::= "IN" | "CH" | "HS"
```

## Best Practices

1. **Always parse before modify** - Use `showzone` to get complete configuration
2. **Validate IP addresses** - Check IPs before adding to configuration
3. **Test with real data** - Use production BIND9 output in tests
4. **Handle parse errors** - Provide clear error messages to users
5. **Preserve unknown fields** - Don't discard configuration you don't understand
6. **Log parser failures** - Help diagnose issues with BIND9 output format changes
7. **Use round-trip tests** - Ensure serialization produces parseable output

## Related Documentation

- [RNDC Integration](./rndc-integration.md) - RNDC command execution
- [Zone Operations](../user-guide/zone-operations.md) - Using zone modification API
- [API Reference](../reference/api-zones.md) - Zone API endpoints
- [RNDC Conf Parser Roadmap](../../roadmaps/rndc-conf-parser.md) - Future enhancements
