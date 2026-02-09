# DNSSEC Support

Bindcar provides comprehensive support for DNSSEC (DNS Security Extensions) through integration with BIND9's native DNSSEC capabilities. This guide covers how to configure and manage DNSSEC-enabled zones.

## Overview

DNSSEC adds cryptographic signatures to DNS records, providing:
- **Authentication**: Verify that DNS responses come from authoritative sources
- **Data Integrity**: Ensure DNS data hasn't been tampered with in transit
- **Non-existence Proof**: Cryptographically prove that a domain name doesn't exist

Bindcar supports DNSSEC through two key configuration options:
- `dnssec_policy`: Specifies which DNSSEC policy to apply
- `inline_signing`: Enables automatic inline signing by BIND9

## Prerequisites

### BIND9 Version Requirements
- **BIND9 9.16+** is required for `dnssec-policy` support
- Older versions may support manual DNSSEC configuration but are not covered here

### BIND9 DNSSEC Policy Configuration

Before using DNSSEC in Bindcar, you must define DNSSEC policies in your BIND9 configuration (`named.conf` or `named.conf.options`):

```bind
# Example DNSSEC policy definition
dnssec-policy "default" {
    keys {
        ksk lifetime unlimited algorithm ecdsa256;
        zsk lifetime 30d algorithm ecdsa256;
    };

    dnskey-ttl 3600;
    publish-safety PT1H;
    retire-safety PT1H;

    signatures-refresh 5d;
    signatures-validity 14d;
    signatures-validity-dnskey 14d;

    max-zone-ttl 86400;
    zone-propagation-delay 300;
    parent-propagation-delay 3600;

    parent-ds-ttl 86400;
    parent-registration-delay 0;
};

# High security policy example
dnssec-policy "high-security" {
    keys {
        ksk lifetime 365d algorithm ecdsa384;
        zsk lifetime 30d algorithm ecdsa384;
    };

    signatures-validity 7d;
    signatures-validity-dnskey 7d;
};
```

### Required Permissions

Ensure the BIND9 working directory has proper permissions:
```bash
# BIND9 needs write access to store keys and signed zones
chown bind:bind /var/cache/bind
chmod 755 /var/cache/bind
```

## Configuration

### Creating a DNSSEC-Enabled Zone

To create a zone with DNSSEC enabled, include the `dnssecPolicy` and `inlineSigning` fields in your zone creation request:

```bash
curl -X POST http://localhost:8080/api/v1/zones \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "zoneName": "example.com",
    "zoneType": "primary",
    "zoneConfig": {
      "ttl": 3600,
      "soa": {
        "primaryNs": "ns1.example.com.",
        "adminEmail": "admin.example.com.",
        "serial": 2025020601,
        "refresh": 3600,
        "retry": 600,
        "expire": 604800,
        "negativeTtl": 86400
      },
      "nameServers": ["ns1.example.com.", "ns2.example.com."],
      "nameServerIps": {},
      "records": [],
      "dnssecPolicy": "default",
      "inlineSigning": true
    }
  }'
```

### Configuration Fields

#### `dnssecPolicy` (Optional)

Specifies the name of a DNSSEC policy defined in your BIND9 configuration.

- **Type**: String
- **Default**: None (DNSSEC disabled)
- **Example values**: `"default"`, `"high-security"`, `"custom-policy"`
- **Requirements**:
  - Must reference a policy defined in BIND9's `named.conf`
  - Requires BIND9 9.16 or newer
  - Should be used with `inlineSigning: true`

#### `inlineSigning` (Optional)

Enables BIND9's inline signing feature, which automatically signs the zone.

- **Type**: Boolean
- **Default**: `false`
- **When to enable**:
  - **Required** when using `dnssecPolicy`
  - Required for dynamic zones with DNSSEC
  - Recommended for all modern DNSSEC deployments

**How it works**:
- BIND9 maintains two versions: unsigned source zone and signed presentation zone
- Zone updates are applied to the unsigned version
- BIND9 automatically re-signs when changes occur
- No manual signing or key management required

## Usage Examples

### Example 1: Standard DNSSEC Configuration

```json
{
  "zoneName": "secure.example.com",
  "zoneType": "primary",
  "zoneConfig": {
    "ttl": 3600,
    "soa": {
      "primaryNs": "ns1.secure.example.com.",
      "adminEmail": "admin.secure.example.com."
    },
    "nameServers": ["ns1.secure.example.com."],
    "records": [
      {
        "name": "@",
        "recordType": "A",
        "value": "192.0.2.1"
      }
    ],
    "dnssecPolicy": "default",
    "inlineSigning": true
  }
}
```

### Example 2: High-Security DNSSEC Zone

```json
{
  "zoneName": "banking.example.com",
  "zoneType": "primary",
  "zoneConfig": {
    "ttl": 3600,
    "soa": {
      "primaryNs": "ns1.banking.example.com.",
      "adminEmail": "security.banking.example.com."
    },
    "nameServers": ["ns1.banking.example.com.", "ns2.banking.example.com."],
    "records": [],
    "allowTransfer": ["10.1.1.1", "10.1.1.2"],
    "alsoNotify": ["10.1.1.1", "10.1.1.2"],
    "dnssecPolicy": "high-security",
    "inlineSigning": true
  }
}
```

