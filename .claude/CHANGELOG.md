# Changelog

## [2026-06-29 23:00] - SHA-pin all GitHub Actions + Dependabot SHA updates

**Author:** Erick Bourgeois

### Changed
- `.github/workflows/{docs.yaml,main.yaml,pr.yml,release.yml}`: pinned every
  external action `uses:` ref to a full commit SHA with a `# <tag>` comment
  (102 refs), including the first-party `firestoned/github-actions/*` composite
  actions. SHAs resolved from the live GitHub API.
- `.github/actions/prepare-docker-binaries/action.yaml`: pinned
  `actions/download-artifact@v4` to its commit SHA.
- `.github/dependabot.yml`: documented that the github-actions ecosystem now
  updates SHA pins (Dependabot bumps the SHA + version comment to the newest
  release) and covers the composite action directory.

### Why
Supply-chain hardening: a mutable tag (`@v4`, `@stable`) can be force-pushed to
point at malicious code, whereas a commit SHA is immutable. Pinning to SHA with
a `# vX` comment lets Dependabot keep them current by bumping the SHA itself.
Exceptions: the SLSA reusable workflow stays on a semver tag (required for
provenance verification), and the local `./.github/actions/...` action needs no
pin.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] CI/config change only
- [ ] Documentation only

## [2026-06-29 22:00] - Fix drone integration test for B-4 startup guard

**Author:** Erick Bourgeois

### Changed
- `integration-test/drone-external-bind9.sh`: added `BINDCAR_ALLOW_INSECURE_AUTH="true"`
  to the bindcar drone launch env block.

### Why
The B-4 startup guard refuses to start when bound to a non-loopback interface
(`0.0.0.0`, the default) without real authentication. The drone integration test
intentionally runs with `DISABLE_AUTH=true` in a trusted local environment, which
is exactly the case the explicit operator override exists for. Without the override
the guard exited before the API bound, so the test's "wait for API on 8080" timed
out. Verified locally: without the var bindcar refuses (logs "refusing to start");
with it the HTTP listener binds and `/api/v1/health` returns healthy.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [ ] Config change only
- [x] Test-only change

## [2026-06-29 21:00] - Bump anyhow to 1.0.103 (RUSTSEC-2026-0190)

**Author:** Erick Bourgeois

### Changed
- `Cargo.lock`: `anyhow` 1.0.102 → 1.0.103.

### Why
`cargo audit --deny warnings` failed in CI on RUSTSEC-2026-0190, an unsoundness
advisory in `anyhow::Error::downcast_mut()` (published 2026-06-25) affecting
1.0.102. 1.0.103 is the patched release. `cargo audit` now passes clean.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] Config change only (lockfile / dependency)
- [ ] Documentation only

## [2026-06-09 13:00] - TSIG key out of argv: nsupdate -k keyfile instead of -y (B-7)

**Author:** Erick Bourgeois

### Changed
- `Cargo.toml`: promoted `tempfile = "3"` from dev-dependency to runtime dependency
  (well-maintained, std-adjacent crate; creates 0600 temp files on Unix).
- `src/nsupdate.rs`: replaced `-y algorithm:keyname:secret` with `-k <keyfile>`.
  Added `create_tsig_key_file` (0600 `NamedTempFile`, removed immediately after
  nsupdate exits), `build_tsig_key_file_content` (renders the BIND key file with
  strict validation: key name allowlist, algorithm normalized + checked against
  the known HMAC list, secret restricted to base64 charset), and
  `build_nsupdate_args` (pure, testable argv builder).
- `src/nsupdate_test.rs`: `tsig_keyfile_tests` (8 tests) — argv never contains the
  secret, `-y` is gone, keyfile is 0600, content/normalization correct, file
  removed after use, malicious key-file fields rejected.

### Why
B-7 (2026-06-09 audit): the TSIG secret was passed on the nsupdate command line,
making it world-readable via `/proc/<pid>/cmdline` and process listings to any
process sharing the pod/host.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [ ] Documentation only

## [2026-06-09 12:00] - Real authentication by default: shared secret + startup guard + NetworkPolicy (B-4)

**Author:** Erick Bourgeois

### Changed
- `Cargo.toml`: added `subtle = "2.6"` (constant-time comparison).
- `src/auth.rs`: shared-secret auth layer (`BIND_API_TOKEN`) with constant-time
  `compare_shared_secret`/`validate_shared_secret`; added `is_loopback_host`,
  `has_real_auth`, and the pure `check_startup_auth_posture` guard decision. The
  `authenticate` middleware enforces the shared secret before optional TokenReview.
- `src/cli.rs`: added global `--i-know-this-is-insecure` flag.
- `src/main.rs`: startup guard refusing to run presence-only/disabled auth on a
  non-loopback bind unless overridden; configurable `BIND_API_ADDRESS`.
- `deploy/networkpolicy.yaml`: restrict the API (TCP 8080) to the bindy operator.
- `src/auth_test.rs`: `b4_auth_posture_tests` (loopback detection, constant-time
  match/reject, all guard branches).

