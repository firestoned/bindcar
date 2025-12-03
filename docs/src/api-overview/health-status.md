# Health & Status Endpoints

bindcar provides health check and status endpoints for monitoring and orchestration.

## Health Check

**GET** `/api/v1/health`

Simple health check endpoint that returns the service health status.

### No Authentication Required

This endpoint does not require authentication.

### Request

```bash
curl http://localhost:8080/api/v1/health
```

### Response

```json
{
  "healthy": true
}
```

### Status Codes

- `200 OK` - Service is healthy
- `503 Service Unavailable` - Service is unhealthy

### Use Cases

- Kubernetes liveness probes
- Load balancer health checks
- Monitoring systems
- Simple uptime checks

## Readiness Check

**GET** `/api/v1/ready`

Readiness check that verifies the service is ready to accept traffic.

### No Authentication Required

This endpoint does not require authentication.

### Request

```bash
curl http://localhost:8080/api/v1/ready
```

### Response

```json
{
  "ready": true,
  "checks": {
    "zone_directory": "ok",
    "rndc_binary": "ok"
  }
}
```

### Status Codes

- `200 OK` - Service is ready
- `503 Service Unavailable` - Service is not ready

### Readiness Checks

The service performs these checks:
1. Zone directory is accessible and writable
2. RNDC binary is executable
3. Service has started successfully

### Use Cases

- Kubernetes readiness probes
- Rolling deployments
- Traffic routing decisions

## Server Status

**GET** `/api/v1/server/status`

Get BIND9 server status via rndc.

### Authentication Required

Requires Bearer token authentication.

### Request

```bash
curl http://localhost:8080/api/v1/server/status \
  -H "Authorization: Bearer $TOKEN"
```

### Response

```json
{
  "status": "version: 9.18.10\nCPUs found: 4\nworker threads: 4\nUDP listeners per interface: 4\nnumber of zones: 12 (0 automatic)\ndebug level: 0\nxfers running: 0\nxfers deferred: 0\nsoa queries in progress: 0\nquery logging is OFF\nrecursive clients: 0/900/1000\ntcp clients: 0/150/150\nTCP high-water: 0\nserver is up and running"
}
```

### Status Codes

- `200 OK` - Status retrieved successfully
- `401 Unauthorized` - Missing or invalid token
- `502 Bad Gateway` - RNDC command failed

### Use Cases

- Monitoring BIND9 server status
- Checking number of zones
- Verifying BIND9 is running
- Performance monitoring

## Kubernetes Integration

### Liveness Probe

```yaml
livenessProbe:
  httpGet:
    path: /api/v1/health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
  timeoutSeconds: 3
  failureThreshold: 3
```

### Readiness Probe

```yaml
readinessProbe:
  httpGet:
    path: /api/v1/ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
  timeoutSeconds: 3
  successThreshold: 1
  failureThreshold: 3
```

### Startup Probe

```yaml
startupProbe:
  httpGet:
    path: /api/v1/ready
    port: 8080
  initialDelaySeconds: 0
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 30
```

## Monitoring Examples

### Simple Health Check Script

```bash
#!/bin/bash
if curl -f -s http://localhost:8080/api/v1/health > /dev/null; then
  echo "Service is healthy"
  exit 0
else
  echo "Service is unhealthy"
  exit 1
fi
```

### Prometheus Monitoring

While bindcar doesn't currently export Prometheus metrics, you can use blackbox_exporter to monitor the health endpoint.

## Next Steps

- [Zone Operations](./zone-operations.md) - Zone management endpoints
- [API Reference](../api-reference/index.md) - Complete API documentation
- [Monitoring](../monitoring/index.md) - Monitoring and observability
