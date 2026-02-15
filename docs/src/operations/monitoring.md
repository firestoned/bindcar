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

## Prometheus Metrics

bindcar exports comprehensive Prometheus metrics at `/metrics`:

```bash
curl http://localhost:8080/metrics
```

### Available Metrics

#### HTTP Request Metrics

**`bindcar_http_requests_total`**
- Type: Counter
- Labels: `method`, `path`, `status`
- Description: Total number of HTTP requests processed

**`bindcar_http_request_duration_seconds`**
- Type: Histogram
- Labels: `method`, `path`
- Buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s, 10s
- Description: HTTP request duration in seconds

#### Zone Operation Metrics

**`bindcar_zone_operations_total`**
- Type: Counter
- Labels: `operation`, `result`
- Operations: `create`, `delete`, `reload`, `freeze`, `thaw`, `notify`, `status`
- Results: `success`, `error`
- Description: Total number of zone operations

**`bindcar_zones_managed_total`**
- Type: Gauge
- Description: Current number of zones managed

#### RNDC Command Metrics

**`bindcar_rndc_commands_total`**
- Type: Counter
- Labels: `command`, `result`
- Results: `success`, `error`
- Description: Total number of RNDC commands executed

**`bindcar_rndc_command_duration_seconds`**
- Type: Histogram
- Labels: `command`
- Buckets: 10ms, 50ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s, 10s
- Description: RNDC command execution duration

#### Rate Limiting Metrics

**`bindcar_rate_limit_requests_total`**
- Type: Counter
- Labels: `result`
- Results: `allowed`, `rejected`
- Description: Total number of rate limit checks

#### Application Metrics

**`bindcar_app_info`**
- Type: Counter
- Labels: `version`
- Description: Application version information

### Prometheus Configuration

Add bindcar to your Prometheus scrape configuration:

```yaml
scrape_configs:
  - job_name: 'bindcar'
    static_configs:
      - targets: ['bindcar:8080']
    metrics_path: '/metrics'
```

### Grafana Dashboard Example

Key queries for monitoring:

```promql
# Request rate
rate(bindcar_http_requests_total[5m])

# Request latency (p95)
histogram_quantile(0.95, rate(bindcar_http_request_duration_seconds_bucket[5m]))

# Error rate
sum(rate(bindcar_http_requests_total{status=~"5.."}[5m])) / sum(rate(bindcar_http_requests_total[5m]))

# Zone operations by type
rate(bindcar_zone_operations_total[5m])

# Rate limit rejections
rate(bindcar_rate_limit_requests_total{result="rejected"}[5m])

# RNDC command failures
rate(bindcar_rndc_commands_total{result="error"}[5m])
```

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
- Health endpoint failures (`/api/v1/health` returning non-200)
- 5xx error rates (`bindcar_http_requests_total{status=~"5.."}`)
- RNDC command failures (`bindcar_rndc_commands_total{result="error"}`)
- Authentication failures (401 errors)
- Rate limit rejections (`bindcar_rate_limit_requests_total{result="rejected"}`)
- High request latency (p95 > 500ms)

### Example Prometheus Alerts

```yaml
groups:
  - name: bindcar
    rules:
      - alert: BindcarHighErrorRate
        expr: |
          sum(rate(bindcar_http_requests_total{status=~"5.."}[5m]))
          / sum(rate(bindcar_http_requests_total[5m])) > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate in bindcar"
          description: "Error rate is {{ $value | humanizePercentage }}"

      - alert: BindcarRateLimitHigh
        expr: rate(bindcar_rate_limit_requests_total{result="rejected"}[5m]) > 1
        for: 5m
        labels:
          severity: info
        annotations:
          summary: "High rate limit rejections"
          description: "Rate limiting is rejecting {{ $value }} requests/sec"

      - alert: BindcarRndcFailures
        expr: rate(bindcar_rndc_commands_total{result="error"}[5m]) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "RNDC commands failing"
          description: "RNDC failure rate: {{ $value }}/sec"

      - alert: BindcarHighLatency
        expr: |
          histogram_quantile(0.95,
            rate(bindcar_http_request_duration_seconds_bucket[5m])
          ) > 0.5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High request latency"
          description: "p95 latency is {{ $value }}s"
```

## Next Steps

- [Logging](./logging.md) - Configure logging
- [Troubleshooting](./troubleshooting.md) - Debug issues