### Why
B-4 (2026-06-09 audit): presence-only auth accepted any non-empty token and
`DISABLE_AUTH` only warned, silently exposing the privileged API to any in-cluster pod.

### Impact
- [x] Breaking change
- [ ] API change
- [ ] Config change only
- [ ] Documentation only

## [2026-06-09 11:00] - Input validation: path traversal + nsupdate/RNDC injection (B-1, B-2, B-3)

**Author:** Erick Bourgeois

### Changed
- `src/zones.rs`: `validate_zone_name` (strict DNS grammar — blocks path traversal,
  B-1) and `validate_rndc_identifier` (allowlist — blocks RNDC quote-breakout, B-3);
  wired into `create_zone`/`delete_zone`.
- `src/records.rs`: control-character rejection in `validate_record_value` (incl.
  TXT/CAA/SRV) + new `validate_record_name`; zone name validated in
  `validate_zone_for_updates`.
- `src/nsupdate.rs`: `reject_injection_chars` defense-in-depth at the nsupdate sink
  (zone/name/value) — blocks newline command injection (B-2).
- Tests in `src/zones_test.rs`, `src/records_test.rs`, `src/nsupdate_test.rs`.

### Why
B-1/B-2 (Criticals) + B-3 from the 2026-06-09 audit: unsanitized request fields
reached filesystem paths, the nsupdate command stream, and rndc config literals.

### Impact
- [x] Breaking change (malformed names/fields now rejected with HTTP 400)
- [ ] API change
- [ ] Config change only
- [ ] Documentation only

## [2026-03-29 21:00] - Move BYOB9 roadmap to forage project

**Author:** Erick Bourgeois

### Changed
- Moved `docs/roadmaps/byob9-bring-your-own-bind9.md` → `~/dev/forage/docs/roadmaps/byob9-bring-your-own-bind9.md`

### Why
The roadmap describes the forage binary, not bindcar. It belongs in the forage repository.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [x] Documentation only

## [2026-03-29 20:00] - Fix zone directory permissions for BIND9 container

**Author:** Erick Bourgeois

### Changed
- `integration-test/drone-external-bind9.sh`: Added `chmod 777 "$ZONE_DIR"` after `mkdir -p` so the `bind` user inside the container can write to the zone directory

### Why
On the CI runner the zone directory is created by the `runner` user. The BIND9 container runs `named` as user `bind` (uid 101), which has no write access to the host-owned directory. named aborts with `permission denied` when it can't write to its configured `directory`.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [x] CI/CD only

## [2026-03-29 19:00] - Fix drone integration test binary path

**Author:** Erick Bourgeois

### Changed
- `.github/workflows/pr.yml`: Fixed `chmod` and `BINDCAR_BIN` paths in drone integration test — artifact upload preserves the full relative path `target/x86_64-unknown-linux-gnu/release/bindcar`, so after downloading to `/tmp/bindcar-bin` the binary is at `/tmp/bindcar-bin/target/x86_64-unknown-linux-gnu/release/bindcar`, not `/tmp/bindcar-bin/bindcar`

### Why
`chmod: cannot access '/tmp/bindcar-bin/bindcar': No such file or directory` — `upload-artifact` preserves the workspace-relative path structure inside the artifact zip, so `download-artifact` mirrors that structure under the destination directory.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [x] CI/CD only

## [2026-03-29 18:00] - Fix CI test and drone integration test failures

**Author:** Erick Bourgeois

### Changed
- `.github/workflows/pr.yml`: Updated `K8S_OPENAPI_ENABLED_VERSION` from `"1.31"` to `"1.32"` to match the merged `k8s-openapi = "0.27"` Cargo.toml dependency that enables `v1_32` feature
- `.github/workflows/pr.yml`: Added `apt-get update` before `apt-get install` in drone integration test job to prevent 404 mirror failures
- `.github/workflows/release.yml`: Updated `K8S_OPENAPI_ENABLED_VERSION` from `"1.31"` to `"1.32"` to match

### Why
Merging `main` into `out-of-cluster` brought in `kube 3.0` and `k8s-openapi 0.27` with `v1_32` feature. The CI env var `K8S_OPENAPI_ENABLED_VERSION: "1.31"` enabled `v1_31` while Cargo.toml enabled `v1_32` — k8s-openapi panics when both are active. The drone test apt install failed because the runner's package mirror had a stale/missing entry for `bind9-utils`.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [x] CI/CD only

## [2026-03-26 00:02] - Drone integration test in PR CI

**Author:** Erick Bourgeois

### Changed
- `integration-test/drone-external-bind9.sh`: `BINDCAR_BIN` is now overridable via env var (`${BINDCAR_BIN:-${REPO_ROOT}/target/debug/bindcar}`) so CI can point to a pre-built artifact
- `Makefile`: Added `drone-integration-test-ci` target — runs the integration test script directly without `cargo build` (binary supplied via `BINDCAR_BIN`)
- `.github/workflows/pr.yml`: Added `drone-integration-test` job — runs after `build`, downloads `bindcar-linux-amd64` artifact, installs `dnsutils` for `dig`, then calls `make drone-integration-test-ci`

