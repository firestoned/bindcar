# Zone Operations

Overview of zone management operations available through the bindcar API.

## Available Operations

### Zone Lifecycle

1. **Create Zone** - Create a new DNS zone
2. **List Zones** - Get all zones
3. **Get Zone** - Get specific zone information
4. **Delete Zone** - Remove a zone
5. **Reload Zone** - Reload zone from file
6. **Zone Status** - Get zone status from BIND9

### Zone Management

7. **Freeze Zone** - Disable dynamic updates
8. **Thaw Zone** - Enable dynamic updates
9. **Notify Secondaries** - Trigger zone transfer notifications

## Quick Reference

| Operation | Method | Endpoint | Auth Required |
|-----------|--------|----------|---------------|
| Create Zone | POST | `/api/v1/zones` | Yes |
| List Zones | GET | `/api/v1/zones` | Yes |
| Get Zone | GET | `/api/v1/zones/{name}` | Yes |
| Delete Zone | DELETE | `/api/v1/zones/{name}` | Yes |
| Reload Zone | POST | `/api/v1/zones/{name}/reload` | Yes |
| Zone Status | GET | `/api/v1/zones/{name}/status` | Yes |
| Freeze Zone | POST | `/api/v1/zones/{name}/freeze` | Yes |
| Thaw Zone | POST | `/api/v1/zones/{name}/thaw` | Yes |
| Notify Secondaries | POST | `/api/v1/zones/{name}/notify` | Yes |

## Common Workflows

### Creating a New Zone

```bash
# 1. Create zone with configuration
curl -X POST http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "zoneName": "example.com",
    "zoneType": "primary",
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
      "records": []
    }
  }'

# 2. Verify zone was created
curl http://localhost:8080/api/v1/zones/example.com \
  -H "Authorization: Bearer $TOKEN"
```

### Updating a Zone

```bash
# 1. Modify zone file directly on filesystem
#    (bindcar and BIND9 share the same volume)

# 2. Reload the zone in BIND9
curl -X POST http://localhost:8080/api/v1/zones/example.com/reload \
  -H "Authorization: Bearer $TOKEN"

# 3. Verify changes were applied
curl http://localhost:8080/api/v1/zones/example.com/status \
  -H "Authorization: Bearer $TOKEN"
```

### Deleting a Zone

```bash
# 1. Check zone exists
curl http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer $TOKEN"

# 2. Delete the zone
curl -X DELETE http://localhost:8080/api/v1/zones/example.com \
  -H "Authorization: Bearer $TOKEN"

# 3. Verify deletion
curl http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer $TOKEN"
```

## Next Steps

- [Creating Zones](../creating-zones/index.md) - Detailed zone creation guide
- [Managing Zones](../managing-zones/index.md) - Zone management operations
- [API Reference](../api-reference/api-zones.md) - Complete zone endpoint reference
