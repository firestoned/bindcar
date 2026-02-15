# Deleting Zones

Remove zones from BIND9.

## Delete Zone

**DELETE** `/api/v1/zones/{name}`

Removes the zone from BIND9 and deletes the zone file.

### Request

```bash
curl -X DELETE http://localhost:8080/api/v1/zones/example.com \
  -H "Authorization: Bearer $TOKEN"
```

### Response

```json
{
  "success": true,
  "message": "Zone example.com deleted successfully",
  "details": "zone 'example.com' was deleted"
}
```

### Status Codes

- `200 OK` - Zone deleted successfully
- `401 Unauthorized` - Missing/invalid token
- `404 Not Found` - Zone doesn't exist
- `500 Internal Server Error` - RNDC delete command failed

## What Gets Deleted

When you delete a zone, bindcar automatically cleans up all related files:

1. **Zone removal from BIND9** - Executes `rndc delzone` to remove the zone from memory
2. **Zone file deletion** - Removes the `.zone` file from the filesystem (e.g., `example.com.zone`)
3. **Journal file cleanup** - Automatically removes BIND9 journal files (`.zone.jnl`) to prevent "journal out of sync" errors on zone recreation

This ensures complete cleanup and prevents any stale files from interfering with future zone operations.

## Workflow

### 1. List Zones

Check which zones exist:

```bash
curl http://localhost:8080/api/v1/zones \
  -H "Authorization: Bearer $TOKEN"
```

### 2. Verify Zone Info

Get zone details before deletion:

```bash
curl http://localhost:8080/api/v1/zones/example.com \
  -H "Authorization: Bearer $TOKEN"
```

### 3. Delete Zone

```bash
curl -X DELETE http://localhost:8080/api/v1/zones/example.com \
  -H "Authorization: Bearer $TOKEN"
```

### 4. Confirm Deletion

```bash
# Should return 404
curl http://localhost:8080/api/v1/zones/example.com \
  -H "Authorization: Bearer $TOKEN"
```

## Important Notes

- **Deletion is immediate** - No confirmation prompt
- **Not reversible** - Zone file is deleted
- **Backup first** - If you might need the zone later
- **No cascade** - Parent/child zones are independent

## Safety Recommendations

### Create Backup

```bash
# Before deletion, backup zone file
kubectl exec -it dns-pod -c bind9 -- \
  cp /var/cache/bind/example.com.zone /backup/

# Or via Docker
docker cp bind9:/var/cache/bind/example.com.zone ./backup/
```

### Export Zone Data

```bash
# Get zone configuration for re-creation
curl http://localhost:8080/api/v1/zones/example.com \
  -H "Authorization: Bearer $TOKEN" > zone-backup.json
```

## Troubleshooting

### Zone Not Found

```json
{
  "error": "ZoneNotFound",
  "message": "Zone 'example.com' does not exist"
}
```

**Solution**: Verify zone name spelling and check zone list.

### Permission Denied

```json
{
  "error": "RndcError",
  "message": "rndc: delzone: permission denied"
}
```

**Solution**: Check BIND9 permissions and rndc configuration.

## Next Steps

- [Creating Zones](./creating-zones.md) - Create new zones
- [Zone Status](./zone-status.md) - Check zone information
