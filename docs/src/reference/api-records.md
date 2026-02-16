# Record Management API Reference

Complete API reference for DNS record management endpoints.

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/zones/{zone_name}/records` | Add a new DNS record |
| DELETE | `/api/v1/zones/{zone_name}/records` | Remove a DNS record |
| PUT | `/api/v1/zones/{zone_name}/records` | Update an existing DNS record |

## Add Record

**POST** `/api/v1/zones/{zone_name}/records`

Adds a new DNS record to an existing zone using nsupdate.

### Path Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `zone_name` | string | Yes | Zone name (e.g., `example.com`) |

### Request Headers

| Header | Required | Description |
|--------|----------|-------------|
| `Authorization` | Yes | Bearer token for authentication |
| `Content-Type` | Yes | Must be `application/json` |

### Request Body

```json
{
  "name": "string",
  "type": "string",
  "value": "string",
  "ttl": number,
  "priority": number | null
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Record name (`@` for apex, relative, or FQDN) |
| `type` | string | Yes | Record type (`A`, `AAAA`, `CNAME`, `MX`, `TXT`, `NS`, `PTR`, `SRV`, `CAA`) |
| `value` | string | Yes | Record value (format depends on type) |
| `ttl` | number | No | Time-to-live in seconds (default: 3600) |
| `priority` | number | No | Priority for MX and SRV records (0-65535) |

### Response

**Status**: `201 Created`

```json
{
  "success": true,
  "message": "Record added to zone example.com",
  "details": {
    "zone": "example.com",
    "record": {
      "name": "www",
      "type": "A",
      "value": "192.0.2.100",
      "ttl": 3600
    }
  }
}
```

### Error Responses

| Status | Error | Cause |
|--------|-------|-------|
| 400 | `Invalid request` | Missing or invalid fields |
| 400 | `Dynamic updates not enabled` | Zone lacks `allow-update` |
| 400 | `Invalid record` | Invalid record type or value |
| 404 | `Zone not found` | Zone doesn't exist |
| 500 | `nsupdate command failed` | TSIG auth failure, REFUSED, etc. |

### Examples

#### Add A Record

```bash
curl -X POST http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "A",
    "value": "192.0.2.100",
    "ttl": 3600
  }'
```

#### Add MX Record

```bash
curl -X POST http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "@",
    "type": "MX",
    "value": "mail.example.com.",
    "ttl": 3600,
    "priority": 10
  }'
```

#### Add AAAA Record

```bash
curl -X POST http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "AAAA",
    "value": "2001:db8::1",
    "ttl": 3600
  }'
```

---

## Remove Record

**DELETE** `/api/v1/zones/{zone_name}/records`

Removes a specific DNS record or all records of a type from a zone.

### Path Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `zone_name` | string | Yes | Zone name (e.g., `example.com`) |

### Request Headers

| Header | Required | Description |
|--------|----------|-------------|
| `Authorization` | Yes | Bearer token for authentication |
| `Content-Type` | Yes | Must be `application/json` |

### Request Body

```json
{
  "name": "string",
  "type": "string",
  "value": "string | null"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Record name (`@` for apex, relative, or FQDN) |
| `type` | string | Yes | Record type to remove |
| `value` | string | No | Specific value to remove. If omitted, removes **all** records of this type for the name |

### Response

**Status**: `200 OK`

```json
{
  "success": true,
  "message": "Record removed from zone example.com",
  "details": {
    "zone": "example.com",
    "record": {
      "name": "www",
      "type": "A",
      "value": "192.0.2.100"
    }
  }
}
```

### Error Responses

| Status | Error | Cause |
|--------|-------|-------|
| 400 | `Invalid request` | Missing required fields |
| 400 | `Dynamic updates not enabled` | Zone lacks `allow-update` |
| 400 | `Invalid record` | Invalid record type |
| 404 | `Zone not found` | Zone doesn't exist |
| 500 | `nsupdate command failed` | TSIG auth failure, record not found |

### Examples

#### Remove Specific A Record

```bash
curl -X DELETE http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "A",
    "value": "192.0.2.100"
  }'
```

#### Remove All A Records for a Name

```bash
curl -X DELETE http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "A"
  }'
```

#### Remove MX Record

```bash
curl -X DELETE http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "@",
    "type": "MX",
    "value": "mail.example.com."
  }'
