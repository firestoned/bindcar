# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- **DNSSEC Support**: Native support for DNSSEC (DNS Security Extensions) via BIND9 9.16+ policies
  - New `dnssecPolicy` field in `ZoneConfig` to specify DNSSEC policy name
  - New `inlineSigning` field to enable automatic inline signing
  - Full integration with BIND9's `dnssec-policy` and `inline-signing` directives
  - Automatic key generation and management by BIND9
  - Comprehensive DNSSEC documentation in user guide and advanced topics
  - API examples and Rust library usage patterns

### Changed
- Updated `ZoneConfig` struct with optional DNSSEC fields (backward compatible)
- Enhanced zone creation endpoint to support DNSSEC configuration
- Updated all documentation with DNSSEC examples and best practices

### Documentation
- Added comprehensive [DNSSEC Guide](docs/src/advanced/dnssec.md)
- Updated [Zone Configuration](docs/src/user-guide/zone-config.md) with DNSSEC fields
- Updated [API Reference](docs/src/reference/api-zones.md) with DNSSEC examples
- Added DNSSEC examples to code documentation and doctests

## [2025-12-03] - Initial Release

**Author:** Erick Bourgeois

### Added
- HTTP REST API for managing BIND9 zones via rndc commands
- Structured zone configuration with type-safe JSON API
- Support for SOA records, NS records, and all common DNS record types
- Bearer token authentication
- Health check and readiness endpoints
- RNDC executor with async command execution
- Zone file generation from structured configuration
- Comprehensive test coverage

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

### Impact
- [x] Initial standalone release
- [ ] Breaking change
- [ ] Requires migration
