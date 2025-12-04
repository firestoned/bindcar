# Quick Start

Get bindcar up and running in minutes with this quick start guide.

## Using Docker

The fastest way to try bindcar:

```bash
# Pull the latest image
docker pull ghcr.io/firestoned/bindcar:latest

# Run bindcar
docker run -d \
  --name bindcar \
  -p 8080:8080 \
  -v /var/cache/bind:/var/cache/bind \
  -e RUST_LOG=info \
  ghcr.io/firestoned/bindcar:latest
```

## Verify It's Running

Check the health endpoint:

```bash
curl http://localhost:8080/api/v1/health
```

Expected response:
```json
{"healthy":true}
```

## Create Your First Zone

```bash
curl -X POST http://localhost:8080/api/v1/zones \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token-here" \
  -d '{
    "zoneName": "example.com",
    "zoneType": "master",
    "zoneConfig": {
      "ttl": 3600,
      "soa": {
        "primaryNs": "ns1.example.com.",
        "adminEmail": "admin.example.com.",
        "serial": 1,
        "refresh": 3600,
        "retry": 1800,
        "expire": 604800,
        "negativeTtl": 86400
      },
      "nameservers": ["ns1.example.com.", "ns2.example.com."],
      "records": [
        {
          "name": "@",
          "type": "A",
          "value": "192.0.2.1",
          "ttl": 3600
        }
      ]
    }
  }'
```

## List Zones

```bash
curl http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer your-token-here"
```

## Get Zone Status

```bash
curl http://localhost:8080/api/v1/zones/example.com/status \
  -H "Authorization: Bearer your-token-here"
```

## Interactive API Documentation

bindcar provides interactive Swagger UI documentation:

```bash
open http://localhost:8080/api/v1/docs
```

## Next Steps

- [Configuration](./configuration.md) - Configure environment variables
- [Authentication](./authentication.md) - Set up authentication
- [Creating Zones](./creating-zones.md) - Learn more about zone creation
- [API Reference](./api-reference.md) - Complete API documentation
