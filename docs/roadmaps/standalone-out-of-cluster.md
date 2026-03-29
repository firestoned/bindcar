# Standalone bindcar: Out-of-Cluster BIND9 Management

**Status:** Planning
**Created:** 2026-03-16
**Target:** Allow bindcar to run outside Kubernetes alongside existing BIND9 instances

## Overview

bindcar currently runs exclusively as a Kubernetes sidecar. This roadmap enables bindcar to run as a standalone process on any host — bare metal, VM, or Docker — while still accepting Kubernetes ServiceAccount JWTs from callers and validating them via the Kubernetes TokenReview API. This allows users to manage pre-existing BIND9 infrastructure using Kubernetes-native identity without migrating BIND9 into the cluster.

This pattern is analogous to how HashiCorp Vault's Kubernetes auth method works: Vault runs outside the cluster, a client presents their Kubernetes ServiceAccount JWT, and Vault calls back to the Kubernetes API server's TokenReview endpoint (using its own credentials) to validate the token.

```
┌──────────────────────────────────────────────────────────────────────┐
│                      Kubernetes Cluster                              │
│                                                                      │
│   ┌──────────────┐          ┌─────────────────────────────────────┐ │
│   │  API Client  │          │           Kubernetes API            │ │
│   │  (workload)  │          │        (TokenReview endpoint)       │ │
│   └──────┬───────┘          └──────────────▲────────────────────  │ │
│          │ Bearer Token (SA JWT)            │ TokenReview call     │ │
└──────────┼──────────────────────────────────┼──────────────────────┘
           │                                  │ (bindcar's own SA token
           │ HTTPS                            │  or kubeconfig creds)
           ▼                                  │
┌──────────────────────────────────────────────────────────────────────┐
│                      External Host (VM / bare metal)                 │
│                                                                      │
│   ┌─────────────────────────────────────────┐   ┌────────────────┐  │
│   │          bindcar (standalone)           │   │                │  │
│   │                                         │   │    BIND9       │  │
│   │  ┌────────────────┐ ┌────────────────┐  │   │  (named)       │  │
│   │  │ Auth Middleware│ │  Rate Limiter  │  │   │                │  │
│   │  └───────┬────────┘ └────────────────┘  │   │  :53 (DNS)     │  │
│   │          │                              │   │  :953 (RNDC)   │  │
│   │  ┌───────▼────────────────────────────┐ │   │                │  │
│   │  │      Zone & Record Handlers        │ │   └───────▲────────┘  │
│   │  └───────┬──────────────┬─────────────┘ │           │           │
│   │          │              │               │           │           │
│   │  ┌───────▼──────┐ ┌────▼──────────────┐│  RNDC :953│           │
│   │  │RndcExecutor  │ │ ZoneFileTransport  ├┼───────────┘           │
│   │  │(remote :953) │ │ (local or SSH/SFTP)│ │   zone files         │
│   │  └──────────────┘ └────────────────────┘ │                      │
│   └─────────────────────────────────────────┘                       │
└──────────────────────────────────────────────────────────────────────┘
```

## Current State

### What Already Works Out-of-Cluster Today

- **RNDC remote connectivity** — `RNDC_SERVER` accepts any `host:port`, not just `127.0.0.1:953`
- **nsupdate remote** — `NSUPDATE_SERVER` and `NSUPDATE_PORT` already support remote BIND9
- **Kubernetes client fallback** — `kube::Client::try_default()` in `src/auth.rs` already checks `KUBECONFIG` env and `~/.kube/config` before trying in-cluster, so the TokenReview path works if a valid kubeconfig is present

### What Needs to Change

| Area | Problem | Solution |
|------|---------|----------|
| **K8s auth** | No env-var-based client config for headless hosts | Add `KUBE_API_SERVER` / `KUBE_TOKEN_PATH` / `KUBE_CA_CERT_PATH` support |
| **Zone files** | `src/zones.rs` uses `tokio::fs::*` directly (local only) | `ZoneFileTransport` trait + SSH/SFTP implementation |
| **AppState** | Single BIND9 instance assumed | Instance registry for multi-instance support |
| **Deployment** | Only documented as Kubernetes sidecar | systemd unit, standalone Dockerfile |