```

---

## Update Record

**PUT** `/api/v1/zones/{zone_name}/records`

Updates an existing DNS record by atomically deleting the old value and adding the new value.

### Path Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `zone_name` | string | Yes | Zone name (e.g., `example.com`) |

### Request Headers

| Header | Required | Description |
|--------|----------|-------------|
| `Authorization` | Yes | Bearer token for authentication |
| `Content-Type` | Yes | Must be `application/json` |

### Request Body

```json
{
  "name": "string",
  "type": "string",
  "currentValue": "string",
  "newValue": "string",
  "ttl": number,
  "priority": number | null
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Record name |
| `type` | string | Yes | Record type |
| `currentValue` | string | Yes | Current record value to replace |
| `newValue` | string | Yes | New record value |
| `ttl` | number | No | New TTL in seconds (default: 3600) |
| `priority` | number | No | Priority for MX and SRV records |

### Response

**Status**: `200 OK`

```json
{
  "success": true,
  "message": "Record updated in zone example.com",
  "details": {
    "zone": "example.com",
    "record": {
      "name": "www",
      "type": "A",
      "currentValue": "192.0.2.100",
      "newValue": "192.0.2.101",
      "ttl": 3600
    }
  }
}
```

### Error Responses

| Status | Error | Cause |
|--------|-------|-------|
| 400 | `Invalid request` | Missing required fields |
| 400 | `Dynamic updates not enabled` | Zone lacks `allow-update` |
| 400 | `Invalid record` | Invalid values |
| 404 | `Zone not found` | Zone doesn't exist |
| 500 | `nsupdate command failed` | TSIG auth failure, record not found |

### Examples

#### Update A Record IP

```bash
curl -X PUT http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "A",
    "currentValue": "192.0.2.100",
    "newValue": "192.0.2.101",
    "ttl": 3600
  }'
```

#### Update MX Record

```bash
curl -X PUT http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "@",
    "type": "MX",
    "currentValue": "mail1.example.com.",
    "newValue": "mail2.example.com.",
    "ttl": 7200,
    "priority": 10
  }'
```

#### Update CNAME Target

```bash
curl -X PUT http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "CNAME",
    "currentValue": "old.example.com.",
    "newValue": "new.example.com.",
    "ttl": 3600
  }'
```

---

## Validation Rules

### Zone Requirements

✅ **Required**:
- Zone must exist
- Zone type must be `primary`
- Zone must have `allow-update` configured

❌ **Not Supported**:
- Secondary zones (read-only, synced from primary)
- Zones without `allow-update` directive

### Record Types

✅ **Supported**: `A`, `AAAA`, `CNAME`, `MX`, `TXT`, `NS`, `PTR`, `SRV`, `CAA`

❌ **Not Supported**: `SOA` (managed automatically), `DNSKEY`, `RRSIG` (DNSSEC managed)

### Value Formats

| Type | Format | Example | Validation |
|------|--------|---------|------------|
| A | IPv4 address | `192.0.2.1` | Valid IPv4 |
| AAAA | IPv6 address | `2001:db8::1` | Valid IPv6 |
| CNAME | FQDN with dot | `target.example.com.` | Ends with `.` |
| MX | FQDN with dot | `mail.example.com.` | Ends with `.` |
| TXT | Any string | `v=spf1 mx -all` | Not empty |
| NS | FQDN with dot | `ns1.example.com.` | Ends with `.` |
| PTR | FQDN with dot | `host.example.com.` | Ends with `.` |
| SRV | Service record | `0 5 5060 sip.example.com.` | Valid format |
| CAA | CAA record | `0 issue "letsencrypt.org"` | Valid format |

### Name Formats

All formats are normalized to FQDN automatically:

| Input | Zone | Result |
|-------|------|--------|
| `@` | `example.com` | `example.com.` |
| `www` | `example.com` | `www.example.com.` |
| `www.example.com.` | `example.com` | `www.example.com.` |
| `api.v2` | `example.com` | `api.v2.example.com.` |

---

## Rate Limiting

Record operations are subject to the same rate limits as other API endpoints (default: 100 requests/60 seconds).

Configure via environment variables:
```bash
RATE_LIMIT_ENABLED=true
RATE_LIMIT_REQUESTS=100
RATE_LIMIT_PERIOD_SECS=60
RATE_LIMIT_BURST=10
```

---

## Metrics

All record operations are tracked with Prometheus metrics:

```prometheus
# Operation counts
bindcar_zone_operations_total{operation="record_add",result="success"} 42
bindcar_zone_operations_total{operation="record_remove",result="success"} 15
bindcar_zone_operations_total{operation="record_update",result="success"} 8

# nsupdate command duration
bindcar_rndc_command_duration_seconds{command="nsupdate_update"} 0.234
```

Monitor via `/metrics` endpoint:
```bash
curl http://localhost:8080/metrics
```

---

## See Also

- [Managing DNS Records](../user-guide/managing-records.md) - Usage guide and examples
- [DNS Record Types](../user-guide/dns-records.md) - Detailed record type reference
- [Environment Variables](../operations/env-vars.md) - TSIG configuration
- [Troubleshooting](../operations/troubleshooting.md) - Common issues
