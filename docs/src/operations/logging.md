# Logging

bindcar uses structured JSON logging for easy parsing and analysis.

## Log Format

All logs are output in JSON format:

```json
{
  "timestamp": "2024-12-03T10:30:45.123Z",
  "level": "info",
  "target": "bindcar::zones",
  "message": "Zone created successfully",
  "zone": "example.com",
  "serial": 2024010101
}
```

## Log Levels

Configure via `RUST_LOG` environment variable.

### error

Only errors:
```bash
RUST_LOG=error
```

### warn

Warnings and errors:
```bash
RUST_LOG=warn
```

### info (Recommended)

Normal operations:
```bash
RUST_LOG=info
```

Example logs:
- Zone created
- Zone deleted
- Zone reloaded
- RNDC commands executed

### debug

Detailed debugging:
```bash
RUST_LOG=debug
```

Includes:
- HTTP request details
- POST/PATCH request payloads (full JSON body)
- RNDC command output
- Token validation (tokens not logged)

### trace

Very verbose:
```bash
RUST_LOG=trace
```

Use only for debugging specific issues.

## Module-Specific Logging

```bash
# bindcar at debug, everything else at info
RUST_LOG=bindcar=debug,tower_http=info

# Only zone operations at debug
RUST_LOG=bindcar::zones=debug
```

## Log Fields

Common fields:

- `timestamp` - ISO 8601 timestamp with timezone
- `level` - Log level (error, warn, info, debug, trace)
- `target` - Module that generated the log
- `message` - Human-readable message
- Context fields (zone, status_code, etc.)

## Examples

### Zone Creation

```json
{
  "timestamp": "2024-12-03T10:30:45Z",
  "level": "info",
  "target": "bindcar::zones",
  "message": "Creating zone",
  "zone": "example.com",
  "zone_type": "primary"
}
```

### POST Payload Debug Logging

When `RUST_LOG=debug` is enabled, POST and PATCH request payloads are logged:

```json
{
  "timestamp": "2024-12-03T10:30:45Z",
  "level": "debug",
  "target": "bindcar::zones",
  "message": "POST /api/v1/zones payload: {\n  \"zoneName\": \"example.com\",\n  \"zoneType\": \"primary\",\n  \"zoneConfig\": {\n    \"ttl\": 3600,\n    \"soa\": {...}\n  }\n}"
}
```

This is useful for:
- Troubleshooting malformed requests
- Debugging serialization issues
- Auditing zone configuration changes

### HTTP Request

```json
{
  "timestamp": "2024-12-03T10:30:45Z",
  "level": "info",
  "target": "tower_http::trace",
  "message": "request completed",
  "method": "POST",
  "uri": "/api/v1/zones",
  "status": 201,
  "duration_ms": 45
}
```

### Error

```json
{
  "timestamp": "2024-12-03T10:30:45Z",
  "level": "error",
  "target": "bindcar::rndc",
  "message": "RNDC command failed",
  "command": "addzone",
  "zone": "example.com",
  "error": "zone already exists"
}
```

## Log Aggregation

### Kubernetes

Use Fluent Bit or Fluentd:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: fluent-bit-config
data:
  parsers.conf: |
    [PARSER]
        Name   json
        Format json
        Time_Key timestamp
        Time_Format %Y-%m-%dT%H:%M:%S.%LZ
```

### Docker

```bash
docker logs bindcar --follow | jq
```

### ELK Stack

Logs are compatible with Elasticsearch/Kibana:

```json
{
  "@timestamp": "2024-12-03T10:30:45Z",
  "level": "info",
  "message": "Zone created",
  "zone": "example.com"
}
```

## Filtering Logs

### jq Examples

```bash
# Show only errors
docker logs bindcar | jq 'select(.level=="error")'

# Show zone operations
docker logs bindcar | jq 'select(.zone != null)'

# Show slow requests (>100ms)
docker logs bindcar | jq 'select(.duration_ms > 100)'
```

### grep Examples

```bash
# Find errors
kubectl logs -l app=dns -c bindcar | grep '"level":"error"'

# Find specific zone
kubectl logs -l app=dns -c bindcar | grep 'example.com'
```

## Best Practices

1. **Use info in production** - Balance detail and volume
2. **Use debug for troubleshooting** - Temporarily when needed
3. **Ship logs to aggregation** - Don't rely on pod logs alone
4. **Set retention policies** - Logs can grow quickly
5. **Monitor for errors** - Alert on error rate increases

## Next Steps

- [Monitoring](./index.md) - Overview of monitoring
- [Troubleshooting](../troubleshooting.md) - Debug issues