---

## Phase 1: Out-of-Cluster Kubernetes Authentication

**Status:** Planning
**Effort:** 3–5 days
**Dependencies:** None

### Context

`kube::Client::try_default()` already handles kubeconfig-based auth. The gap is explicit env-var configuration for headless environments (CI runners, VMs, containers) that have neither a kubeconfig file nor access to the standard filesystem paths, but do have a token file and CA certificate (e.g., obtained via Kubernetes projected volumes or secrets copied to the host).

### Changes Required

#### `src/auth.rs`

Replace the inline `Client::try_default()` call with a new helper `build_kube_client()`, gated behind the `k8s-token-review` feature flag. The helper resolves the Kubernetes client configuration in priority order:

1. **Explicit env vars** — when `KUBE_API_SERVER`, `KUBE_TOKEN_PATH`, and `KUBE_CA_CERT_PATH` are all present, construct a `kube::Config` explicitly. This is the recommended path for standalone deployments.
2. **KUBECONFIG env** — delegate to `Client::try_default()` (already works)
3. **~/.kube/config** — delegate to `Client::try_default()` (already works)
4. **In-cluster service account** — delegate to `Client::try_default()` (already works)

```rust
// New helper (inside #[cfg(feature = "k8s-token-review")] block)
async fn build_kube_client() -> Result<Client, String> {
    let api_server = env::var("KUBE_API_SERVER").ok();
    let token_path = env::var("KUBE_TOKEN_PATH").ok();
    let ca_cert_path = env::var("KUBE_CA_CERT_PATH").ok();

    match (api_server, token_path, ca_cert_path) {
        (Some(server), Some(token_path), Some(ca_path)) => {
            // Build explicit config from files
            build_explicit_kube_client(server, token_path, ca_path).await
        }
        _ => {
            // Fall through to try_default() (handles KUBECONFIG, ~/.kube/config, in-cluster)
            Client::try_default().await.map_err(|e| e.to_string())
        }
    }
}
```

#### `src/main.rs`

Log the Kubernetes client resolution mode at startup when `k8s-token-review` is compiled in:

```
INFO bindcar: Kubernetes auth mode: explicit (KUBE_API_SERVER=https://api.prod.example.com:6443)
INFO bindcar: Kubernetes auth mode: kubeconfig (/home/bindcar/.kube/config)
INFO bindcar: Kubernetes auth mode: in-cluster
```

### New Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `KUBE_API_SERVER` | Kubernetes API server URL | — |
| `KUBE_TOKEN_PATH` | Path to a ServiceAccount token file | — |
| `KUBE_CA_CERT_PATH` | Path to the cluster CA certificate (PEM) | — |

All three must be set together; if only some are present, bindcar falls through to `try_default()` and logs a warning.

### RBAC Requirements

bindcar's own service account (or user in kubeconfig) needs permission to create `tokenreviews`. The minimum ClusterRole:

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: bindcar-token-validator
rules:
- apiGroups: ["authentication.k8s.io"]
  resources: ["tokenreviews"]
  verbs: ["create"]
```

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: bindcar-token-validator
subjects:
- kind: User
  name: bindcar           # or ServiceAccount name if using SA token
  apiGroup: rbac.authorization.k8s.io
roleRef:
  kind: ClusterRole
  name: bindcar-token-validator
  apiGroup: rbac.authorization.k8s.io
```

> **Note:** Binding to the built-in `system:auth-delegator` ClusterRole is equivalent but grants broader impersonation rights. The purpose-specific role above is preferred for least-privilege.

### Backward Compatibility

- `try_default()` is only replaced when all three `KUBE_*` vars are present
- Existing in-cluster sidecar deployments continue working without any configuration change
- Existing kubeconfig-based setups continue working unchanged

### Success Criteria

