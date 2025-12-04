# Environment Variables

Complete reference of all environment variables used by bindcar.

## Core Variables

### BIND_ZONE_DIR

- **Type**: String (path)
- **Default**: `/var/cache/bind`
- **Required**: No
- **Description**: Directory where BIND9 zone files are stored

```bash
BIND_ZONE_DIR=/var/cache/bind
```

Must be:
- Readable and writable by the bindcar user (UID 1000)
- Shared between bindcar and BIND9 containers
- An existing directory

### API_PORT

- **Type**: Integer
- **Default**: `8080`
- **Required**: No
- **Description**: Port for the HTTP API server

```bash
API_PORT=8080
```

Valid range: 1-65535

### RNDC_PATH

- **Type**: String (path)
- **Default**: `/usr/sbin/rndc`
- **Required**: No
- **Description**: Path to the rndc binary

```bash
RNDC_PATH=/usr/sbin/rndc
```

The rndc binary must be:
- Executable by the bindcar user
- A valid BIND9 rndc installation

## Logging Variables

### RUST_LOG

- **Type**: String (log level)
- **Default**: `info`
- **Required**: No
- **Description**: Log level for the application

```bash
RUST_LOG=info
```

Valid values:
- `error` - Only error messages
- `warn` - Warnings and errors
- `info` - Informational messages, warnings, and errors
- `debug` - Debug information
- `trace` - Very detailed trace information

Can also be module-specific:
```bash
RUST_LOG=bindcar=debug,tower_http=info
```

## Security Variables

### DISABLE_AUTH

- **Type**: Boolean
- **Default**: `false`
- **Required**: No
- **Description**: Disable Bearer token authentication

```bash
DISABLE_AUTH=false
```

Valid values:
- `true` - Disable authentication (USE ONLY IN TRUSTED ENVIRONMENTS)
- `false` - Enable authentication (recommended)

**WARNING**: Setting this to `true` disables all authentication. Only use in environments where authentication is handled by infrastructure (Linkerd service mesh, API gateway, etc.).

## Environment Variable Precedence

1. Explicit environment variables (highest priority)
2. Default values (lowest priority)

## Validation

bindcar validates all environment variables on startup:

- Path variables must point to existing, accessible locations
- Port numbers must be valid (1-65535)
- Log levels must be recognized values

Invalid configuration will cause bindcar to exit with an error message.

## Examples

### Development

```bash
export RUST_LOG=debug
export BIND_ZONE_DIR=./zones
export API_PORT=3000
export DISABLE_AUTH=true
```

### Production (Docker)

```bash
docker run -d \
  -e BIND_ZONE_DIR=/var/cache/bind \
  -e API_PORT=8080 \
  -e RUST_LOG=info \
  -e DISABLE_AUTH=false \
  ghcr.io/firestoned/bindcar:latest
```

### Kubernetes

```yaml
env:
- name: BIND_ZONE_DIR
  value: "/var/cache/bind"
- name: API_PORT
  value: "8080"
- name: RUST_LOG
  value: "info"
- name: RNDC_PATH
  value: "/usr/sbin/rndc"
- name: DISABLE_AUTH
  value: "false"
```

## Best Practices

1. **Use defaults where possible** - Only override when necessary
2. **Store secrets securely** - Use Kubernetes Secrets or similar for sensitive values
3. **Use appropriate log levels** - `info` for production, `debug` for troubleshooting
4. **Don't disable authentication** - Unless absolutely necessary and in trusted environments
5. **Validate paths exist** - Ensure directories and binaries exist before starting

## Next Steps

- [Configuration](./configuration.md) - Configuration overview
- [Authentication](./authentication.md) - Authentication setup
- [Deployment](./deployment.md) - Deployment guides
