# Changelog

All notable changes to bindcar will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive RNDC output parser using nom combinators
- Support for parsing `rndc showzone` output into structured ZoneConfig
- CIDR notation handling in IP address lists (e.g., `10.0.0.1/32`)
- Key-based `allow-update` directive parsing (key references ignored)
- Support for both modern (`primary`/`secondary`) and legacy (`master`/`slave`) BIND9 terminology
- Round-trip serialization: parse → modify → serialize → apply
- Zone modification via PATCH /api/v1/zones/{name} for `also-notify` and `allow-transfer`

### Changed
- Zone modification now uses `rndc showzone` instead of `rndc zonestatus` for full configuration retrieval
- RNDC errors now return 500 Internal Server Error (was 502 Bad Gateway)
- Raw RNDC error messages returned to clients (no wrapper text)

### Fixed
- `rndc modzone` now sends complete zone definition including type (was causing "zone type not specified" errors)
- Parser handles real-world BIND9 output with CIDR notation and TSIG keys

### Documentation
- Added comprehensive RNDC parser documentation in developer guide
- Added parser architecture diagrams and usage examples
- Documented CIDR stripping rationale and key-based ACL handling
- Created roadmap for rndc.conf parser implementation

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