- [ ] bindcar with `k8s-token-review` starts on a bare-metal host using only `KUBE_API_SERVER`, `KUBE_TOKEN_PATH`, `KUBE_CA_CERT_PATH`
- [ ] Valid Kubernetes ServiceAccount JWTs from the cluster are accepted
- [ ] Expired or tampered JWTs are rejected with `401 Unauthorized`
- [ ] Existing in-cluster sidecar deployments pass all existing tests unchanged
- [ ] Startup logs clearly indicate which auth resolution path was used

---

## Phase 2: Remote BIND9 Zone File Management

**Status:** Planning
**Effort:** 2–3 weeks
**Dependencies:** None (can run in parallel with Phase 1)

### Context

`src/zones.rs` calls `tokio::fs::write`, `tokio::fs::remove_file`, and `tokio::fs::read_dir` directly against local disk. For BIND9 instances on separate hosts, a transport abstraction is needed. The abstraction must not change the behavior of the existing local-path mode (shared volumes, NFS, same-host deployments all continue working as-is).

### Design: `ZoneFileTransport` Trait

New module `src/zone_transport.rs`:

```rust
#[async_trait]
pub trait ZoneFileTransport: Send + Sync {
    /// Write a zone file. `zone_name` is the bare zone name (e.g. "example.com").
    async fn write_zone(&self, zone_name: &str, content: &str) -> Result<(), ApiError>;
    /// Remove a zone file.
    async fn remove_zone(&self, zone_name: &str) -> Result<(), ApiError>;
    /// Remove the journal file (.jnl) for a zone, if present.
    async fn remove_zone_journal(&self, zone_name: &str) -> Result<(), ApiError>;
    /// List zone names managed in the zone directory.
    async fn list_zones(&self) -> Result<Vec<String>, ApiError>;
    /// Check whether a zone file exists.
    async fn zone_exists(&self, zone_name: &str) -> Result<bool, ApiError>;
    /// Return the on-BIND9-host path for a zone file (used in rndc addzone config).
    fn remote_zone_path(&self, zone_name: &str) -> String;
}
```

**`LocalZoneTransport`** — wraps existing `tokio::fs::*` calls exactly. Zero behavioral change. The `remote_zone_path()` returns `{zone_dir}/{zone_name}.db`.

**`SshZoneTransport`** (feature `ssh-zone-transport`) — uses the `russh` + `russh-sftp` crates (pure Rust, async-native) for SFTP file operations. `remote_zone_path()` returns `{bind_zone_dir_remote}/{zone_name}.db` which may differ from the local staging path.

### `AppState` Changes

`src/types.rs`:

```rust
pub struct AppState {
    pub rndc: Arc<RndcExecutor>,
    pub nsupdate: Arc<NsupdateExecutor>,
    pub zone_dir: String,                           // retained for local transport
    pub zone_transport: Arc<dyn ZoneFileTransport>, // new
}
```

### `src/zones.rs` Changes

Replace all `tokio::fs::*` call sites with `state.zone_transport.*` calls. The `rndc addzone` config string's `file` directive must use `state.zone_transport.remote_zone_path(&zone_name)` rather than constructing a local path. This is the key distinction enabling remote deployments: bindcar writes via SFTP to the remote host, but tells RNDC the zone's path from BIND9's perspective.

### New Feature Flag

```toml
# Cargo.toml
[features]
ssh-zone-transport = ["dep:russh", "dep:russh-sftp"]
```

### New Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ZONE_TRANSPORT` | `local` or `ssh` | `local` |
| `SSH_HOST` | Remote BIND9 host for SFTP zone writes | — |
| `SSH_PORT` | SFTP port | `22` |
| `SSH_USER` | SSH username on remote host | — |
| `SSH_KEY_PATH` | Path to SSH private key (Ed25519 recommended) | — |
| `SSH_KNOWN_HOSTS_PATH` | Known hosts file for host key verification | — |
| `BIND_ZONE_DIR_REMOTE` | Zone directory path as BIND9 sees it on the remote host | value of `BIND_ZONE_DIR` |

### Security Requirements