### Example 3: Dynamic Zone with DNSSEC

For zones that receive dynamic updates (e.g., via RFC 2136):

```json
{
  "zoneName": "dynamic.example.com",
  "zoneType": "primary",
  "zoneConfig": {
    "ttl": 300,
    "soa": {
      "primaryNs": "ns1.example.com.",
      "adminEmail": "admin.example.com."
    },
    "nameServers": ["ns1.example.com."],
    "records": [],
    "allowUpdate": ["10.2.2.2"],
    "dnssecPolicy": "default",
    "inlineSigning": true
  },
  "updateKeyName": "update-key"
}
```

## Rust API Usage

### Creating a DNSSEC Zone with the Rust API

```rust
use bindcar::{CreateZoneRequest, ZoneConfig, SoaRecord};
use std::collections::HashMap;

let request = CreateZoneRequest {
    zone_name: "secure.example.com".to_string(),
    zone_type: "primary".to_string(),
    zone_config: ZoneConfig {
        ttl: 3600,
        soa: SoaRecord {
            primary_ns: "ns1.secure.example.com.".to_string(),
            admin_email: "admin.secure.example.com.".to_string(),
            serial: 2025020601,
            refresh: 3600,
            retry: 600,
            expire: 604800,
            negative_ttl: 86400,
        },
        name_servers: vec!["ns1.secure.example.com.".to_string()],
        name_server_ips: HashMap::new(),
        records: vec![],
        also_notify: None,
        allow_transfer: None,
        primaries: None,
        dnssec_policy: Some("default".to_string()),
        inline_signing: Some(true),
    },
    update_key_name: None,
};

// Serialize and send to Bindcar API
let json = serde_json::to_string(&request)?;
```

### Using Bindcar as a Library

```rust
use bindcar::ZoneConfig;

// Create a zone configuration with DNSSEC
let mut zone_config = ZoneConfig {
    // ... other fields ...
    dnssec_policy: Some("default".to_string()),
    inline_signing: Some(true),
    // ... rest of configuration ...
};

// Generate BIND9 zone file
let zone_file = zone_config.to_zone_file();
```

## Verification and Testing

### Verifying DNSSEC Configuration

After creating a DNSSEC-enabled zone, verify it's properly configured:

```bash
# Check zone status
rndc showzone example.com

# Should show:
# zone "example.com" {
#     type primary;
#     file "/var/cache/bind/example.com.zone";
#     dnssec-policy "default";
#     inline-signing yes;
#     ...
# };
```

### Checking DNSSEC Keys

BIND9 automatically generates keys when inline signing is enabled:

```bash
# List generated keys
ls -la /var/cache/bind/K*.key
ls -la /var/cache/bind/K*.private

# Example output:
# Kexample.com.+013+12345.key      (ZSK public)
# Kexample.com.+013+12345.private  (ZSK private)
# Kexample.com.+013+54321.key      (KSK public)
# Kexample.com.+013+54321.private  (KSK private)
```

### Verifying DNSSEC Signatures

Use `dig` to verify DNSSEC records:

```bash
# Query DNSKEY records
dig @localhost DNSKEY example.com +dnssec

# Query for RRSIG records
dig @localhost A www.example.com +dnssec

# Verify DNSSEC validation
dig @localhost example.com +dnssec +multiline
```

### External Validation

