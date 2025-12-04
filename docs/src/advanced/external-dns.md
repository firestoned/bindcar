# External DNS Integration

Integration with external-dns for automatic Kubernetes DNS management.

## Overview

While bindcar provides direct API control, you can also integrate with external-dns for automatic service discovery.

## Architecture

```
Kubernetes Services/Ingresses
    ↓
external-dns (watches resources)
    ↓
bindcar API
    ↓
BIND9
```

## Configuration

Configure external-dns to use bindcar as a provider (custom webhook).

## Use Cases

- Automatic DNS for Kubernetes services
- Ingress DNS management
- Service-based zone updates

## Next Steps

- [Integration](./integration.md) - Integration overview
- [Examples](../reference/examples.md) - Example configurations