- SSH host key verification must default to strict (equivalent to `StrictHostKeyChecking=yes`); `SSH_KNOWN_HOSTS_PATH` must point to a pre-populated file
- SSH private key file must be mode `0600`; bindcar should verify this at startup and refuse to start if not
- The SFTP user on the BIND9 host should be a dedicated non-root account with write access only to the zone directory

### `src/main.rs` Startup Logic

```rust
let zone_transport: Arc<dyn ZoneFileTransport> = match std::env::var("ZONE_TRANSPORT")
    .unwrap_or_else(|_| "local".to_string())
    .as_str()
{
    "ssh" => {
        #[cfg(not(feature = "ssh-zone-transport"))]
        {
            error!("ZONE_TRANSPORT=ssh but ssh-zone-transport feature not compiled in");
            return Err(anyhow::anyhow!("ssh transport unavailable"));
        }
        #[cfg(feature = "ssh-zone-transport")]
        Arc::new(SshZoneTransport::from_env().context("failed to initialize SSH zone transport")?)
    }
    _ => Arc::new(LocalZoneTransport::new(zone_dir.clone())),
};
```

SSH connectivity is verified at startup (before the HTTP server starts) and reflected in the `/api/v1/ready` endpoint.

### Backward Compatibility

- `ZONE_TRANSPORT` defaults to `local`; existing behavior is completely unchanged
- `LocalZoneTransport` is a direct refactor of existing inline calls — no observable difference

### Success Criteria

- [ ] `ZONE_TRANSPORT=ssh` successfully writes, reads, and deletes zone files on the remote host via SFTP
- [ ] `ZONE_TRANSPORT=local` behavior is byte-for-byte identical to current behavior (all existing tests pass)
- [ ] SSH key with incorrect permissions (non-0600) causes startup failure with a clear error
- [ ] Unreachable SSH host causes `GET /api/v1/ready` to return `503`
- [ ] `BIND_ZONE_DIR_REMOTE` correctly decouples the SFTP write path from the rndc zone file path

---

## Phase 3: Multi-Instance BIND9 Support

**Status:** Planning
**Effort:** 2–3 weeks
**Dependencies:** Phase 2 (requires `ZoneFileTransport` trait)

### Context

Today `AppState` holds exactly one RNDC executor, one nsupdate executor, and one zone transport. Organizations with multiple BIND9 instances (e.g., primary + secondary, or per-datacenter instances) currently need to run a separate bindcar process per instance. A named-instance model eliminates that operational overhead.

### Design: Instance Registry

New module `src/instance.rs`:

```rust
pub struct Bind9Instance {
    pub name: String,
    pub rndc: Arc<RndcExecutor>,
    pub nsupdate: Arc<NsupdateExecutor>,
    pub zone_transport: Arc<dyn ZoneFileTransport>,
}
```

Updated `AppState`:

```rust
pub struct AppState {
    pub instances: Arc<HashMap<String, Bind9Instance>>,
    pub default_instance: String,
}
```

### Configuration File Format (TOML)

Activated via `BINDCAR_INSTANCES_CONFIG` env var pointing to a TOML file:

```toml
[defaults]
instance = "primary"

[instances.primary]
rndc_server = "192.168.1.10:953"
rndc_algorithm = "sha256"
rndc_secret = "base64secret=="
zone_transport = "local"
zone_dir = "/var/cache/bind"

[instances.secondary]
rndc_server = "192.168.1.11:953"
rndc_algorithm = "sha256"
rndc_secret = "base64secret=="
zone_transport = "ssh"
zone_dir = "/tmp/bindcar-zones"        # local staging path
zone_dir_remote = "/var/cache/bind"    # path on BIND9 host
ssh_host = "192.168.1.11"
ssh_user = "bindcar"
ssh_key_path = "/etc/bindcar/id_ed25519"
ssh_known_hosts_path = "/etc/bindcar/known_hosts"
```

When `BINDCAR_INSTANCES_CONFIG` is absent, bindcar constructs a single-instance registry from the existing individual env vars — no breaking change.

### API Route Changes

New routes (additive only):

