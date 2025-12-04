# Managing DNS Records

DNS records define how your domain resolves to IP addresses and other resources.

## Overview

DNS records are included in zone creation requests. bindcar supports all common DNS record types used in BIND9 zones.

## Record Structure

All DNS records share a common structure:

```json
{
  "name": "@",           // Record name (@ for zone apex)
  "type": "A",           // Record type
  "value": "192.0.2.1",  // Record value
  "ttl": 3600            // Optional: override zone TTL
}
```

### Fields

- **name** (required): The record name relative to the zone
  - @ - Zone apex (example.com)
  - www - Subdomain (www.example.com)
  - mail - Another subdomain (mail.example.com)
  
- **type** (required): The DNS record type
  - See [DNS Record Types](./dns-records.md) for full list
  
- **value** (required): The record data
  - Format depends on record type
  
- **ttl** (optional): Time-to-live in seconds
  - Overrides zone default TTL
  - If omitted, uses zone TTL

- **priority** (optional): For MX and SRV records
  - Lower values have higher priority

## Next Steps

- [DNS Record Types](./dns-records.md) - Complete record type reference
- [Zone Configuration](./zone-config.md) - Zone configuration details
- [Creating Zones](./creating-zones.md) - Zone creation guide
