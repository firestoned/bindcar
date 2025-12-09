# Authentication

bindcar uses Bearer token authentication to secure API endpoints.

## Overview

By default, authentication is **enabled** for all API endpoints except:
- `/api/v1/health`
- `/api/v1/ready`
- `/metrics`

All other endpoints require a valid Bearer token in the Authorization header.

## Authentication Modes

bindcar supports two authentication modes:

### Basic Mode (Default)

- Validates token presence and format only
- Does NOT verify token signatures
- Does NOT check expiration
- Suitable for trusted environments or when using external auth (API gateway, Linkerd service mesh)

### TokenReview Mode (Optional)

- Full token validation with Kubernetes TokenReview API
- Verifies token signatures
- Checks token expiration
- Validates token audience
- Restricts to allowed namespaces and ServiceAccounts
- **Recommended for production Kubernetes deployments**

Enable TokenReview mode by building with the `k8s-token-review` feature. See [Kubernetes TokenReview Validation](../developer-guide/k8s-token-validation.md) for detailed configuration.

## Bearer Token Authentication

### How It Works

1. Client obtains a token (e.g., Kubernetes ServiceAccount token)
2. Client includes token in the `Authorization` header
3. bindcar validates the token format
4. Request is processed if token is valid

### Token Format

```http
Authorization: Bearer <token>
```

Example:
```bash
curl http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer eyJhbGciOiJSUzI1NiIsImtpZCI6..."
```

## Kubernetes ServiceAccount Tokens

In Kubernetes environments, use ServiceAccount tokens for authentication.

### Creating a ServiceAccount

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: bindcar-client
  namespace: default
```

### Using the Token

Get the token:
```bash
# Kubernetes 1.24+
kubectl create token bindcar-client

# Or from a mounted secret
kubectl get secret bindcar-client-token -o jsonpath='{.data.token}' | base64 -d
```

Use with bindcar:
```bash
TOKEN=$(kubectl create token bindcar-client)

curl http://bindcar-service:8080/api/v1/zones \
  -H "Authorization: Bearer $TOKEN"
```

### Complete Kubernetes Example

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: dns-api-client
---
apiVersion: v1
kind: Pod
metadata:
  name: dns-client
spec:
  serviceAccountName: dns-api-client
  containers:
  - name: client
    image: curlimages/curl:latest
    command:
    - sh
    - -c
    - |
      TOKEN=$(cat /var/run/secrets/kubernetes.io/serviceaccount/token)
      curl -H "Authorization: Bearer $TOKEN" \
        http://bindcar-service:8080/api/v1/zones
```

## Custom Bearer Tokens

For non-Kubernetes environments, you can use any bearer token:

```bash
# Generate a random token
TOKEN=$(openssl rand -base64 32)

# Use with bindcar
curl http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer $TOKEN"
```

**Note**: In **Basic Mode**, bindcar validates token format but does not verify token signatures. Token verification should be handled by infrastructure (API gateway, Linkerd service mesh, etc.). For production environments requiring token signature verification, use **TokenReview Mode**.

## Disabling Authentication

For trusted environments where authentication is handled by infrastructure:

### Docker

```bash
docker run -d \
  -p 8080:8080 \
  -e DISABLE_AUTH=true \
  ghcr.io/firestoned/bindcar:latest
```

### Kubernetes

```yaml
env:
- name: DISABLE_AUTH
  value: "true"
```

**WARNING**: Only disable authentication when:
- Running behind an authenticating API gateway
- Using Linkerd service mesh with mTLS and authorization policies
- Running in a completely trusted network
- For local development only

## Authentication Errors

### 401 Unauthorized

Missing or invalid authorization header:

```bash
curl http://localhost:8080/api/v1/zones
```

Response:
```json
{
  "error": "Unauthorized",
  "message": "Missing Authorization header"
}
```

### Invalid Token Format

```bash
curl http://localhost:8080/api/v1/zones \
  -H "Authorization: InvalidFormat"
```

Response:
```json
{
  "error": "Unauthorized",
  "message": "Invalid Authorization header format. Expected: Bearer <token>"
}
```

### Empty Token

```bash
curl http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer "
```

Response:
```json
{
  "error": "Unauthorized",
  "message": "Empty token"
}
```

## Linkerd Service Mesh Integration

When using Linkerd, authentication can be handled at the mesh level:

```yaml
apiVersion: v1
kind: Pod
metadata:
  annotations:
    linkerd.io/inject: enabled
spec:
  containers:
  - name: bindcar
    image: ghcr.io/firestoned/bindcar:latest
    env:
    - name: DISABLE_AUTH
      value: "true"  # Linkerd handles auth
```

## Best Practices

1. **Always enable authentication in production** - Unless using Linkerd service mesh
2. **Rotate tokens regularly** - Especially for long-lived tokens
3. **Use short-lived tokens** - Kubernetes ServiceAccount tokens are ideal
4. **Limit token scope** - Use Kubernetes RBAC to limit what tokens can do
5. **Monitor authentication failures** - Watch for 401 errors in logs
6. **Use HTTPS in production** - Protect tokens in transit

## Security Considerations

**Basic Mode**:
- Validates token format but not signatures
- Token verification should be done by:
  - API gateway or Linkerd service mesh
  - External authentication service
- Suitable for trusted environments only

**TokenReview Mode**:
- Full token validation with Kubernetes API
- Recommended for production environments
- Provides defense-in-depth security
- Can restrict access by namespace and ServiceAccount

**General**:
- Tokens are logged at `debug` level - use `info` in production
- Always use HTTPS in production to protect tokens
- Rotate tokens regularly
- Use short-lived tokens when possible

## Next Steps

- [Kubernetes TokenReview Validation](../developer-guide/k8s-token-validation.md) - Enhanced security setup
- [Configuration](./configuration.md) - Configure authentication settings
- [Environment Variables](./env-vars.md) - TokenReview environment variables
- [Deployment](./deployment.md) - Deploy with authentication
- [Troubleshooting](./troubleshooting.md) - Debug authentication issues
