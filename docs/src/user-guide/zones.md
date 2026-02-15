# Creating Zones

Learn how to create DNS zones using the bindcar API.

## Overview

Zone creation in bindcar involves:
1. Preparing zone configuration (SOA, NS records, DNS records)
2. Sending POST request to `/api/v1/zones`
3. bindcar generates zone file and executes `rndc addzone`
4. BIND9 loads the new zone

## Quick Example

```bash
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
      "records": [
        {
          "name": "@",
          "type": "A",
          "value": "192.0.2.1",
          "ttl": 3600
        },
        {
          "name": "www",
          "type": "A",
          "value": "192.0.2.1"
        }
      ]
    }
  }'
```

## Zone Configuration

See [Zone Configuration](./zone-config.md) for detailed configuration options.

## DNS Records

See [DNS Records](./dns-records.md) for supported record types.

## Best Practices

1. **Use appropriate TTL values** - Balance between update frequency and caching
2. **Set correct SOA values** - Especially refresh, retry, and expire
3. **Include NS records** - At least two nameservers recommended
4. **Validate before creating** - Check zone name format and record values
5. **Start with minimal records** - Add more records after zone is verified

## Validation

bindcar validates:
- Zone name format (valid domain name)
- SOA record completeness
- Record types and values
- TTL values are positive integers

Invalid configurations return 400 Bad Request with error details.

## Next Steps

- [Zone Configuration](./zone-config.md) - Detailed configuration reference
- [DNS Records](./dns-records.md) - Supported DNS record types
- [Managing Zones](./zones.md) - Manage existing zones