Use online DNSSEC validators:
- [DNSViz](https://dnsviz.net/)
- [Verisign DNSSEC Debugger](https://dnssec-debugger.verisignlabs.com/)

## DS Record Publication

For DNSSEC to work properly, you must publish DS (Delegation Signer) records at your parent zone:

### Extract DS Records

```bash
# Get DS records for your zone
dig @localhost DNSKEY example.com | dnssec-dsfromkey -f - example.com

# Or directly from key files
dnssec-dsfromkey /var/cache/bind/Kexample.com.+013+54321.key
```

### Publish to Parent

Provide the DS records to your domain registrar or parent zone operator. The format will be:

```
example.com. IN DS 54321 13 2 ABC123...
```

## Best Practices

### Policy Selection

1. **Use `"default"` for most zones**
   - Balanced security and performance
   - Suitable for general-purpose domains

2. **Use `"high-security"` for sensitive zones**
   - Shorter signature validity
   - More frequent key rotations
   - Higher cryptographic strength

3. **Custom policies for specific requirements**
   - Define your own policies in BIND9 configuration
   - Reference them by name in Bindcar

### Key Management

- **Never commit private keys to version control**
- **Backup private keys securely**
  ```bash
  tar -czf keys-backup-$(date +%Y%m%d).tar.gz /var/cache/bind/K*
  gpg -c keys-backup-*.tar.gz
  ```
- **Store backups in secure, encrypted locations**
- **Automate key rotation** (BIND9 handles this automatically)

### Monitoring

Monitor DNSSEC health:
```bash
# Check signature expiration
rndc dnssec -status example.com

# View DNSSEC statistics
rndc stats
```

### Zone Updates

When updating DNSSEC zones:
- BIND9 automatically re-signs after changes
- Allow time for signature propagation (5-15 minutes)
- Monitor logs for signing errors

## Troubleshooting

### Common Issues

#### 1. "DNSSEC policy not found"

**Symptom**: Zone creation fails with policy error

**Solution**:
- Verify policy exists in BIND9 configuration
- Check policy name spelling (case-sensitive)
- Reload BIND9 after adding policies: `rndc reconfig`

#### 2. Keys Not Generated

**Symptom**: No key files created

**Solution**:
```bash
# Check directory permissions
ls -ld /var/cache/bind
# Should be: drwxr-xr-x bind bind

# Check BIND9 logs
tail -f /var/log/named/named.log | grep -i dnssec
```

#### 3. Signature Validation Failures

**Symptom**: DNS clients report DNSSEC validation errors

**Solution**:
- Verify DS records published at parent
- Check signature validity: `dig +dnssec`
- Ensure system time is synchronized (NTP)

#### 4. Performance Issues

**Symptom**: Slow query responses

**Solution**:
- Reduce signature validity period
- Increase `signatures-refresh` interval
- Consider using NSEC3 instead of NSEC
- Enable query rate limiting

### Debug Logging

Enable DNSSEC debugging in BIND9:

```bind
logging {
    channel dnssec_log {
        file "/var/log/named/dnssec.log" versions 3 size 10m;
        severity debug 3;
        print-time yes;
        print-category yes;
    };

    category dnssec { dnssec_log; };
};
```

## Migration Guide

### Enabling DNSSEC on Existing Zones

To enable DNSSEC on an existing zone:

1. **Verify BIND9 version** (`named -v`)
2. **Define DNSSEC policy** in BIND9 configuration
3. **Use PATCH endpoint** to update zone configuration:

```bash
curl -X PATCH http://localhost:8080/api/v1/zones/example.com \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "dnssecPolicy": "default",
    "inlineSigning": true
  }'
```

> **Note**: The PATCH endpoint currently supports `alsoNotify`, `allowTransfer`, and `allowUpdate`. DNSSEC field updates will be available in a future release. For now, delete and recreate the zone to enable DNSSEC.

### Disabling DNSSEC

To disable DNSSEC on a zone:

1. **Remove DS records** from parent zone first (important!)
2. **Wait for TTL expiration** (typically 24-48 hours)
3. **Recreate zone** without DNSSEC fields
4. **Clean up key files**:
   ```bash
   rm /var/cache/bind/K<zonename>.*
   ```

## Security Considerations

### Key Storage

- Store private keys on encrypted filesystems
- Restrict file permissions: `chmod 600 K*.private`
- Use HSM (Hardware Security Modules) for high-value zones

### Algorithm Selection

Recommended algorithms (BIND9 9.16+):
- **ECDSA P-256 (algorithm 13)**: Good balance, widely supported
- **ECDSA P-384 (algorithm 14)**: Higher security
- **Ed25519 (algorithm 15)**: Modern, efficient (BIND9 9.18+)

Avoid legacy algorithms:
- RSA/SHA-1 (algorithms 5, 7): Deprecated
- DSA (algorithm 3): Weak

### Signature Validity

Balance security and operational flexibility:
- **Production zones**: 7-14 days validity
- **High-security zones**: 3-7 days validity
- **Development zones**: 14-30 days validity

## Performance Impact

DNSSEC adds computational overhead:

### Query Performance
- Additional ~20-50% CPU usage
- Larger response sizes (RRSIG records)
- Increased bandwidth requirements

### Zone Loading
- Longer initial zone load times
- Higher memory usage
- More disk I/O for key operations

### Optimization Tips

1. **Use NSEC3** for zone enumeration protection
2. **Enable query caching** on resolvers
3. **Use aggressive negative caching**
4. **Consider query rate limiting**
5. **Monitor signature refresh timing**

## Related Configuration

DNSSEC works with other Bindcar features:

### With Zone Transfers
```json
{
  "dnssecPolicy": "default",
  "inlineSigning": true,
  "allowTransfer": ["10.1.1.1"],
  "alsoNotify": ["10.1.1.1"]
}
```

### With Dynamic Updates
```json
{
  "dnssecPolicy": "default",
  "inlineSigning": true,
  "allowUpdate": ["key:update-key"]
}
```

### With Secondary Zones

Secondary zones automatically receive DNSSEC signatures via zone transfer:
```json
{
  "zoneType": "secondary",
  "primaries": ["10.1.1.1"]
}
```

## Further Reading

- [BIND9 DNSSEC Guide](https://bind9.readthedocs.io/en/latest/dnssec-guide.html)
- [RFC 4033: DNS Security Introduction](https://www.rfc-editor.org/rfc/rfc4033.html)
- [RFC 4034: Resource Records for DNSSEC](https://www.rfc-editor.org/rfc/rfc4034.html)
- [RFC 4035: Protocol Modifications for DNSSEC](https://www.rfc-editor.org/rfc/rfc4035.html)
- [DNSSEC Best Practices (IETF)](https://datatracker.ietf.org/doc/html/rfc6781)