```
GET  /api/v1/instances                          # list configured instances + health
GET  /api/v1/instances/{name}/zones             # list zones on named instance
POST /api/v1/instances/{name}/zones             # create zone on named instance
GET  /api/v1/instances/{name}/zones/{zone}      # get zone on named instance
# ... all existing zone and record routes mirrored under /instances/{name}/
```

Existing un-prefixed routes (`/api/v1/zones`, etc.) continue to delegate to `default_instance`.

### Per-Instance Auth Scoping

Optional: restrict which Kubernetes ServiceAccounts may access which BIND9 instances via `BIND_INSTANCE_ALLOWLIST`:

```
BIND_INSTANCE_ALLOWLIST=system:serviceaccount:team-a:dns-sa=primary,system:serviceaccount:team-b:dns-sa=secondary
```

When unset, authenticated users may access all instances (existing behavior).

### New Feature Flag

```toml
[features]
multi-instance = []
```

### Backward Compatibility

- Single-instance mode (default) when `BINDCAR_INSTANCES_CONFIG` is absent
- All existing API routes unchanged; un-prefixed routes delegate to `default_instance`
- The `multi-instance` feature flag keeps instance registry code out of the default binary

### Success Criteria

- [ ] Two BIND9 instances configurable in a single bindcar process
- [ ] `GET /api/v1/instances` returns instance names and per-instance health
- [ ] Zone created on `instances/primary` is not visible on `instances/secondary`
- [ ] Un-prefixed routes (`/api/v1/zones`) continue working for existing clients
- [ ] Config file parse errors at startup exit with code 1 and a descriptive message
- [ ] `BIND_INSTANCE_ALLOWLIST` correctly restricts access per ServiceAccount

---

## Phase 4: New Deployment Modes

**Status:** Planning
**Effort:** 1–2 weeks
**Dependencies:** None (can run in parallel with Phases 2–3)

### Systemd Service Unit

New file `packaging/bindcar.service`:

```ini
[Unit]
Description=bindcar BIND9 REST API
After=network-online.target named.service
Wants=network-online.target
Documentation=https://github.com/firestoned/bindcar

[Service]
Type=simple
User=bindcar
Group=bindcar
EnvironmentFile=-/etc/bindcar/bindcar.env
ExecStart=/usr/local/bin/bindcar
Restart=on-failure
RestartSec=5s
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/var/cache/bind
AmbientCapabilities=

[Install]
WantedBy=multi-user.target
```

The `EnvironmentFile` at `/etc/bindcar/bindcar.env` contains the same env vars documented today — no changes to `src/main.rs`.

New file `packaging/install.sh`:
- Creates `bindcar` system user (no login shell, no home directory in /home)
- Copies binary to `/usr/local/bin/bindcar`
- Creates `/etc/bindcar/` with mode `0750` owned by `bindcar:bindcar`
- Installs unit file and runs `systemctl daemon-reload`

### Standalone Docker Mode

New `Dockerfile.standalone` distinct from any sidecar-oriented Dockerfile:

Key differences:
- Does not assume shared volume for zone files
- `ZONE_TRANSPORT=ssh` as the recommended out-of-cluster mode, documented in comments
- Exposes `KUBE_API_SERVER`, `KUBE_TOKEN_PATH`, `KUBE_CA_CERT_PATH` as documented entry points

New `docker-compose.standalone.yml` showing bindcar as a standalone container connecting to BIND9 on the Docker host (not in Docker), using SSH transport.

### Success Criteria

- [ ] `systemctl start bindcar` on Ubuntu 22.04 starts the process successfully
- [ ] `systemctl status bindcar` reports active/running
- [ ] `GET /api/v1/health` returns 200 within 5 seconds of service start
- [ ] Service restarts cleanly after `kill -9` (Restart=on-failure)
- [ ] Standalone Docker image passes `/api/v1/health` with `ZONE_TRANSPORT=ssh`

---

## Phase 5: Documentation

**Status:** Planning
**Effort:** 1 week
**Dependencies:** Phases 1–4 (docs are written as each phase ships)

### New Documentation Files

