# Prerequisites

Before installing and running bindcar, ensure your system meets the following requirements.

## Runtime Requirements

### For Docker Deployment

- Docker 20.10 or later
- Docker Compose 2.0 or later (optional, for multi-container setups)
- Access to BIND9 container or host

### For Kubernetes Deployment

- Kubernetes 1.24 or later
- kubectl configured with cluster access
- BIND9 running in the same pod as bindcar (sidecar pattern)

### For Standalone Deployment

- Linux operating system (Ubuntu 20.04+, Debian 11+, RHEL 8+, or Alpine 3.20+)
- BIND9 installed and running
- `rndc` binary available at `/usr/sbin/rndc` (or custom path)
- Access to BIND9 zone directory (default: `/var/cache/bind`)

## Network Requirements

- Port 8080 available for the API server (configurable via `API_PORT`)
- Network access to BIND9 rndc port 953 (if using remote rndc)
- Outbound HTTPS access for pulling Docker images (if using Docker)

## BIND9 Configuration

bindcar requires BIND9 to be configured to accept rndc commands:

```bash
# Check rndc is working
rndc status

# Expected output: server is up and running
```

If rndc is not configured, you'll need to set up the rndc key. See [RNDC Integration](./rndc-integration.md) for details.

## Build Requirements

If building from source, you'll need:

- Rust 1.87.0 or later
- Cargo (included with Rust)
- Git
- C compiler (gcc or clang) for some dependencies

Install Rust using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Storage Requirements

- Minimal disk space: ~50MB for the bindcar binary
- Additional space for zone files (varies based on number and size of zones)
- Shared volume between bindcar and BIND9 (when running in containers)

## Permissions

bindcar runs as a non-root user (UID 1000) and requires:

- Read/write access to the zone directory
- Execute permission for the rndc binary
- Network permissions to bind to the API port

## Optional Requirements

### For Authentication

- Bearer tokens (Kubernetes ServiceAccount tokens or custom tokens)
- Can be disabled via `DISABLE_AUTH=true` for Linkerd service mesh environments

### For Monitoring

- Prometheus for metrics collection (future feature)
- Log aggregation system for structured JSON logs

### For Development

- mdBook for building documentation
- Docker BuildX for multi-arch builds
- GitHub CLI (`gh`) for pull request operations

## Verification

Verify your system meets the requirements:

```bash
# Check Docker version
docker --version

# Check Kubernetes access
kubectl version

# Check BIND9 and rndc
rndc status

# Check Rust version (if building from source)
rustc --version
```

## Next Steps

- [Installation](./installation.md) - Install bindcar
- [Quick Start](./quickstart.md) - Get started quickly
- [Configuration](./configuration.md) - Configure your environment
