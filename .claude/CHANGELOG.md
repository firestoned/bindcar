# Changelog

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
