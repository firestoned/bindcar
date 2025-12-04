# Access Control

Access control patterns for bindcar API.

## Network Policies

Restrict network access to bindcar:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: bindcar-policy
spec:
  podSelector:
    matchLabels:
      app: dns
  ingress:
  - from:
    - podSelector:
        matchLabels:
          role: dns-client
    ports:
    - protocol: TCP
      port: 8080
```

## API Gateway Integration

Use an API gateway for additional access control:

- Rate limiting
- IP allowlisting
- Request validation

## RBAC

Kubernetes RBAC for ServiceAccount tokens:

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: bindcar-api-user
rules:
- apiGroups: [""]
  resources: ["serviceaccounts/token"]
  verbs: ["create"]
```

## Next Steps

- [Security](./security.md) - Security overview
- [Kubernetes](../operations/kubernetes.md) - Kubernetes deployment
