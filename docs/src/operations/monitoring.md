# Monitoring

Monitor bindcar and BIND9 health, performance, and operations.

## Monitoring Endpoints

### Health Check

```bash
curl http://localhost:8080/api/v1/health
```

Use for:
- Liveness probes
- Basic uptime monitoring
- Load balancer health checks

### Readiness Check

```bash
curl http://localhost:8080/api/v1/ready
```

Use for:
- Readiness probes
- Deployment readiness
- Traffic routing decisions

### Server Status

```bash
curl http://localhost:8080/api/v1/server/status \
  -H "Authorization: Bearer $TOKEN"
```

Returns BIND9 server statistics.

## Logging

See [Logging](./logging.md) for detailed logging configuration.

### Log Levels

- `error` - Errors only
- `warn` - Warnings and errors
- `info` - Normal operations (recommended)
- `debug` - Detailed debugging
- `trace` - Very verbose

### Structured Logging

All logs are JSON format:

```json
{
  "timestamp": "2024-12-03T10:30:45Z",
  "level": "info",
  "message": "Zone created successfully",
  "zone": "example.com"
}
```

## Metrics

Currently, bindcar does not export Prometheus metrics. Future versions may include:

- Request count
- Request duration
- Zone count
- Error rates

## Kubernetes Monitoring

### Probes

```yaml
livenessProbe:
  httpGet:
    path: /api/v1/health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /api/v1/ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
```

### Log Collection

Use a DaemonSet like Fluent Bit or Fluentd to collect JSON logs.

## Alerting

Monitor for:
- Health endpoint failures
- 5xx error rates
- RNDC command failures (502 errors)
- Authentication failures (401 errors)

## Next Steps

- [Logging](./logging.md) - Configure logging
- [Troubleshooting](../troubleshooting.md) - Debug issues
