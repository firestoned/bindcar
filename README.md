# bindcar

[![codecov](https://codecov.io/gh/firestoned/bindcar/branch/main/graph/badge.svg)](https://codecov.io/gh/firestoned/bindcar)
[![Build Status](https://github.com/firestoned/bindcar/workflows/CI/badge.svg)](https://github.com/firestoned/bindcar/actions)

A lightweight HTTP REST API server for managing BIND9 zones via rndc commands.

## Overview

bindcar runs as a sidecar container alongside BIND9, providing a REST interface for zone management operations. It executes rndc commands locally and manages zone files on a shared volume.

## Features

- Zone management via REST API (create, delete, reload, status)
- ServiceAccount token authentication
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
- `RUST_LOG` - Log level (default: `info`)
- `DISABLE_AUTH` - Disable authentication (default: `false`)

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

To disable authentication (e.g., when using a service mesh like Linkerd for authentication):

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

## API Endpoints

- `GET /api/v1/health` - Health check
- `GET /api/v1/ready` - Readiness check
- `GET /metrics` - Prometheus metrics (no auth required)
- `GET /api/v1/server/status` - BIND9 server status
- `POST /api/v1/zones` - Create zone
- `GET /api/v1/zones` - List zones
- `GET /api/v1/zones/{name}` - Get zone info
- `DELETE /api/v1/zones/{name}` - Delete zone
- `POST /api/v1/zones/{name}/reload` - Reload zone
- `GET /api/v1/zones/{name}/status` - Zone status
- `POST /api/v1/zones/{name}/freeze` - Freeze zone
- `POST /api/v1/zones/{name}/thaw` - Thaw zone
- `POST /api/v1/zones/{name}/notify` - Notify secondaries

## Documentation

Full documentation is available at: [https://firestoned.github.io/bindcar](https://firestoned.github.io/bindcar)

- [Getting Started](https://firestoned.github.io/bindcar/installation.html)
- [API Reference](https://firestoned.github.io/bindcar/api-reference.html)
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