**`docs/src/operations/standalone.md`**
End-to-end guide for running bindcar outside Kubernetes:
- Architecture diagram (same host, SSH, NFS variants)
- Step-by-step for each sub-mode
- Full env var reference for standalone mode

**`docs/src/advanced/k8s-auth-external.md`**
Kubernetes auth from outside the cluster:
- How `kube::Client` resolution order works
- Creating and binding `bindcar-token-validator` ClusterRole
- Token file vs kubeconfig approaches
- Network requirements (API server must be reachable from bindcar host)
- Comparison to HashiCorp Vault Kubernetes auth for operator familiarity

**`docs/src/operations/systemd.md`**
Systemd deployment guide:
- Installation steps
- `bindcar.env` configuration reference
- Log inspection (`journalctl -u bindcar`)
- Enabling at boot

**`docs/src/operations/multi-instance.md`**
Multi-instance configuration:
- TOML config file format
- Per-instance auth scoping
- Instance health endpoint

**`docs/src/operations/migrating-from-sidecar.md`**
Migration guide:
- Decision tree: when to stay with sidecar vs move to standalone
- Step-by-step: extract from sidecar pod, configure out-of-cluster auth, configure zone transport
- Rollback procedure

### `mkdocs.yml` Updates

Add new pages to the navigation under `Operations` and `Advanced` sections.

### Success Criteria

- [ ] All new pages render without errors in `mkdocs serve`
- [ ] Every new env var from Phases 1–4 appears in `docs/src/operations/env-vars.md`
- [ ] No dead internal links
- [ ] Migration guide validated by a test run against a real BIND9 instance

---

## Dependency Graph and Sequencing

```
Phase 1 (K8s auth) ──────────────────────────────────► ship independently
                                                        docs: k8s-auth-external.md

Phase 2 (ZoneTransport) ──► Phase 3 (multi-instance)
                                                        docs: standalone.md
                                                             multi-instance.md
                                                             migrating-from-sidecar.md

Phase 4 (packaging) ─── independent, parallel with 2/3
                                                        docs: systemd.md, docker.md
```

---

## New Dependencies Summary

| Phase | Dependency | Crate | Feature Flag | Notes |
|-------|-----------|-------|--------------|-------|
| 1 | None | — | — | `kube` already in tree |
| 2 | SFTP client | `russh` + `russh-sftp` | `ssh-zone-transport` | Pure Rust, async, MIT |
| 3 | Config parsing | `toml` | `multi-instance` | Widely used, minimal |
| 4 | None | — | — | Packaging only |
| 5 | None | — | — | Documentation only |

Default build (`cargo build` with no features) compiles with zero new dependencies.

---

## Backward Compatibility Guarantees

Every phase ships with these invariants:

1. **Default build unchanged** — `cargo build` produces the same binary as today; all new functionality is opt-in via feature flags
2. **Existing env vars unchanged** — no existing env var is renamed or removed
3. **Existing sidecar deployments unchanged** — no configuration changes required for users running bindcar as a Kubernetes sidecar
4. **Existing API routes unchanged** — all new routes are additive; un-prefixed routes continue to work

---

## Related Files

| File | Description |
|------|-------------|
| [`src/auth.rs`](../../src/auth.rs) | TokenReview auth middleware — Phase 1 target |
| [`src/types.rs`](../../src/types.rs) | `AppState` definition — Phase 2 and 3 target |
| [`src/zones.rs`](../../src/zones.rs) | Zone handlers with inline `tokio::fs::*` calls — Phase 2 target |
| [`src/main.rs`](../../src/main.rs) | Startup config resolution and router — Phases 1–3 target |
| [`Cargo.toml`](../../Cargo.toml) | Feature flags and dependencies — Phases 2–3 target |
| [`docs/src/developer-guide/k8s-token-validation.md`](../src/developer-guide/k8s-token-validation.md) | Existing TokenReview documentation — update in Phase 5 |
| [`docs/src/operations/env-vars.md`](../src/operations/env-vars.md) | Env var reference — update in Phase 5 |
