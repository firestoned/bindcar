# Zone Configuration

Detailed reference for zone configuration structure.

## ZoneConfig Structure

```json
{
  "zoneName": "string",
  "zoneType": "primary|secondary",
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
- **Values**: `primary` or `secondary`

```json
{
  "zoneType": "primary"
}
```

Primary zones are authoritative zones managed locally.
Secondary zones are transferred from a primary server.

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

## Zone Transfer Configuration (Optional)

Configure zone transfers for high availability with secondary DNS servers.

### Also Notify

IP addresses of secondary servers to notify when the zone changes:

```json
{
  "alsoNotify": ["10.244.2.101", "10.244.2.102"]
}
```

- **Type**: Array of strings
- **Format**: IP addresses (IPv4 or IPv6)
- **Use Case**: Automatically notify secondary servers of zone updates
- **BIND9 Directive**: `also-notify { ... };`

### Allow Transfer

IP addresses allowed to transfer the zone:

```json
{
  "allowTransfer": ["10.244.2.101", "10.244.2.102"]
}
```

- **Type**: Array of strings
- **Format**: IP addresses (IPv4 or IPv6)
- **Use Case**: Control which servers can request zone transfers
- **BIND9 Directive**: `allow-transfer { ... };`

### High Availability Example

```json
{
  "zoneName": "example.com",
  "zoneType": "primary",
  "zoneConfig": {
    "ttl": 3600,
    "soa": { ... },
    "nameServers": ["ns1.example.com.", "ns2.example.com."],
    "nameServerIps": {
      "ns1.example.com.": "10.244.1.101",
      "ns2.example.com.": "10.244.2.101"
    },
    "records": [ ... ],
    "alsoNotify": ["10.244.2.101", "10.244.2.102"],
    "allowTransfer": ["10.244.2.101", "10.244.2.102"]
  }
}
```

**Benefits**:
- Automatic zone replication to secondary servers
- High availability (HA) if primary fails
- Load distribution for DNS queries
- Geographic redundancy

## DNSSEC Configuration (Optional)

Configure DNSSEC (DNS Security Extensions) to add cryptographic signatures to your DNS records.

### DNSSEC Policy

Specifies which DNSSEC policy to apply to the zone:

```json
{
  "dnssecPolicy": "default"
}
```

- **Type**: String
- **Format**: Name of a DNSSEC policy defined in BIND9 configuration
- **Examples**: `"default"`, `"high-security"`, `"custom-policy"`
- **Requirements**:
  - BIND9 9.16 or newer
  - Policy must be defined in `named.conf`
  - Should be used with `inlineSigning: true`

### Inline Signing

Enables BIND9's automatic inline signing:

```json
{
  "inlineSigning": true
}
```

- **Type**: Boolean
- **Default**: `false`
- **Use Case**: Required for DNSSEC with `dnssecPolicy`
- **How it works**: BIND9 automatically signs the zone and manages keys

### DNSSEC Example

```json
{
  "zoneName": "secure.example.com",
  "zoneType": "primary",
  "zoneConfig": {
    "ttl": 3600,
    "soa": { ... },
    "nameServers": ["ns1.secure.example.com."],
    "records": [ ... ],
    "dnssecPolicy": "default",
    "inlineSigning": true
  }
}
```

**Prerequisites**:
- Define DNSSEC policy in BIND9 `named.conf`:
  ```bind
  dnssec-policy "default" {
      keys {
          ksk lifetime unlimited algorithm ecdsa256;
          zsk lifetime 30d algorithm ecdsa256;
      };
      signatures-validity 14d;
  };
  ```

See [DNSSEC Guide](../advanced/dnssec.md) for comprehensive documentation.

## Complete Example

Full zone configuration with all optional fields:

```json
{
  "zoneName": "example.com",
  "zoneType": "primary",
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
    "nameServers": [
      "ns1.example.com.",
      "ns2.example.com.",
      "ns3.example.com."
    ],
    "nameServerIps": {
      "ns1.example.com.": "10.244.1.101",
      "ns2.example.com.": "10.244.2.101",
      "ns3.example.com.": "10.244.3.101"
    },
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
    ],
    "alsoNotify": ["10.244.2.101", "10.244.3.101"],
    "allowTransfer": ["10.244.2.101", "10.244.3.101"],
    "dnssecPolicy": "default",
    "inlineSigning": true
  },
  "updateKeyName": "update-key"
}
```

## Next Steps

- [DNS Records](./dns-records.md) - Record types and formats
- [Managing Zones](./zones.md) - Manage zones after creation
