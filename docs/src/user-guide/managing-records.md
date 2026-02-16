# Managing DNS Records

Individual DNS record management allows you to add, update, and remove specific DNS records in existing zones without recreating the entire zone.

## Overview

Bindcar provides REST API endpoints for granular DNS record operations using BIND9's dynamic DNS update protocol (RFC 2136). All operations use `nsupdate` with TSIG authentication for secure updates.

## Prerequisites

Before you can manage individual records, your zone must be configured to support dynamic updates:

### 1. Enable Dynamic Updates on Zone Creation

When creating a zone, specify the TSIG key name for `allow-update`:

```bash
curl -X POST http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "zoneName": "example.com",
    "zoneType": "primary",
    "updateKeyName": "update-key",
    "zoneConfig": {
      "ttl": 3600,
      "soa": {
        "primaryNs": "ns1.example.com.",
        "adminEmail": "admin.example.com."
      },
      "nameServers": ["ns1.example.com."],
      "nameServerIps": {
        "ns1.example.com.": "192.0.2.10"
      }
    }
  }'
```

### 2. Verify Zone Supports Updates

Check that your zone has `allow-update` configured:

```bash
curl http://localhost:8080/api/v1/zones/example.com/status \
  -H "Authorization: Bearer $TOKEN"
```

The response should include `allow-update` in the zone configuration.

### 3. Configure TSIG Keys

Bindcar needs TSIG credentials to authenticate nsupdate commands. Configure via environment variables:

```bash
# Option 1: Dedicated nsupdate credentials
export NSUPDATE_KEY_NAME="update-key"
export NSUPDATE_ALGORITHM="HMAC-SHA256"
export NSUPDATE_SECRET="base64-encoded-secret"

# Option 2: Use RNDC credentials (automatic fallback)
# If NSUPDATE_* vars not set, bindcar uses RNDC credentials
```

## Operations

### Add a DNS Record

Add a new DNS record to an existing zone.

**Endpoint**: `POST /api/v1/zones/{zone_name}/records`

**Request Body**:
```json
{
  "name": "www",
  "type": "A",
  "value": "192.0.2.100",
  "ttl": 3600,
  "priority": null
}
```

**Example**:
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

**Response**:
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

### Remove a DNS Record

Remove a specific DNS record or all records of a type.

**Endpoint**: `DELETE /api/v1/zones/{zone_name}/records`

**Request Body**:
```json
{
  "name": "www",
  "type": "A",
  "value": "192.0.2.100"
}
```

**Remove Specific Record**:
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

**Remove All Records of a Type** (omit `value`):
```bash
curl -X DELETE http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "A"
  }'
```

**Response**:
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

### Update a DNS Record

Update an existing DNS record (atomic delete + add).

**Endpoint**: `PUT /api/v1/zones/{zone_name}/records`

**Request Body**:
```json
{
  "name": "www",
  "type": "A",
  "currentValue": "192.0.2.100",
  "newValue": "192.0.2.101",
  "ttl": 7200,
  "priority": null
}
```

**Example**:
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

**Response**:
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

## Common Use Cases

### Multiple A Records (Load Balancing)

Add multiple A records with the same name but different IP addresses:

```bash
# Add first IP
curl -X POST http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "www", "type": "A", "value": "192.0.2.1", "ttl": 300}'

# Add second IP
curl -X POST http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "www", "type": "A", "value": "192.0.2.2", "ttl": 300}'

# Remove specific IP (others remain)
curl -X DELETE http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "www", "type": "A", "value": "192.0.2.1"}'
```

### MX Records with Priority

```bash
curl -X POST http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "@",
    "type": "MX",
    "value": "mail1.example.com.",
    "ttl": 3600,
    "priority": 10
  }'

curl -X POST http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "@",
    "type": "MX",
    "value": "mail2.example.com.",
    "ttl": 3600,
    "priority": 20
  }'
```

### TXT Records (SPF, DKIM)

```bash
curl -X POST http://localhost:8080/api/v1/zones/example.com/records \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "@",
    "type": "TXT",
    "value": "v=spf1 mx -all",
    "ttl": 3600
  }'
```

### IPv6 (AAAA) Records

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

## Record Name Formats

The `name` field accepts several formats:

| Format | Description | Example | Resolves To |
|--------|-------------|---------|-------------|
| `@` | Zone apex | `{"name": "@"}` | `example.com.` |
| Relative | Relative to zone | `{"name": "www"}` | `www.example.com.` |
| FQDN | Fully qualified | `{"name": "www.example.com."}` | `www.example.com.` |
| Subdomain | Multi-level | `{"name": "api.v2"}` | `api.v2.example.com.` |

