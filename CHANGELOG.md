# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

#### [2026-07-13] - Per-endpoint port support for zone transfer targets

**Author:** Erick Bourgeois

- `src/zones.rs`: `primaries` and `also-notify` entries in `POST /api/v1/zones`
  now accept BIND's per-endpoint port syntax (`"<ip> port <n>"`) in addition to
  a bare IP address, via a new `validate_ip_port_list` validator. The values are
  rendered verbatim into the `rndc addzone` `primaries { ... }` / `also-notify
  { ... }` config literals.
- `allow-transfer` is unchanged (bare IPs only, via `validate_ip_list`), since it
  is an ACL and takes no port.
- Tests: `zones_test::test_validate_ip_port_list_accepts_bare_and_ported_entries`
  and `..._rejects_bad_entries` cover bare IPs, `port <n>`, and rejection of
  malformed/injection inputs.

#### Why
bindy runs `named` and needs cross-pod AXFR/NOTIFY. bindcar 0.7 only accepted
bare IPs for `primaries`/`also-notify`, which forced transfers onto the default
DNS port 53 and therefore required the operand pod to bind port 53 with the
`NET_BIND_SERVICE` capability. Supporting an explicit per-endpoint port lets the
operand run `named` on an unprivileged port (e.g. 5353) without that capability.
The grammar is deliberately narrow (IP literal, exact `port` token, `u16`) so the
existing C-1 `rndc addzone` injection guard is preserved.

#### Impact
- [ ] Breaking change (backward compatible: bare IPs still accepted)
- [ ] Requires cluster rollout
- [ ] Config change only
- [ ] Documentation only

### Security

#### [2026-06-09] - TSIG key out of argv (B-7)

**Author:** Erick Bourgeois

- `src/nsupdate.rs`: the TSIG key is now passed to nsupdate via `-k <keyfile>` — a
  mode-0600 temporary key file removed immediately after the update completes —
  instead of `-y algorithm:keyname:secret` on the command line, which exposed the
  secret to any process on the host via `/proc/<pid>/cmdline`. Key-file fields are
  strictly validated (key-name allowlist, known-HMAC algorithm list, base64-only
  secret) before being rendered into the BIND key-file format.
- `Cargo.toml`: `tempfile` promoted to a runtime dependency.

#### [2026-06-09] - Real authentication by default: shared secret + startup guard + NetworkPolicy (B-4)

**Author:** Erick Bourgeois

- `Cargo.toml`: added `subtle = "2.6"` for constant-time token comparison.
- `src/auth.rs`: added a shared-secret authentication layer. When `BIND_API_TOKEN`
  is set, every request's Bearer token is compared against it in **constant time**
  (`compare_shared_secret`/`validate_shared_secret`) — closing the presence-only gap
  (B-4) where any non-empty token was accepted. Added `is_loopback_host`,
  `has_real_auth`, and `check_startup_auth_posture` (the pure startup-guard decision).
- `src/auth.rs` (`authenticate` middleware): now enforces the shared secret (if set)
  before the optional Kubernetes TokenReview check.
- `src/cli.rs`: added the global `--i-know-this-is-insecure` flag.
- `src/main.rs`: added a **startup guard** — bindcar refuses to start when the API is
  bound to a non-loopback interface without real authentication (no TokenReview
  feature and no `BIND_API_TOKEN`), unless `--i-know-this-is-insecure` /
  `BINDCAR_ALLOW_INSECURE_AUTH=true` is set. Added a configurable bind address
  (`BIND_API_ADDRESS`, default `0.0.0.0`).
- `deploy/networkpolicy.yaml` (new): restricts the bindcar REST API (TCP 8080) to the
  bindy operator pods while keeping DNS (53) open, as defense-in-depth.
- Tests: added `auth_test::b4_auth_posture_tests` covering loopback detection,
  constant-time secret matching/rejection, and every branch of the startup guard.

#### Why
B-4 from the 2026-06-09 security audit: bindcar's default "basic" auth accepted any
non-empty Bearer token (presence-only), and `DISABLE_AUTH` only logged a warning, so a
privileged zone/record API could be silently exposed to any in-cluster pod. bindcar now
requires real authentication (TokenReview or a shared secret) to start on a non-loopback
interface, and provides a constant-time shared-secret mode for non-Kubernetes deployments.

#### Impact
- [x] Breaking change (a non-loopback deployment with presence-only/disabled auth now
      refuses to start unless `BIND_API_TOKEN` is set, the TokenReview feature is enabled,
      the bind is loopback, or `--i-know-this-is-insecure` is passed)
- [ ] Requires cluster rollout
- [ ] Config change only
- [ ] Documentation only

#### [2026-06-09] - Input validation: path traversal + nsupdate/RNDC injection (B-1, B-2, B-3)

**Author:** Erick Bourgeois

- `src/zones.rs`: Added `validate_zone_name` (strict DNS grammar) and
  `validate_rndc_identifier` (safe identifier allowlist). `create_zone` and
  `delete_zone` now reject zone names containing path separators, `..`,
  whitespace, NUL, or other control characters with HTTP 400 — closing the path
  traversal into the zone directory (B-1). `updateKeyName` and `dnssecPolicy`
  are validated up front so `"`, `;`, `{`, `}` can no longer break out of the
  quoted `rndc addzone` config literals (B-3).
- `src/records.rs`: `validate_record_value` now rejects control characters for
  all record types (replacing the previous "any non-empty string is valid"
  behaviour for TXT/CAA/SRV); added `validate_record_name`. The add/remove/update
  record handlers reject `\n`/`\r`/NUL in record names and values with HTTP 400,
  and the zone name from the URL path is validated via `validate_zone_name`.
- `src/nsupdate.rs`: Added `reject_injection_chars`, a defense-in-depth check at
  the nsupdate sink. `add_record`, `remove_record`, and `update_record` reject
  control characters in the zone, name, and value(s) before assembling the
  newline-delimited nsupdate command script, preventing DNS UPDATE command
  injection (B-2) even if a handler-layer check is ever bypassed.
- Tests: added validation/injection unit tests in `src/zones_test.rs`,
  `src/records_test.rs`, and `src/nsupdate_test.rs` covering traversal,
  newline-injection, and quote-breakout payloads plus legitimate inputs.

#### Why
Any in-cluster pod reaching bindcar could previously traverse out of the zone
directory, inject additional `nsupdate` commands via newlines in record fields,
or break out of `rndc addzone` quoted literals via crafted key/policy names.
These were the two Criticals (B-1, B-2) and the high-severity RNDC injection
(B-3) from the 2026-06-09 security audit.

#### Impact
- [x] Breaking change (malformed zone names / record fields that were previously
      accepted are now rejected with HTTP 400)
- [ ] Requires cluster rollout
- [ ] Config change only
- [ ] Documentation only

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
