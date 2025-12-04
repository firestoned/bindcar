# Authentication & Authorization

Detailed authentication and authorization configuration.

## Bearer Token Authentication

Configure static tokens in `BIND_ALLOWED_TOKENS`:

```yaml
env:
- name: BIND_ALLOWED_TOKENS
  valueFrom:
    secretKeyRef:
      name: bindcar-tokens
      key: tokens
```

## Kubernetes ServiceAccount Tokens

Use Kubernetes ServiceAccount tokens for dynamic authentication:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: bindcar-client
```

## Token Management

- Token rotation
- Expiration policies
- Revocation strategies

## Next Steps

- [Configuration](../operations/configuration.md) - Configuration options
- [Authentication](../operations/authentication.md) - Authentication setup