Bindcar automatically normalizes names to FQDNs before sending to nsupdate.

## Supported Record Types

| Type | Description | Value Format | Priority Required |
|------|-------------|--------------|-------------------|
| A | IPv4 address | `192.0.2.1` | No |
| AAAA | IPv6 address | `2001:db8::1` | No |
| CNAME | Canonical name | `target.example.com.` | No |
| MX | Mail exchange | `mail.example.com.` | Yes |
| TXT | Text record | Any string | No |
| NS | Name server | `ns1.example.com.` | No |
| PTR | Pointer (reverse DNS) | `host.example.com.` | No |
| SRV | Service locator | `0 5 5060 sip.example.com.` | Yes |
| CAA | Certificate authority | `0 issue "letsencrypt.org"` | No |

## Validation

Bindcar validates all record operations before sending to nsupdate:

### Zone Validation
- Zone must exist
- Zone must be `primary` type (secondary zones don't support updates)
- Zone must have `allow-update` configured

### Record Type Validation
- Type must be one of the supported types
- Type is case-insensitive (converted to uppercase)

### Value Validation
- **A records**: Must be valid IPv4 address
- **AAAA records**: Must be valid IPv6 address
- **CNAME, NS, PTR, MX**: Must end with `.` (FQDN)
- **All types**: Cannot be empty

## Error Handling

### Common Errors

**Dynamic Updates Not Enabled** (400 Bad Request):
```json
{
  "error": "Dynamic updates not enabled: Zone example.com does not have allow-update configured"
}
```

**Solution**: Create zone with `updateKeyName` or modify zone configuration.

---

**Invalid Record** (400 Bad Request):
```json
{
  "error": "Invalid record: Invalid IPv4 address: 999.999.999.999"
}
```

**Solution**: Fix the record value to match the expected format for the record type.

---

**nsupdate Failed: REFUSED** (500 Internal Server Error):
```json
{
  "error": "nsupdate command failed: Zone refused the update (check allow-update configuration)"
}
```

**Solution**: Verify TSIG key is correct and listed in zone's `allow-update` directive.

---

**nsupdate Failed: NOTAUTH** (500 Internal Server Error):
```json
{
  "error": "nsupdate command failed: Not authorized (check TSIG key configuration)"
}
```

**Solution**: Verify `NSUPDATE_KEY_NAME`, `NSUPDATE_ALGORITHM`, and `NSUPDATE_SECRET` are correct.

## Best Practices

### 1. Use Appropriate TTLs

- **Short TTL (60-300s)**: For frequently changing records (dynamic IPs, load balancers)
- **Medium TTL (3600s)**: For standard web services
- **Long TTL (86400s)**: For rarely changing records (NS, SOA)

### 2. Verify Changes

After modifying records, verify with `dig`:

```bash
dig @127.0.0.1 www.example.com A
```

### 3. Atomic Updates

The UPDATE operation is atomic - it deletes the old record and adds the new one in a single transaction, preventing race conditions.

### 4. Serial Number Management

nsupdate automatically increments the zone's serial number - no manual management needed!

### 5. Audit Trail

All operations are logged with structured logging. Monitor logs for unauthorized changes:

```bash
# View record operations
kubectl logs -l app=bindcar | grep "record"
```

## Security Considerations

### TSIG Authentication

- Always use TSIG authentication in production
- Use strong, randomly generated secrets (minimum 32 bytes)
- Rotate keys periodically
- Different keys for RNDC vs nsupdate provides defense in depth

### Restrict allow-update

In BIND9 `named.conf`, restrict updates to specific keys:

```bind
zone "example.com" {
    type master;
    file "/var/cache/bind/example.com.zone";
    allow-update { key "update-key"; };  // Only this key can update
};
```

### Network Security

When running bindcar in Kubernetes:

- Use **Linkerd** or similar service mesh for mTLS
- Use NetworkPolicies to restrict access
- Enable authentication (disable only in trusted environments)

## Next Steps

- [DNS Record Types](./dns-records.md) - Detailed reference for each record type
- [Zone Configuration](./zone-config.md) - Configure zones for dynamic updates
- [Troubleshooting](../operations/troubleshooting.md) - Common issues and solutions
- [API Reference](../reference/api-records.md) - Complete API documentation
