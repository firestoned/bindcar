# Changelog

All notable changes to bindcar will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Enhanced Zone Configuration Support** - Full BIND9 zone option preservation
  - Added 30+ new structured fields to `ZoneConfig` (organized by category: access control, transfer control, DNSSEC, forwarding, zone maintenance, etc.)
  - Added 6 new enum types: `NotifyMode`, `ForwardMode`, `AutoDnssecMode`, `CheckNamesMode`, `MasterfileFormat`, `ForwarderSpec`
  - Added `raw_options: HashMap<String, String>` catch-all for unrecognized BIND9 zone options
  - Catch-all parser automatically preserves any unknown BIND9 zone options
  - Full round-trip preservation: parse → modify → serialize with zero data loss
  - Support for TSIG key references in `allow-update` via `allow_update_raw` field
  - 43 comprehensive tests for new types and serialization in `rndc_types_tests.rs`
  - 11 tests for unknown option preservation in `rndc_parser_tests.rs`
- Comprehensive RNDC output parser using nom combinators
- Support for parsing `rndc showzone` output into structured ZoneConfig
- CIDR notation handling in IP address lists (e.g., `10.0.0.1/32`)
- Support for both modern (`primary`/`secondary`) and legacy (`master`/`slave`) BIND9 terminology
- Round-trip serialization: parse → modify → serialize → apply
- Zone modification via PATCH /api/v1/zones/{name} for `also-notify`, `allow-transfer`, and `allow-update`

### Changed
- **ZoneConfig Structure** - Enhanced with 30+ optional fields organized by category (all backward compatible)
- **Parser** - Now preserves all unknown options in `raw_options` HashMap via catch-all parser
- **Serializer** - Extended `to_rndc_block()` to serialize all new fields and raw options
- Zone modification now uses `rndc showzone` instead of `rndc zonestatus` for full configuration retrieval
- RNDC errors now return 500 Internal Server Error (was 502 Bad Gateway)
- Raw RNDC error messages returned to clients (no wrapper text)

### Fixed
- PATCH operations now preserve key-based `allow-update` directives through `allow_update_raw` field
- Fixed double semicolon bug in serialization of raw directives (now strips trailing semicolons before joining)
- `rndc modzone` now sends complete zone definition including type (was causing "zone type not specified" errors)
- Parser handles real-world BIND9 output with CIDR notation and TSIG keys
- PATCH operations now preserve key-based `allow-update` directives when modifying other fields
- Fixed double semicolon bug in serialization of raw directives

### Documentation
- Added comprehensive RNDC parser documentation in developer guide
- Enhanced RNDC parser docs with new ZoneConfig fields and unknown option preservation
- Added parser architecture diagrams and usage examples
- Documented CIDR stripping rationale and key-based ACL handling
- Created roadmap for BIND9 full zone configuration support
- Updated developer guide with enhanced ZoneConfig structure and capabilities
- Added 11 new tests for unknown option preservation and round-trip serialization

## [0.1.0] - 2025-12-03

### Added
- Initial release of bindcar
- HTTP REST API for managing BIND9 zones via rndc commands
- Structured zone configuration with type-safe JSON API
- Support for SOA records, NS records, and all common DNS record types
- Bearer token authentication
- Health check and readiness endpoints
- RNDC executor with async command execution
- Zone file generation from structured configuration
- Comprehensive test coverage
- Docker container support
- Kubernetes sidecar deployment pattern

### Features
- **Zone Management**: Create, delete, reload zones
- **Zone Operations**: Freeze, thaw, notify secondaries
- **Status Checks**: Server status and per-zone status
- **Authentication**: ServiceAccount token-based auth
- **Logging**: Structured JSON logging with tracing
- **Security**: Non-root execution, input validation

### Why
Extracted from the [bindy Kubernetes operator](https://github.com/firestoned/bindy) to provide
a standalone HTTP API for BIND9 management that can be used in any environment (Docker, VMs,
bare metal) - not just Kubernetes.

[Unreleased]: https://github.com/firestoned/bindcar/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/firestoned/bindcar/releases/tag/v0.1.0