### Why
Wire the drone integration test into the PR gate so every PR is automatically validated end-to-end: BIND9 starts in Docker, bindcar drone manages it, zone and A record are created, and DNS resolution is confirmed with dig.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] New CI gate only

## [2026-03-27 00:00] - External BIND9 example config and drone integration test

**Author:** Erick Bourgeois

### Added
- `examples/external-bind9/setup.sh`: Bare-metal/VM setup script — generates TSIG key, copies config files to `/etc/bind/`, sets permissions, validates with `named-checkconf`
- `examples/external-bind9/integration-test.sh`: End-to-end integration test — starts BIND9 in Docker, starts `bindcar drone`, creates zone `foo.bar`, adds A record `test.foo.bar → 1.2.3.4`, verifies resolution with `dig`
- `Makefile`: Added `drone-integration-test` target

### Why
The `examples/external-bind9/` config files needed both a production setup path (setup.sh)
and an automated test that proves the full drone-mode flow works: BIND9 accepting rndc
addzone, nsupdate for record creation, and actual DNS resolution. This is the key integration
test for Phase 1 of the standalone out-of-cluster roadmap.

### Design notes
- Zone dir is mounted at the **same absolute path** (`/tmp/bindcar-drone-test/zones`) in
  both host and Docker container so `rndc addzone file <path>` resolves correctly from named's
  perspective without Phase 2's `BIND_ZONE_DIR_REMOTE`.
- BIND9 controls and `allow-update` are opened to `any` in the test config because Docker NAT
  changes the source IP from named's perspective (not 127.0.0.1).
- `openssl rand -base64 32` generates the TSIG key — no dependency on BIND9 tools.
- `DISABLE_AUTH=true` — this test validates zone/record management, not Kubernetes auth.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] New test infrastructure only

## [2026-03-26 00:01] - CLI subcommands: `run` (sidecar) and `drone` (standalone)

**Author:** Erick Bourgeois

### Changed
- `Cargo.toml`: Added `clap = { version = "4", features = ["derive"] }` dependency
- `src/cli.rs`: NEW — `Cli` struct and `Commands` enum (`Run`, `Drone`) with `resolved_command()` defaulting to `Run`
- `src/cli_test.rs`: NEW — 8 TDD tests for CLI parsing (subcommand parsing, default resolution, unknown subcommand error, help text)
- `src/lib.rs`: Added `pub mod cli` and `#[cfg(test)] mod cli_test`
- `src/main.rs`: Extracted `init_tracing()` and `start_server(&Commands)` from monolithic `main()`. `main()` now parses CLI and dispatches. `start_server` logs the mode in the startup banner ("sidecar mode" or "drone mode - standalone")

### Why
Users need a clear CLI contract for the two operating modes (sidecar vs standalone drone).
`bindcar` with no args continues to behave identically to `bindcar run` for backwards
compatibility with existing process supervisors and Kubernetes entrypoints. The `drone`
subcommand establishes the entry point for all future Phase 2–4 standalone features
(SSH zone transport, multi-instance config, systemd packaging).

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] Default behavior unchanged (`bindcar` still starts the sidecar server)
- [ ] Documentation only

## [2026-03-26 00:00] - Phase 1: Out-of-Cluster Kubernetes Authentication (TDD)

**Author:** Erick Bourgeois

### Changed
- `src/auth.rs`: Added `KubeAuthMode` enum, `detect_kube_auth_mode()`, `build_explicit_kube_client()`, and `build_kube_client()` under `#[cfg(feature = "k8s-token-review")]`
- `src/auth.rs`: `validate_token_with_k8s()` now calls `build_kube_client()` instead of `Client::try_default()` directly
- `src/auth_test.rs`: Added 11 new TDD tests covering `KubeAuthMode` detection (7 tests) and `build_explicit_kube_client` file error handling (3 tests), all written RED-first
- `src/main.rs`: Startup log now shows the resolved Kubernetes auth mode when `k8s-token-review` feature is compiled in

### Why
Phase 1 of the standalone out-of-cluster roadmap (`docs/roadmaps/standalone-out-of-cluster.md`). Enables bindcar to run on bare-metal/VM hosts that have no kubeconfig file but do have a token file and CA cert (e.g., obtained via projected volumes or secrets copied to the host). The explicit env-var path (`KUBE_API_SERVER` + `KUBE_TOKEN_PATH` + `KUBE_CA_CERT_PATH`) mirrors the HashiCorp Vault Kubernetes auth pattern.

### New Environment Variables (under `k8s-token-review` feature)
| Variable | Description |
|----------|-------------|
| `KUBE_API_SERVER` | Kubernetes API server URL (e.g., `https://api.prod.example.com:6443`) |
| `KUBE_TOKEN_PATH` | Path to a ServiceAccount token file |
| `KUBE_CA_CERT_PATH` | Path to the cluster CA certificate (PEM) |

All three must be set together. Partial configuration logs a warning and falls back to `try_default()`.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [ ] Config change only
- [x] New optional feature — zero impact on existing deployments; `KUBE_*` vars are not set in existing environments so `try_default()` path is unchanged
