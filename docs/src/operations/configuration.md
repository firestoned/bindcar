# Configuration

bindcar is configured through environment variables, making it easy to deploy in containerized environments.

## Configuration Overview

All configuration is done via environment variables. No configuration files are needed.

## Environment Variables

See [Environment Variables](./env-vars.md) for a complete reference of all available environment variables.

## Quick Configuration Examples

### Development

```bash
export RUST_LOG=debug
export BIND_ZONE_DIR=./zones
export API_PORT=8080
export DISABLE_AUTH=true

./bindcar
```

### Production (Docker)

```bash
docker run -d \
  -p 8080:8080 \
  -v /var/cache/bind:/var/cache/bind \
  -e RUST_LOG=info \
  -e BIND_ZONE_DIR=/var/cache/bind \
  -e API_PORT=8080 \
  -e DISABLE_AUTH=false \
  ghcr.io/firestoned/bindcar:latest
```

### Kubernetes

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: dns-server
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
    env:
    - name: BIND_ZONE_DIR
      value: "/var/cache/bind"
    - name: API_PORT
      value: "8080"
    - name: RUST_LOG
      value: "info"
    - name: DISABLE_AUTH
      value: "false"
    volumeMounts:
    - name: zones
      mountPath: /var/cache/bind

  volumes:
  - name: zones
    emptyDir: {}
```

## Core Settings

### Zone Directory

```bash
BIND_ZONE_DIR=/var/cache/bind
```

Directory where BIND9 zone files are stored. Must be shared between bindcar and BIND9.

### API Port

```bash
API_PORT=8080
```

Port for the HTTP API server to listen on.

### RNDC Configuration

bindcar uses the native RNDC protocol to communicate with BIND9. Configuration can be provided via environment variables or automatically parsed from rndc.conf:

**Option 1: Environment Variables**

```bash
RNDC_SERVER=127.0.0.1:953
RNDC_ALGORITHM=sha256
RNDC_SECRET=dGVzdC1zZWNyZXQtaGVyZQ==
```

**Option 2: Using rndc.conf (automatic)**

If `RNDC_SECRET` is not set, bindcar automatically parses `/etc/bind/rndc.conf` or `/etc/rndc.conf`.

The configuration file can include separate key files using `include` directives:

```conf
# /etc/bind/rndc.conf
include "/etc/bind/rndc.key";

options {
    default-key "rndc-key";
    default-server 127.0.0.1;
};
```

See [RNDC Integration](../developer-guide/rndc-integration.md) for more details.

## Logging Configuration

### Log Level

```bash
RUST_LOG=info
```

Valid levels:
- `error` - Only errors
- `warn` - Warnings and errors
- `info` - Info, warnings, and errors (recommended)
- `debug` - Detailed debugging information
- `trace` - Very verbose tracing

### Structured Logging

Logs are output in JSON format for easy parsing by log aggregation systems:

```json
{"timestamp":"2025-12-03T10:30:45Z","level":"info","message":"Zone created successfully","zone":"example.com"}
```

## Authentication Configuration

See [Authentication](./authentication.md) for detailed authentication configuration.

### Disable Authentication

```bash
DISABLE_AUTH=true
```

**WARNING**: Only use this in trusted environments where authentication is handled by infrastructure (Linkerd service mesh, API gateway, etc.).

## Configuration Best Practices

1. **Use secrets management** - Store sensitive values in Kubernetes Secrets or similar
2. **Enable authentication** - Always use authentication in production
3. **Set appropriate log levels** - Use `info` in production, `debug` for troubleshooting
4. **Use shared volumes** - Ensure bindcar and BIND9 can access the same zone directory
5. **Configure health checks** - Use `/health` and `/ready` endpoints for liveness and readiness probes

## Configuration Validation

bindcar validates configuration on startup and will fail fast if misconfigured:

```bash
# Example error if zone directory doesn't exist
Error: Zone directory /var/cache/bind does not exist or is not writable
```

## Next Steps

- [Environment Variables](./env-vars.md) - Complete environment variable reference
- [Authentication](./authentication.md) - Authentication configuration
- [Deployment](./deployment.md) - Deployment guides
