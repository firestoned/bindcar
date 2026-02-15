# Zone Status

Get zone information and status from BIND9.

## Get Zone Status

**GET** `/api/v1/zones/{name}/status`

Returns detailed zone status from BIND9.

### Request

```bash
curl http://localhost:8080/api/v1/zones/example.com/status \
  -H "Authorization: Bearer $TOKEN"
```

### Response

```json
{
  "success": true,
  "message": "Zone example.com status retrieved",
  "details": "name: example.com\ntype: master\nfiles: example.com.zone\nserial: 2024010101\nnodes: 8\nlast loaded: Tue Dec  3 10:30:45 2024\nsecure: no\ninline signing: no\nkey maintenance: no"
}
```

### Status Codes

- `200 OK` - Status retrieved successfully
- `401 Unauthorized` - Missing/invalid token
- `404 Not Found` - Zone doesn't exist
- `500 Internal Server Error` - RNDC command failed

## Get Zone Info

**GET** `/api/v1/zones/{name}`

Returns zone information including file path and metadata.

### Request

```bash
curl http://localhost:8080/api/v1/zones/example.com \
  -H "Authorization: Bearer $TOKEN"
```

### Response

```json
{
  "name": "example.com",
  "zoneType": "primary",
  "serial": 2024010101,
  "filePath": "/var/cache/bind/example.com.zone"
}
```

## Interpreting Status

### Zone Type

- **primary** - Authoritative zone, managed locally
- **secondary** - Secondary zone, transferred from primary
- **stub** - Stub zone with only NS records

### Serial Number

- Incremented with each zone update
- Used by secondaries to detect changes
- Format: YYYYMMDDnn (recommended)

### Nodes

Number of names (nodes) in the zone.

### Last Loaded

Timestamp when zone was last loaded into BIND9.

### Secure/DNSSEC

Indicates if DNSSEC is enabled for the zone.

## Use Cases

### Verify Zone Loaded

```bash
# Check if zone exists and is loaded
curl http://localhost:8080/api/v1/zones/example.com/status \
  -H "Authorization: Bearer $TOKEN"
```

### Check Serial After Update

```bash
# After reloading, verify serial incremented
curl http://localhost:8080/api/v1/zones/example.com/status \
  -H "Authorization: Bearer $TOKEN" | grep serial
```

### Monitor Zone Health

```bash
# Regular health check
curl http://localhost:8080/api/v1/zones/example.com/status \
  -H "Authorization: Bearer $TOKEN" | jq '.success'
```

### List All Zones

```bash
# Get all zones
curl http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer $TOKEN"
```

Response:
```json
{
  "zones": ["example.com", "test.com"],
  "count": 2
}
```

## Monitoring Example

```bash
#!/bin/bash
TOKEN="your-token-here"

# Check all zones
zones=$(curl -s -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/zones | jq -r '.zones[]')

for zone in $zones; do
  echo "Checking $zone..."
  curl -s -H "Authorization: Bearer $TOKEN" \
    "http://localhost:8080/api/v1/zones/$zone/status" | \
    grep -q "success.*true" && echo "$zone: OK" || echo "$zone: FAIL"
done
```

## Next Steps

- [Reloading Zones](./reloading-zones.md) - Reload after changes
- [Deleting Zones](./deleting-zones.md) - Remove zones
- [API Reference](../reference/api-zones.md) - Complete endpoint documentation
