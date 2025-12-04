# Deployment

bindcar can be deployed in various environments, from local development to production Kubernetes clusters.

## Deployment Options

### Docker

Simplest deployment method using Docker containers.

- Suitable for: Development, testing, small deployments
- Requires: Docker, shared volume with BIND9
- See: [Docker Deployment](./docker.md)

### Kubernetes

Recommended for production deployments using the sidecar pattern.

- Suitable for: Production, high availability, scale
- Requires: Kubernetes cluster, BIND9 pod
- See: [Kubernetes Deployment](./kubernetes.md)

## Architecture Patterns

### Sidecar Pattern (Recommended)

bindcar runs as a sidecar container alongside BIND9 in the same pod:

```
┌─────────────────────────────┐
│          Pod                │
│  ┌──────────┐  ┌─────────┐ │
│  │  BIND9   │  │ bindcar │ │
│  │  :53     │  │  :8080  │ │
│  └──────────┘  └─────────┘ │
│       │            │        │
│       └────────────┘        │
│     Shared Volume           │
│   /var/cache/bind           │
└─────────────────────────────┘
```

Benefits:
- Shared filesystem for zone files
- Local rndc communication
- Atomic deployment updates
- Resource sharing

### Standalone Pattern

bindcar and BIND9 run separately:

```
┌──────────┐      ┌─────────┐
│  BIND9   │      │ bindcar │
│  :53     │◄─────┤  :8080  │
└──────────┘      └─────────┘
     │                 │
     └────Network──────┘
       Zone Files
```

Use when:
- BIND9 already deployed
- Cannot modify existing BIND9 deployment
- Testing or development

## Quick Start

### Docker Compose

```yaml
version: '3.8'
services:
  bind9:
    image: ubuntu/bind9:latest
    ports:
      - "53:53/tcp"
      - "53:53/udp"
    volumes:
      - zones:/var/cache/bind

  bindcar:
    image: ghcr.io/firestoned/bindcar:latest
    ports:
      - "8080:8080"
    environment:
      - BIND_ZONE_DIR=/var/cache/bind
      - RUST_LOG=info
    volumes:
      - zones:/var/cache/bind

volumes:
  zones:
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
    image: ubuntu/bind9:latest
    volumeMounts:
    - name: zones
      mountPath: /var/cache/bind

  - name: bindcar
    image: ghcr.io/firestoned/bindcar:latest
    ports:
    - containerPort: 8080
    volumeMounts:
    - name: zones
      mountPath: /var/cache/bind

  volumes:
  - name: zones
    emptyDir: {}
```

## Configuration

See [Configuration](../configuration/index.md) for environment variables and settings.

## Security Considerations

- Enable authentication in production
- Use HTTPS with TLS termination
- Implement network policies in Kubernetes
- Use least-privilege service accounts
- Rotate tokens regularly

## Next Steps

- [Docker Deployment](./docker.md) - Deploy with Docker
- [Kubernetes Deployment](./kubernetes.md) - Deploy to Kubernetes
- [Configuration](../configuration/index.md) - Configure bindcar
- [Monitoring](../monitoring/index.md) - Monitor your deployment
