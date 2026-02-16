# API Overview

bindcar provides a RESTful HTTP API for managing BIND9 DNS zones.

## API Design Principles

- **RESTful** - Standard HTTP methods (GET, POST, DELETE)
- **JSON** - All requests and responses use JSON
- **Versioned** - API version in URL path (`/api/v1`)
- **Authenticated** - Bearer token authentication (configurable)
- **OpenAPI** - Interactive documentation via Swagger UI

## Base URL

```
http://localhost:8080/api/v1
```

In production, use HTTPS:
```
https://your-domain.com/api/v1
```

## API Structure

```
/api/v1
├── /health              # Health check (no auth)
├── /ready               # Readiness check (no auth)
├── /server
│   └── /status          # BIND9 server status
└── /zones
    ├── /                # List/create zones
    └── /{name}
        ├── /            # Get/delete zone
        ├── /reload      # Reload zone
        ├── /status      # Zone status
        ├── /freeze      # Freeze zone updates
        ├── /thaw        # Thaw zone updates
        ├── /notify      # Notify secondaries
        └── /records     # Manage individual DNS records
            ├── POST     # Add record
            ├── DELETE   # Remove record
            └── PUT      # Update record
```

## Authentication

All endpoints except `/health` and `/ready` require Bearer token authentication:

```http
Authorization: Bearer <token>
```

See [Authentication](../operations/authentication.md) for details.

## Interactive Documentation

bindcar provides interactive API documentation via Swagger UI:

```
http://localhost:8080/api/v1/docs
```

## Next Steps

- [Health & Status](./health-status.md) - Health check endpoints
- [Zone Operations](./zone-operations.md) - Zone management operations
- [Managing DNS Records](./managing-records.md) - Individual record management
- [API Reference](../reference/api.md) - Complete API documentation
