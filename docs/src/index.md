# Introduction

**bindcar** is a lightweight HTTP REST API server for managing BIND9 zones via rndc commands. It provides a modern HTTP interface to BIND9's native control protocol, making DNS zone management simple and scriptable.

### Project Status

[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/bindcar.svg)](https://crates.io/crates/bindcar)
[![GitHub Release](https://img.shields.io/github/v/release/firestoned/bindcar)](https://github.com/firestoned/bindcar/releases/latest)
[![GitHub commits since latest release](https://img.shields.io/github/commits-since/firestoned/bindcar/latest)](https://github.com/firestoned/bindcar/commits/main)
[![Last Commit](https://img.shields.io/github/last-commit/firestoned/bindcar)](https://github.com/firestoned/bindcar/commits/main)

### CI/CD Status

[![Main Branch CI/CD](https://github.com/firestoned/bindcar/actions/workflows/main.yaml/badge.svg)](https://github.com/firestoned/bindcar/actions/workflows/main.yaml)
[![Pull Request Checks](https://github.com/firestoned/bindcar/actions/workflows/pr.yml/badge.svg)](https://github.com/firestoned/bindcar/actions/workflows/pr.yml)
[![Release Workflow](https://github.com/firestoned/bindcar/actions/workflows/release.yml/badge.svg)](https://github.com/firestoned/bindcar/actions/workflows/release.yml)
[![Documentation](https://github.com/firestoned/bindcar/actions/workflows/docs.yaml/badge.svg)](https://github.com/firestoned/bindcar/actions/workflows/docs.yaml)

## What is bindcar?

bindcar runs as a sidecar container alongside BIND9, exposing a REST API for zone management operations. It executes rndc commands locally and manages zone files on a shared volume, providing:

- **HTTP REST API** - Modern interface for BIND9 zone management
- **Zone Lifecycle** - Create, delete, reload zones via HTTP endpoints
- **Zone File Generation** - Create zones from structured JSON configuration
- **Authentication** - Token-based authentication for secure access
- **Structured Logging** - JSON logging with tracing for observability
- **Container-Ready** - Designed to run as a sidecar in Docker/Kubernetes

## Why bindcar?

Traditional BIND9 management requires:
- Direct server access (SSH)
- Manual zone file editing
- Command-line rndc usage
- Complex orchestration

bindcar simplifies this by:
- Providing HTTP API access from anywhere
- Accepting structured zone configurations
- Handling rndc commands automatically
- Enabling easy automation and integration

## Who Should Use bindcar?

bindcar is ideal for:
- **Platform Engineers** building DNS automation
- **DevOps Teams** managing BIND9 in containers
- **System Administrators** seeking scriptable DNS management
- **Kubernetes Operators** needing BIND9 control APIs
- **Anyone** running BIND9 who wants a modern API

## Quick Example

Create a zone with a simple HTTP request:

```bash
curl -X POST http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "zoneName": "example.com",
    "zoneType": "primary",
    "zoneConfig": {
      "ttl": 3600,
      "soa": {
        "primaryNs": "ns1.example.com.",
        "adminEmail": "admin.example.com.",
        "serial": 2025010101,
        "refresh": 3600,
        "retry": 600,
        "expire": 604800,
        "negativeTtl": 86400
      },
      "nameServers": [
        "ns1.example.com.",
        "ns2.example.com."
      ],
      "records": [
        {
          "name": "www",
          "type": "A",
          "value": "192.0.2.1",
          "ttl": 300
        }
      ]
    }
  }'
```

bindcar automatically:
1. Validates the configuration
2. Generates the zone file
3. Executes `rndc addzone`
4. Returns the result

## Key Features

### Zone Management

- **Create zones** - From structured JSON configuration with automatic zone file generation
- **Delete zones** - Remove zones and clean up files
- **Reload zones** - Update zones without restart
- **Zone status** - Check zone information via rndc

### Operations

- **Freeze/Thaw** - Control dynamic updates
- **Notify** - Trigger secondary notifications
- **Server status** - Check BIND9 health

### API Design

- **RESTful** - Standard HTTP verbs and status codes
- **JSON** - Simple, structured request/response format
- **Authenticated** - Bearer token security
- **Validated** - Input validation and error handling

## Architecture

```
┌─────────────┐     HTTP/JSON      ┌──────────┐
│   Client    │ ─────────────────> │ bindcar  │
│             │ <───────────────── │   API    │
└─────────────┘                    └────┬─────┘
                                        │ rndc
                                        │ commands
                                        v
                                   ┌─────────┐
                                   │  BIND9  │
                                   └─────────┘
```

bindcar translates HTTP requests into rndc commands, providing a modern interface to BIND9's native management protocol.

## Performance

- **Startup Time**: <1 second
- **Memory Usage**: ~10-20MB
- **Zone Creation**: <500ms per zone
- **API Latency**: <100ms typical

## Project Status

bindcar is actively developed and production-ready. It follows semantic versioning and maintains API compatibility within major versions.

Current version: **v0.1.0**

## Use Cases

### Container Sidecar

Run bindcar alongside BIND9 in Docker or Kubernetes:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: dns-server
spec:
  containers:
  - name: bind9
    image: ubuntu/bind9:latest
  - name: bindcar
    image: ghcr.io/firestoned/bindcar:latest
    ports:
    - containerPort: 8080
```

### Kubernetes Operator

Use bindcar as the control plane for a BIND9 Kubernetes operator (see [bindy](https://github.com/firestoned/bindy)).

### Automation

Integrate with CI/CD, infrastructure-as-code, or configuration management tools.

## Next Steps

- [Installation](./getting-started/index.md) - Get started with bindcar
- [Quick Start](./getting-started/quickstart.md) - Deploy your first zone
- [API Reference](./reference/api.md) - Complete API documentation
- [Architecture](./getting-started/architecture.md) - Understand how bindcar works

## Support & Community

- **GitHub Issues**: [Report bugs or request features](https://github.com/firestoned/bindcar/issues)
- **Documentation**: You're reading it!

## Related Projects

- **[bindy](https://github.com/firestoned/bindy)** - Kubernetes operator for BIND9 that uses bindcar as its control plane

## License

bindcar is open-source software licensed under the [MIT License](./license.md).
