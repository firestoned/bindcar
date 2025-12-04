# DNS Records

Supported DNS record types and formats.

## Record Structure

```json
{
  "name": "string",
  "type": "string",
  "value": "string",
  "ttl": integer (optional),
  "priority": integer (optional, for MX/SRV)
}
```

## Common Fields

### name

- **Type**: String
- **Required**: Yes
- **Format**: Hostname or `@` for zone apex
- **Examples**: `@`, `www`, `mail`, `*.wildcard`

### type

- **Type**: String
- **Required**: Yes
- **Values**: A, AAAA, CNAME, MX, TXT, NS, PTR, SRV, CAA

### value

- **Type**: String
- **Required**: Yes
- **Format**: Depends on record type

### ttl

- **Type**: Integer
- **Required**: No (uses zone default if omitted)
- **Unit**: Seconds

### priority

- **Type**: Integer
- **Required**: For MX and SRV records
- **Range**: 0-65535

## Supported Record Types

### A Record (IPv4 Address)

Maps hostname to IPv4 address.

```json
{
  "name": "www",
  "type": "A",
  "value": "192.0.2.1",
  "ttl": 3600
}
```

### AAAA Record (IPv6 Address)

Maps hostname to IPv6 address.

```json
{
  "name": "www",
  "type": "AAAA",
  "value": "2001:db8::1",
  "ttl": 3600
}
```

### CNAME Record (Canonical Name)

Alias one name to another.

```json
{
  "name": "www",
  "type": "CNAME",
  "value": "example.com.",
  "ttl": 3600
}
```

**Note**: Value must be FQDN ending with dot.

### MX Record (Mail Exchange)

Specifies mail servers for the domain.

```json
{
  "name": "@",
  "type": "MX",
  "value": "mail.example.com.",
  "priority": 10,
  "ttl": 3600
}
```

Lower priority values are preferred.

### TXT Record (Text)

Arbitrary text data, often used for SPF, DKIM, verification.

```json
{
  "name": "@",
  "type": "TXT",
  "value": "v=spf1 mx -all",
  "ttl": 3600
}
```

### NS Record (Nameserver)

Delegates a subdomain to other nameservers.

```json
{
  "name": "subdomain",
  "type": "NS",
  "value": "ns1.other.com.",
  "ttl": 86400
}
```

### PTR Record (Pointer)

Reverse DNS lookup.

```json
{
  "name": "1",
  "type": "PTR",
  "value": "host.example.com.",
  "ttl": 3600
}
```

### SRV Record (Service)

Specifies location of services.

```json
{
  "name": "_service._proto",
  "type": "SRV",
  "value": "0 5 5060 sipserver.example.com.",
  "priority": 10,
  "ttl": 3600
}
```

Format: `priority weight port target`

### CAA Record (Certification Authority Authorization)

Specifies which CAs can issue certificates.

```json
{
  "name": "@",
  "type": "CAA",
  "value": "0 issue \"letsencrypt.org\"",
  "ttl": 86400
}
```

## Examples

### Basic Website

```json
{
  "records": [
    {
      "name": "@",
      "type": "A",
      "value": "192.0.2.1"
    },
    {
      "name": "www",
      "type": "A",
      "value": "192.0.2.1"
    }
  ]
}
```

### Website with Email

```json
{
  "records": [
    {
      "name": "@",
      "type": "A",
      "value": "192.0.2.1"
    },
    {
      "name": "www",
      "type": "CNAME",
      "value": "example.com."
    },
    {
      "name": "@",
      "type": "MX",
      "value": "mail.example.com.",
      "priority": 10
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
```

### Subdomain Delegation

```json
{
  "records": [
    {
      "name": "sub",
      "type": "NS",
      "value": "ns1.subdomain.example.com."
    },
    {
      "name": "sub",
      "type": "NS",
      "value": "ns2.subdomain.example.com."
    }
  ]
}
```

## Best Practices

1. **Use appropriate TTLs** - Lower for frequently changing records
2. **FQDN formatting** - Always end with dot for absolute names
3. **Multiple MX records** - For redundancy
4. **SPF records** - Prevent email spoofing
5. **CAA records** - Restrict certificate issuance

## Next Steps

- [Zone Configuration](./zone-config.md) - Complete zone configuration
- [Managing Zones](../managing-zones/index.md) - Modify zones after creation
