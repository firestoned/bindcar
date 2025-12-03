# Managing Zones

Operations for managing existing DNS zones.

## Available Operations

- **Reload Zone** - Reload zone from file
- **Delete Zone** - Remove zone from BIND9
- **Zone Status** - Get zone information from BIND9
- **Freeze Zone** - Disable dynamic updates
- **Thaw Zone** - Enable dynamic updates
- **Notify Secondaries** - Trigger zone transfer

## Quick Reference

| Operation | Endpoint | Method |
|-----------|----------|--------|
| Reload | `/api/v1/zones/{name}/reload` | POST |
| Delete | `/api/v1/zones/{name}` | DELETE |
| Status | `/api/v1/zones/{name}/status` | GET |
| Freeze | `/api/v1/zones/{name}/freeze` | POST |
| Thaw | `/api/v1/zones/{name}/thaw` | POST |
| Notify | `/api/v1/zones/{name}/notify` | POST |

## Common Workflows

### Update Zone Content

1. Modify zone file directly on shared volume
2. Reload zone via API
3. Verify changes with status endpoint

See [Reloading Zones](./reloading-zones.md)

### Remove Zone

1. Verify zone exists
2. Delete via API
3. Confirm deletion

See [Deleting Zones](./deleting-zones.md)

### Check Zone Health

Query zone status to verify:
- Zone is loaded
- Serial number
- Zone type
- Last modified time

See [Zone Status](./zone-status.md)

## Next Steps

- [Reloading Zones](./reloading-zones.md)
- [Deleting Zones](./deleting-zones.md)
- [Zone Status](./zone-status.md)
