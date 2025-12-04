# Docker Deployment

Deploy bindcar using Docker containers.

## Prerequisites

- Docker 20.10 or later
- Docker Compose (optional)
- BIND9 container or installation

## Quick Start

### Pull the Image

```bash
docker pull ghcr.io/firestoned/bindcar:latest
```

### Run bindcar

```bash
docker run -d \
  --name bindcar \
  -p 8080:8080 \
  -v /var/cache/bind:/var/cache/bind \
  -e RUST_LOG=info \
  -e BIND_ZONE_DIR=/var/cache/bind \
  ghcr.io/firestoned/bindcar:latest
```

## Docker Compose

### Complete Stack

```yaml
version: '3.8'

services:
  bind9:
    image: ubuntu/bind9:latest
    container_name: bind9
    ports:
      - "53:53/tcp"
      - "53:53/udp"
    volumes:
      - zones:/var/cache/bind
      - ./named.conf:/etc/bind/named.conf
    restart: unless-stopped

  bindcar:
    image: ghcr.io/firestoned/bindcar:latest
    container_name: bindcar
    ports:
      - "8080:8080"
    environment:
      - BIND_ZONE_DIR=/var/cache/bind
      - API_PORT=8080
      - RUST_LOG=info
      - DISABLE_AUTH=false
    volumes:
      - zones:/var/cache/bind
    depends_on:
      - bind9
    restart: unless-stopped

volumes:
  zones:
```

### Start the Stack

```bash
docker-compose up -d
```

### Verify

```bash
# Check containers are running
docker-compose ps

# Check bindcar health
curl http://localhost:8080/api/v1/health

# Check logs
docker-compose logs -f bindcar
```

## Environment Variables

See [Environment Variables](../configuration/env-vars.md) for complete reference.

Common variables:

```bash
BIND_ZONE_DIR=/var/cache/bind
API_PORT=8080
RUST_LOG=info
RNDC_PATH=/usr/sbin/rndc
DISABLE_AUTH=false
```

## Volumes

### Zone Directory

Must be shared between BIND9 and bindcar:

```bash
-v zones:/var/cache/bind
```

Options:
- Named volume (recommended for production)
- Host path (for development)
- tmpfs (for testing)

## Networking

### Bridge Network (Default)

```yaml
services:
  bind9:
    networks:
      - dns-network
  bindcar:
    networks:
      - dns-network

networks:
  dns-network:
    driver: bridge
```

### Host Network

For direct host access:

```bash
docker run --network host \
  ghcr.io/firestoned/bindcar:latest
```

## Security

### Run as Non-Root

bindcar runs as UID 1000 by default:

```dockerfile
USER bindcar
```

### Read-Only Root Filesystem

```bash
docker run --read-only \
  -v /var/cache/bind:/var/cache/bind \
  ghcr.io/firestoned/bindcar:latest
```

### Limit Resources

```yaml
deploy:
  resources:
    limits:
      cpus: '0.5'
      memory: 512M
    reservations:
      cpus: '0.25'
      memory: 256M
```

## Health Checks

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8080/api/v1/health"]
  interval: 30s
  timeout: 3s
  retries: 3
  start_period: 5s
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker logs bindcar

# Check permissions
docker exec bindcar ls -la /var/cache/bind
```

### Cannot Connect to API

```bash
# Check port binding
docker port bindcar

# Check firewall
sudo ufw status
```

### RNDC Command Fails

```bash
# Verify rndc is accessible
docker exec bindcar which rndc

# Test rndc
docker exec bind9 rndc status
```

## Next Steps

- [Kubernetes Deployment](./kubernetes.md) - Deploy to Kubernetes
- [Configuration](../configuration/index.md) - Advanced configuration
- [Monitoring](../monitoring/index.md) - Monitor your deployment
