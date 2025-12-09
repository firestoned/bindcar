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

### RNDC_SERVER

- **Type**: String (address:port)
- **Default**: `127.0.0.1:953` (or from `/etc/bind/rndc.conf`)
- **Required**: No
- **Description**: RNDC server address and port

```bash
RNDC_SERVER=127.0.0.1:953
```

### RNDC_ALGORITHM

- **Type**: String
- **Default**: `sha256` (or from `/etc/bind/rndc.conf`)
- **Required**: No
- **Description**: HMAC algorithm for RNDC authentication

```bash
RNDC_ALGORITHM=sha256
```

Valid values:
- `md5` (or `hmac-md5`)
- `sha1` (or `hmac-sha1`)
- `sha224` (or `hmac-sha224`)
- `sha256` (or `hmac-sha256`)
- `sha384` (or `hmac-sha384`)
- `sha512` (or `hmac-sha512`)

Both formats (with or without `hmac-` prefix) are accepted.

### RNDC_SECRET

- **Type**: String (base64-encoded)
- **Default**: None (read from `/etc/bind/rndc.conf` or `/etc/rndc.conf`)
- **Required**: Only if not using rndc.conf
- **Description**: Base64-encoded RNDC secret key

```bash
RNDC_SECRET=dGVzdC1zZWNyZXQtaGVyZQ==
```

**Note**: If `RNDC_SECRET` is not set, bindcar will automatically parse the RNDC configuration from `/etc/bind/rndc.conf` or `/etc/rndc.conf`, including any `include` directives for separate key files.

The secret must be:
- Base64-encoded
- Match the key configured in BIND9's `rndc.conf`

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

### BIND_TOKEN_AUDIENCES

- **Type**: String (comma-separated)
- **Default**: `bindcar`
- **Required**: No (only when `k8s-token-review` feature enabled)
- **Description**: Expected token audiences for TokenReview validation

```bash
BIND_TOKEN_AUDIENCES=bindcar,https://bindcar.dns-system.svc.cluster.local
```

Prevents token reuse across different services. Tokens must be created with matching audience:
```bash
kubectl create token my-app --audience=bindcar
```

### BIND_ALLOWED_NAMESPACES

- **Type**: String (comma-separated)
- **Default**: None (allow all)
- **Required**: No (only when `k8s-token-review` feature enabled)
- **Description**: Allowed Kubernetes namespaces for token authentication

```bash
BIND_ALLOWED_NAMESPACES=dns-system,kube-system
```

Empty value allows all namespaces (default). When set, only tokens from specified namespaces are accepted.

### BIND_ALLOWED_SERVICE_ACCOUNTS

- **Type**: String (comma-separated)
- **Default**: None (allow all)
- **Required**: No (only when `k8s-token-review` feature enabled)
- **Description**: Allowed ServiceAccounts for token authentication

```bash
BIND_ALLOWED_SERVICE_ACCOUNTS=system:serviceaccount:dns-system:external-dns,system:serviceaccount:dns-system:cert-manager
```

Format: `system:serviceaccount:<namespace>:<name>`

Empty value allows all ServiceAccounts (default). When set, only specified ServiceAccounts are accepted.

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
  -e RNDC_SERVER=127.0.0.1:953 \
  -e RNDC_ALGORITHM=sha256 \
  -e RNDC_SECRET=dGVzdC1zZWNyZXQtaGVyZQ== \
  -e DISABLE_AUTH=false \
  ghcr.io/firestoned/bindcar:latest
```

### Kubernetes (Basic Auth)

```yaml
env:
- name: BIND_ZONE_DIR
  value: "/var/cache/bind"
- name: API_PORT
  value: "8080"
- name: RUST_LOG
  value: "info"
- name: RNDC_SERVER
  value: "127.0.0.1:953"
- name: RNDC_ALGORITHM
  value: "sha256"
- name: RNDC_SECRET
  valueFrom:
    secretKeyRef:
      name: rndc-secret
      key: secret
- name: DISABLE_AUTH
  value: "false"
```

### Kubernetes (TokenReview Mode - Production)

```yaml
env:
- name: BIND_ZONE_DIR
  value: "/var/cache/bind"
- name: API_PORT
  value: "8080"
- name: RUST_LOG
  value: "info"
- name: RNDC_SERVER
  value: "127.0.0.1:953"
- name: RNDC_ALGORITHM
  value: "sha256"
- name: RNDC_SECRET
  valueFrom:
    secretKeyRef:
      name: rndc-secret
      key: secret
- name: DISABLE_AUTH
  value: "false"
# TokenReview security configuration
- name: BIND_TOKEN_AUDIENCES
  value: "bindcar,https://bindcar.dns-system.svc.cluster.local"
- name: BIND_ALLOWED_NAMESPACES
  value: "dns-system"
- name: BIND_ALLOWED_SERVICE_ACCOUNTS
  value: "system:serviceaccount:dns-system:external-dns"
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
