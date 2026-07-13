# bindcar

## Project Status

[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/bindcar.svg)](https://crates.io/crates/bindcar)
[![GitHub Release](https://img.shields.io/github/v/release/firestoned/bindcar)](https://github.com/firestoned/bindcar/releases/latest)
[![GitHub commits since latest release](https://img.shields.io/github/commits-since/firestoned/bindcar/latest)](https://github.com/firestoned/bindcar/commits/main)
[![Last Commit](https://img.shields.io/github/last-commit/firestoned/bindcar)](https://github.com/firestoned/bindcar/commits/main)

## CI/CD Status

[![Build](https://github.com/firestoned/bindcar/actions/workflows/build.yaml/badge.svg)](https://github.com/firestoned/bindcar/actions/workflows/build.yaml)
[![Documentation](https://github.com/firestoned/bindcar/actions/workflows/docs.yaml/badge.svg)](https://github.com/firestoned/bindcar/actions/workflows/docs.yaml)

## Code Quality

[![codecov](https://codecov.io/gh/firestoned/bindcar/branch/main/graph/badge.svg)](https://codecov.io/gh/firestoned/bindcar)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

## Technology & Compatibility

[![Rust](https://img.shields.io/badge/rust-1.88+-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![BIND9](https://img.shields.io/badge/BIND9-DNS%20Server-blue)](https://www.isc.org/bind/)
[![Linux](https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black)](https://www.linux.org/)
[![Docker](https://img.shields.io/badge/Docker-2496ED?logo=docker&logoColor=white)](https://www.docker.com/)
[![Kubernetes](https://img.shields.io/badge/kubernetes-compatible-326CE5.svg?logo=kubernetes&logoColor=white)](https://kubernetes.io)

## Security & Compliance

[![SPDX](https://img.shields.io/badge/SPDX-License--Identifier-blue)](https://spdx.dev/)
[![SBOM](https://img.shields.io/badge/SBOM-CycloneDX-orange)](https://cyclonedx.org/)
[![Cosign Signed](https://img.shields.io/badge/releases-signed-brightgreen.svg)](https://github.com/firestoned/bindcar/releases)

## Community & Support

[![Issues](https://img.shields.io/github/issues/firestoned/bindcar)](https://github.com/firestoned/bindcar/issues)
[![Pull Requests](https://img.shields.io/github/issues-pr/firestoned/bindcar)](https://github.com/firestoned/bindcar/pulls)
[![Contributors](https://img.shields.io/github/contributors/firestoned/bindcar)](https://github.com/firestoned/bindcar/graphs/contributors)
[![Stars](https://img.shields.io/github/stars/firestoned/bindcar?style=social)](https://github.com/firestoned/bindcar/stargazers)

---

**A lightweight HTTP REST API server for managing BIND9 zones via rndc commands.**

---

## Overview

bindcar runs as a sidecar container alongside BIND9, providing a REST interface for zone management operations. It executes rndc commands locally and manages zone files on a shared volume.

## Features

- **Zone management** via REST API (create, delete, reload, status, modify)
- **Individual DNS record management** (add, update, remove records dynamically via nsupdate)
- **DNSSEC support** with BIND9 9.16+ policy integration and automatic inline signing
- IP-based rate limiting with configurable thresholds (GCRA algorithm)
- Kubernetes ServiceAccount token authentication with optional TokenReview validation
- Fine-grained access control (audience validation, namespace/SA allowlists)
- Health and readiness endpoints
- Prometheus metrics for monitoring
- Structured JSON logging
- Runs as non-root user with minimal permissions

## Quick Start

### Using Docker

```bash
docker run -d \
  -p 8080:8080 \
  -v /var/cache/bind:/var/cache/bind \
  -e RUST_LOG=info \
  ghcr.io/firestoned/bindcar:latest
```

### Using Kubernetes

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: bind9
spec:
  containers:
  - name: bind9
    image: bind9:latest
    volumeMounts:
    - name: zones
      mountPath: /var/cache/bind
  - name: bindcar
    image: ghcr.io/firestoned/bindcar:latest
    ports:
    - containerPort: 8080
    volumeMounts:
    - name: zones
      mountPath: /var/cache/bind
  volumes:
  - name: zones
    emptyDir: {}
```

## Configuration

Environment variables:

- `BIND_ZONE_DIR` - Directory for zone files (default: `/var/cache/bind`)
- `API_PORT` - API server port (default: `8080`)
- `RNDC_SERVER` - RNDC server address (default: `127.0.0.1:953`, or from `/etc/bind/rndc.conf`)
- `RNDC_ALGORITHM` - HMAC algorithm (default: `sha256`, or from `/etc/bind/rndc.conf`)
- `RNDC_SECRET` - Base64-encoded RNDC secret key (required if not using rndc.conf)
- `NSUPDATE_SERVER` - DNS server for nsupdate (default: `127.0.0.1`)
- `NSUPDATE_PORT` - DNS server port for nsupdate (default: `53`)
- `NSUPDATE_KEY_NAME` - TSIG key name for nsupdate (defaults to RNDC key)
- `NSUPDATE_ALGORITHM` - HMAC algorithm for nsupdate (defaults to `RNDC_ALGORITHM`)
- `NSUPDATE_SECRET` - Base64-encoded TSIG secret for nsupdate (defaults to `RNDC_SECRET`)
- `RUST_LOG` - Log level (default: `info`)
- `BIND_API_ADDRESS` - Interface to bind the API to (default: `0.0.0.0`)
- `BIND_API_TOKEN` - Shared secret; when set, the Bearer token must match it (constant-time)
- `DISABLE_AUTH` - Disable authentication (default: `false`)
- `BINDCAR_ALLOW_INSECURE_AUTH` - Override the startup guard for non-loopback weak/disabled auth (default: `false`)
- `RATE_LIMIT_ENABLED` - Enable rate limiting (default: `true`)
- `RATE_LIMIT_REQUESTS` - Max requests per period (default: `100`)
- `RATE_LIMIT_PERIOD_SECS` - Rate limit period in seconds (default: `60`)
- `RATE_LIMIT_BURST` - Burst size for rate limiting (default: `10`)

> **Note:** rate limits are keyed on the real TCP peer IP, **not** the
> `X-Forwarded-For`/`X-Real-IP`/`Forwarded` headers (which a client can forge to
> evade the limit or exhaust another client's bucket). bindcar is reached
> directly by the operator's pods, so it must not run behind an untrusted proxy
> that rewrites the peer address.

### RNDC Configuration

bindcar can be configured in two ways:

**Option 1: Environment Variables**
```bash
export RNDC_SERVER="127.0.0.1:953"
export RNDC_ALGORITHM="sha256"
export RNDC_SECRET="dGVzdC1zZWNyZXQtaGVyZQ=="
```

**Option 2: Using rndc.conf**

If `RNDC_SECRET` is not set, bindcar will automatically parse `/etc/bind/rndc.conf` or `/etc/rndc.conf`:

```conf
# /etc/bind/rndc.conf
key "rndc-key" {
    algorithm hmac-sha256;
    secret "dGVzdC1zZWNyZXQtaGVyZQ==";
};

options {
    default-key "rndc-key";
    default-server 127.0.0.1;
    default-port 953;
};
```

The configuration also supports `include` directives for security-sensitive environments:

```conf
# /etc/bind/rndc.conf
include "/etc/bind/rndc.key";

options {
    default-key "rndc-key";
    default-server 127.0.0.1;
};
```

### Authentication

By default, authentication is **enabled** and requires Bearer token authentication for all API endpoints except `/health` and `/ready`.

**Authentication modes:**

1. **Basic Mode** (presence-only) - Validates token format only. ⚠️ This is *not* real
   authentication and bindcar will **refuse to start** in this mode on a non-loopback
   interface (see *Startup guard* below).
2. **Shared-secret Mode** - Set `BIND_API_TOKEN`; every request's Bearer token must equal it
   (compared in constant time). Real authentication without a Kubernetes API connection —
   suitable for `drone`/bare-metal deployments.
3. **TokenReview Mode** (optional) - Full token validation with Kubernetes TokenReview API.

#### Startup guard (B-4)

bindcar refuses to start when the API is bound to a **non-loopback** interface without real
authentication, so a privileged API is never silently exposed. To start on `0.0.0.0` you must
satisfy one of:

- a real authenticator is configured — `BIND_API_TOKEN` is set, **or** the binary was built
  with the `k8s-token-review` feature; **or**
- the API is bound to loopback (`BIND_API_ADDRESS=127.0.0.1`); **or**
- the operator explicitly accepts the risk via `--i-know-this-is-insecure` (or
  `BINDCAR_ALLOW_INSECURE_AUTH=true`).

**TokenReview Mode** provides enhanced security:
- Validates token signatures
- Checks token expiration
- Validates token audience
- Restricts to specific namespaces/ServiceAccounts

Enable TokenReview mode by building with the `k8s-token-review` feature and configuring environment variables:

```yaml
env:
- name: BIND_TOKEN_AUDIENCES
  value: "bindcar"  # Required audience
- name: BIND_ALLOWED_NAMESPACES
  value: "dns-system"  # Allowed namespaces (empty = all)
- name: BIND_ALLOWED_SERVICE_ACCOUNTS
  value: "system:serviceaccount:dns-system:external-dns"  # Allowed SAs (empty = all)
```

To disable authentication (e.g., when using a service mesh like Linkerd):

```bash
# Docker
docker run -d \
  -p 8080:8080 \
  -e DISABLE_AUTH=true \
  ghcr.io/firestoned/bindcar:latest

# Kubernetes
env:
- name: DISABLE_AUTH
  value: "true"
```

**WARNING**: Disabling authentication should ONLY be done in trusted environments where authentication is handled by infrastructure (Linkerd service mesh, API gateway, etc.). Never disable authentication in production without proper network-level security controls.

Because of the startup guard, `DISABLE_AUTH=true` on a non-loopback bind also requires
`--i-know-this-is-insecure` (or `BINDCAR_ALLOW_INSECURE_AUTH=true`) — an explicit
acknowledgement that you are relying on infrastructure-level controls.

For defense-in-depth, the `deploy/` directory ships:

- [`deploy/networkpolicy.yaml`](deploy/networkpolicy.yaml) — restricts the API (ingress) to the bindy operator.
- [`deploy/rbac.yaml`](deploy/rbac.yaml) — least-privilege RBAC (only `system:auth-delegator` for TokenReview).
- [`deploy/pod-hardening.yaml`](deploy/pod-hardening.yaml) — pod/container `securityContext` reference (drop all caps, read-only rootfs, non-root, no token auto-mount) plus an egress NetworkPolicy.

See [Kubernetes TokenReview Validation](https://firestoned.github.io/bindcar/developer-guide/k8s-token-validation.html) for detailed configuration.

## API Endpoints

### Health & Metrics
- `GET /api/v1/health` - Health check
- `GET /api/v1/ready` - Readiness check
- `GET /metrics` - Prometheus metrics (no auth required)

### Server Status
- `GET /api/v1/server/status` - BIND9 server status

### Zone Management
- `POST /api/v1/zones` - Create zone
- `GET /api/v1/zones` - List zones
- `GET /api/v1/zones/{name}` - Get zone info
- `DELETE /api/v1/zones/{name}` - Delete zone
- `POST /api/v1/zones/{name}/reload` - Reload zone
- `GET /api/v1/zones/{name}/status` - Zone status
- `POST /api/v1/zones/{name}/freeze` - Freeze zone
- `POST /api/v1/zones/{name}/thaw` - Thaw zone
- `POST /api/v1/zones/{name}/notify` - Notify secondaries

### DNS Record Management
- `POST /api/v1/zones/{name}/records` - Add individual record
- `DELETE /api/v1/zones/{name}/records` - Remove individual record
- `PUT /api/v1/zones/{name}/records` - Update individual record

## Documentation

Full documentation is available at: [https://firestoned.github.io/bindcar](https://firestoned.github.io/bindcar)

- [Getting Started](https://firestoned.github.io/bindcar/installation.html)
- [API Reference](https://firestoned.github.io/bindcar/api-reference.html)
- [Managing DNS Records](https://firestoned.github.io/bindcar/user-guide/managing-records.html)
- [Record Endpoints API](https://firestoned.github.io/bindcar/reference/api-records.html)
- [Configuration Guide](https://firestoned.github.io/bindcar/configuration.html)
- [Deployment](https://firestoned.github.io/bindcar/deployment.html)

Or build locally:

```bash
make docs
make docs-serve
```

## Development

```bash
# Build
cargo build --release

# Test
cargo test

# Run locally
RUST_LOG=debug cargo run

# Build docs
make docs
```

## License

MIT - Copyright (c) 2025 Erick Bourgeois, firestoned
