# Reloading Zones

Reload zones after making changes to zone files.

## When to Reload

Reload a zone when:
- Zone file has been modified
- Records have been added/removed/changed
- SOA serial has been incremented
- After manual edits to zone files

## Reload Zone

**POST** `/api/v1/zones/{name}/reload`

### Request

```bash
curl -X POST http://localhost:8080/api/v1/zones/example.com/reload \
  -H "Authorization: Bearer $TOKEN"
```

### Response

```json
{
  "success": true,
  "message": "Zone example.com reloaded successfully",
  "details": "zone example.com/IN: loaded serial 2024010102"
}
```

### Status Codes

- `200 OK` - Zone reloaded successfully
- `401 Unauthorized` - Missing/invalid token
- `404 Not Found` - Zone doesn't exist
- `500 Internal Server Error` - RNDC reload command failed

## Workflow

### 1. Modify Zone File

Edit the zone file on the shared volume:

```bash
# Example: Add a new A record
echo "newhost IN A 192.0.2.10" >> /var/cache/bind/example.com.zone

# Increment SOA serial
# Edit SOA record to increment serial number
```

### 2. Reload via API

```bash
curl -X POST http://localhost:8080/api/v1/zones/example.com/reload \
  -H "Authorization: Bearer $TOKEN"
```

### 3. Verify

```bash
# Check zone status
curl http://localhost:8080/api/v1/zones/example.com/status \
  -H "Authorization: Bearer $TOKEN"

# Or query DNS directly
dig @localhost example.com newhost.example.com
```

## Best Practices

1. **Always increment SOA serial** - Before reloading
2. **Validate zone file syntax** - Use `named-checkzone` if available
3. **Test in development first** - Before production changes
4. **Monitor logs** - Check for reload errors
5. **Verify changes** - Query DNS after reload

## Troubleshooting

### Reload Fails

```json
{
  "error": "RndcError",
  "message": "rndc: 'reload' failed: not found"
}
```

**Solution**: Verify zone name is correct and zone exists.

### Zone File Syntax Error

```json
{
  "error": "RndcError",
  "message": "zone example.com/IN: loading from master file failed: syntax error"
}
```

**Solution**: Check zone file syntax with `named-checkzone`.

### Permission Denied

```json
{
  "error": "RndcError",
  "message": "rndc: reload: permission denied"
}
```

**Solution**: Verify file permissions on zone file.

## Next Steps

- [Zone Status](./zone-status.md) - Check zone status
- [Deleting Zones](./deleting-zones.md) - Remove zones
