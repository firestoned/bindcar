# BIND9 RNDC API

A lightweight HTTP REST API server for managing BIND9 zones via rndc commands.

## Overview

This API server runs as a sidecar container alongside BIND9, providing a REST interface for zone management operations. It executes rndc commands locally and manages zone files on a shared volume.

## Features

- **Zone Management**: Create, delete, reload zones via HTTP API
- **Authentication**: ServiceAccount token-based authentication
- **Health Checks**: Built-in health and readiness endpoints
- **Logging**: Structured JSON logging with tracing
- **Security**: Runs as non-root user, validates all inputs

## API Endpoints

### Health & Status

- `GET /api/v1/health` - Health check
- `GET /api/v1/ready` - Readiness check
- `GET /api/v1/server/status` - BIND9 server status

### Zone Operations

- `POST /api/v1/zones` - Create a new zone
- `DELETE /api/v1/zones/:name` - Delete a zone
- `POST /api/v1/zones/:name/reload` - Reload a zone
- `GET /api/v1/zones/:name/status` - Get zone status
- `POST /api/v1/zones/:name/freeze` - Freeze zone (disable updates)
- `POST /api/v1/zones/:name/thaw` - Thaw zone (enable updates)
- `POST /api/v1/zones/:name/notify` - Notify secondaries

## API Reference

### Create Zone

**POST** `/api/v1/zones`

Creates a new zone from structured configuration, generates the zone file, and executes `rndc addzone`.

**Request Body:**
```json
{
  "zoneName": "example.com",
  "zoneType": "master",
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
      },
      {
        "name": "@",
        "type": "MX",
        "value": "mail.example.com.",
        "priority": 10
      },
      {
        "name": "txt-record",
        "type": "TXT",
        "value": "\"v=spf1 mx ~all\""
      }
    ]
  },
  "updateKeyName": "bindy-operator"
}
```

**Zone Configuration Fields:**
- `ttl` (required): Default TTL for the zone in seconds
- `soa` (required): SOA record configuration
  - `primaryNs`: Primary nameserver (must end with `.`)
  - `adminEmail`: Admin email (must end with `.`)
  - `serial`: Zone serial number
  - `refresh`: Refresh interval (default: 3600)
  - `retry`: Retry interval (default: 600)
  - `expire`: Expire time (default: 604800)
  - `negativeTtl`: Negative caching TTL (default: 86400)
- `nameServers` (required): List of nameservers for the zone
- `records` (optional): DNS records to include in the zone
  - `name`: Record name (e.g., `"www"`, `"@"`)
  - `type`: Record type (e.g., `"A"`, `"AAAA"`, `"MX"`, `"TXT"`)
  - `value`: Record value
  - `ttl` (optional): Record-specific TTL
  - `priority` (optional): Priority for MX/SRV records

**Response:**
```json
{
  "success": true,
  "message": "Zone example.com created successfully",
  "details": "zone example.com/IN: loaded serial 2025010101"
}
```

### Delete Zone

**DELETE** `/api/v1/zones/:name`

Deletes a zone using `rndc delzone` and removes the zone file.

**Response:**
```json
{
  "success": true,
  "message": "Zone example.com deleted successfully"
}
```

### Get Zone Status

**GET** `/api/v1/zones/:name/status`

Retrieves zone status using `rndc zonestatus`.

**Response:**
```json
{
  "success": true,
  "message": "Zone example.com status retrieved",
  "details": "zone status output"
}
```

## Authentication

All API endpoints (except `/health` and `/ready`) require authentication via Bearer token:

```bash
curl -H "Authorization: Bearer <token>" http://localhost:8080/api/v1/zones/example.com/status
```

In Kubernetes, use the ServiceAccount token:

```bash
TOKEN=$(cat /var/run/secrets/kubernetes.io/serviceaccount/token)
curl -H "Authorization: Bearer $TOKEN" http://bind9-rndc-api:8080/api/v1/zones/example.com/status
```

## Configuration

Environment variables:

- `BIND_ZONE_DIR` - Directory for zone files (default: `/var/cache/bind`)
- `API_PORT` - API server port (default: `8080`)
- `RUST_LOG` - Log level (default: `info`)

## Development

### Build

```bash
cargo build --release
```

### Test

```bash
cargo test
```

### Run Locally

```bash
RUST_LOG=debug cargo run
```

### Build Docker Image

```bash
docker build -t bind9-rndc-api:latest .
```

## Security

- Runs as non-root user (`bind9-api`, UID 1000)
- Requires authentication for all zone operations
- Validates all input parameters
- Read-only root filesystem (when configured)
- Minimal container image based on Debian slim

## License

MIT - Copyright (c) 2025 Erick Bourgeois, firestoned
