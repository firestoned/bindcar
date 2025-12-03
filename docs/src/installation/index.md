# Installation

bindcar can be installed and run in several ways depending on your deployment needs.

## Docker (Recommended)

The easiest way to run bindcar is using the pre-built Docker image:

```bash
docker pull ghcr.io/firestoned/bindcar:latest
```

See the [Docker deployment guide](./docker.md) for detailed setup instructions.

## Kubernetes

For production deployments, bindcar is designed to run as a sidecar container alongside BIND9 in Kubernetes:

```bash
kubectl apply -f k8s/deployment.yaml
```

See the [Kubernetes deployment guide](./kubernetes.md) for complete configuration examples.

## Building from Source

If you need to build bindcar from source:

### Prerequisites

- Rust 1.87.0 or later
- Git

### Build Steps

```bash
# Clone the repository
git clone https://github.com/firestoned/bindcar.git
cd bindcar

# Build the release binary
cargo build --release

# The binary will be at target/release/bindcar
./target/release/bindcar
```

See [Building from Source](./building.md) for more detailed build instructions.

## Verifying Installation

After installation, verify bindcar is running:

```bash
# Check health endpoint
curl http://localhost:8080/api/v1/health

# Expected response:
{"healthy":true}
```

## Next Steps

- [Prerequisites](./prerequisites.md) - System requirements
- [Quick Start](./quickstart.md) - Get up and running quickly
- [Configuration](./configuration.md) - Configure bindcar for your environment
