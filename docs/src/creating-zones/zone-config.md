# Zone Configuration

Detailed reference for zone configuration structure.

## ZoneConfig Structure

```json
{
  "zoneName": "string",
  "zoneType": "master|slave",
  "zoneConfig": {
    "ttl": integer,
    "soa": { ... },
    "nameservers": [ ... ],
    "records": [ ... ]
  },
  "updateKeyName": "string (optional)"
}
```

## Zone Name

- **Type**: String
- **Required**: Yes
- **Format**: Valid DNS domain name
- **Examples**: `example.com`, `subdomain.example.com`, `example.co.uk`

```json
{
  "zoneName": "example.com"
}
```

## Zone Type

- **Type**: String
- **Required**: Yes
- **Values**: `master` or `slave`

```json
{
  "zoneType": "master"
}
```

Master zones are authoritative zones managed locally.
Slave zones are transferred from a master server.

## TTL (Time To Live)

- **Type**: Integer
- **Required**: Yes
- **Unit**: Seconds
- **Common Values**: 300, 3600, 86400

```json
{
  "ttl": 3600
}
```

Default TTL for records without explicit TTL.

## SOA Record

Start of Authority record - required for all zones.

```json
{
  "soa": {
    "primaryNs": "ns1.example.com.",
    "adminEmail": "admin.example.com.",
    "serial": 1,
    "refresh": 3600,
    "retry": 1800,
    "expire": 604800,
    "negativeTtl": 86400
  }
}
```

### Fields

| Field | Type | Description | Typical Value |
|-------|------|-------------|---------------|
| primaryNs | String | Primary nameserver FQDN | `ns1.example.com.` |
| adminEmail | String | Admin email (dots, not @) | `admin.example.com.` |
| serial | Integer | Zone serial number | 1 or timestamp |
| refresh | Integer | Secondary refresh interval (seconds) | 3600 |
| retry | Integer | Retry interval on refresh failure (seconds) | 1800 |
| expire | Integer | Zone expiry time (seconds) | 604800 |
| negativeTtl | Integer | Negative caching TTL (seconds) | 86400 |

**Note**: FQDN must end with a dot (`.`)

## Nameservers

List of authoritative nameservers for the zone.

```json
{
  "nameservers": [
    "ns1.example.com.",
    "ns2.example.com."
  ]
}
```

- **Type**: Array of strings
- **Required**: Yes
- **Minimum**: 1 (2+ recommended)
- **Format**: FQDN ending with dot

## Records

List of DNS records for the zone.

```json
{
  "records": [
    {
      "name": "@",
      "type": "A",
      "value": "192.0.2.1",
      "ttl": 3600
    },
    {
      "name": "www",
      "type": "CNAME",
      "value": "example.com."
    }
  ]
}
```

See [DNS Records](./dns-records.md) for detailed record documentation.

## Update Key Name (Optional)

For dynamic updates with TSIG authentication.

```json
{
  "updateKeyName": "update-key"
}
```

Requires BIND9 to be configured with the named key.

## Complete Example

```json
{
  "zoneName": "example.com",
  "zoneType": "master",
  "zoneConfig": {
    "ttl": 3600,
    "soa": {
      "primaryNs": "ns1.example.com.",
      "adminEmail": "hostmaster.example.com.",
      "serial": 2024010101,
      "refresh": 7200,
      "retry": 3600,
      "expire": 1209600,
      "negativeTtl": 86400
    },
    "nameservers": [
      "ns1.example.com.",
      "ns2.example.com.",
      "ns3.example.com."
    ],
    "records": [
      {
        "name": "@",
        "type": "A",
        "value": "192.0.2.1",
        "ttl": 300
      },
      {
        "name": "@",
        "type": "MX",
        "value": "mail.example.com.",
        "priority": 10
      },
      {
        "name": "www",
        "type": "A",
        "value": "192.0.2.1"
      },
      {
        "name": "mail",
        "type": "A",
        "value": "192.0.2.2"
      },
      {
        "name": "@",
        "type": "TXT",
        "value": "v=spf1 mx -all"
      }
    ]
  }
}
```

## Next Steps

- [DNS Records](./dns-records.md) - Record types and formats
- [Managing Zones](../managing-zones/index.md) - Manage zones after creation
