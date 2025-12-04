# Integration

Integration patterns with other systems.

## Overview

bindcar can integrate with various DNS and Kubernetes ecosystem tools.

## External DNS

Integrate with external-dns for automatic Kubernetes service discovery.

## Service Discovery

Use bindcar as a DNS backend for service discovery:

- Kubernetes service registration
- Consul integration
- etcd-based discovery

## CI/CD Integration

Automate zone management in CI/CD pipelines:

```bash
# In CI/CD pipeline
curl -X POST $BINDCAR_URL/api/v1/zones \
  -H "Authorization: Bearer $CI_TOKEN" \
  -d @zone-config.json
```

## Next Steps

- [External DNS](./external-dns.md) - External DNS integration
