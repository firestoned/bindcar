# RNDC Integration

bindcar integrates with BIND9 through the native RNDC (Remote Name Daemon Control) protocol to manage DNS zones dynamically.

## Architecture Overview

```mermaid
graph TD
    A[HTTP API Request] --> B[bindcar Handler]
    B --> C[RNDC Executor]
    C --> D[rndc crate]
    D --> E[RNDC Protocol<br/>TCP Connection]
    E --> F[BIND9 Server<br/>:953]
    F --> G[Zone Files]

    E --> H{Response}
    H -->|Success| I[Parse Response Text]
    H -->|Error| J[Parse Error Message]
    I --> K[Success Response]
    J --> L[Error Response]

    style C fill:#e1f5ff
    style D fill:#c8e6c9
    style F fill:#e1ffe1
```

## RNDC Command Execution

### Native Protocol Model

bindcar communicates with BIND9 using the native RNDC protocol via the `rndc` crate:

```rust
use rndc::RndcClient;

pub struct RndcExecutor {
    client: RndcClient,
}

impl RndcExecutor {
    pub fn new(server: String, algorithm: String, secret: String) -> Result<Self> {
        let client = RndcClient::new(&server, &algorithm, &secret);
        Ok(Self { client })
    }

    async fn execute(&self, command: &str) -> Result<String> {
        let result = tokio::task::spawn_blocking({
            let client = self.client.clone();
            let command = command.to_string();
            move || client.rndc_command(&command)
        }).await?;

        match result {
            Ok(rndc_result) => {
                if let Some(err) = &rndc_result.err {
                    return Err(anyhow::anyhow!("RNDC command failed: {}", err));
                }
                Ok(rndc_result.text.unwrap_or_default())
            }
            Err(e) => Err(anyhow::anyhow!("RNDC command failed: {}", e))
        }
    }
}
```

### Key Characteristics

- **Native Protocol** - Direct RNDC protocol communication, no subprocess overhead
- **Asynchronous** - Non-blocking command execution using tokio spawn_blocking
- **Authenticated** - HMAC-based authentication with configurable algorithms
- **Error Handling** - Structured error responses from BIND9
- **Efficient** - No process spawning, direct TCP communication
- **Configurable** - Supports environment variables or rndc.conf parsing

### Configuration

bindcar can be configured in two ways:

**Option 1: Environment Variables**

```bash
export RNDC_SERVER="127.0.0.1:953"
export RNDC_ALGORITHM="sha256"
export RNDC_SECRET="dGVzdC1zZWNyZXQtaGVyZQ=="
```

Supported algorithms (with or without `hmac-` prefix):
- `md5` / `hmac-md5`
- `sha1` / `hmac-sha1`
- `sha224` / `hmac-sha224`
- `sha256` / `hmac-sha256`
- `sha384` / `hmac-sha384`
- `sha512` / `hmac-sha512`

**Option 2: Automatic rndc.conf Parsing**

If `RNDC_SECRET` is not set, bindcar automatically parses `/etc/bind/rndc.conf` or `/etc/rndc.conf`:

```conf
# /etc/bind/rndc.conf
include "/etc/bind/rndc.key";

options {
    default-server 127.0.0.1;
    default-key "rndc-key";
};
```

The parser supports `include` directives and will automatically load key files:

```conf
# /etc/bind/rndc.key
key "rndc-key" {
    algorithm hmac-sha256;
    secret "dGVzdC1zZWNyZXQtaGVyZQ==";
};
```

## RNDC Commands Used

```mermaid
graph TD
    Root[RNDC Commands] --> Lifecycle[Zone Lifecycle]
    Root --> State[Zone State]
    Root --> Server[Server Operations]

    Lifecycle --> addzone[addzone]
    Lifecycle --> delzone[delzone]
    Lifecycle --> reload[reload]

    addzone --> adddesc["Create new zone<br/>Add to BIND9 config"]
    delzone --> deldesc["Remove zone<br/>Delete from BIND9"]
    reload --> reloaddesc["Reload zone data<br/>Update records"]

    State --> freeze[freeze]
    State --> thaw[thaw]

    freeze --> freezedesc["Pause updates<br/>Manual editing"]
    thaw --> thawdesc["Resume updates<br/>Re-enable dynamic"]

    Server --> status[status]
    Server --> notify[notify]

    status --> statusdesc["Server info<br/>Zone count"]
    notify --> notifydesc["Trigger transfer<br/>Update secondaries"]

    style Root fill:#e1f5ff
    style Lifecycle fill:#c8e6c9
    style State fill:#fff3e0
    style Server fill:#f3e5f5
```

### addzone

Add a new zone to BIND9 dynamically:

```bash
rndc addzone example.com '{ type primary; file "/var/cache/bind/example.com.zone"; };'
```

**When Used**: POST /api/v1/zones

**Error Scenarios**:
- Zone already exists
- Invalid zone configuration
- Permission denied
- BIND9 not running

### delzone

Remove a zone from BIND9:

```bash
rndc delzone example.com
```

**When Used**: DELETE /api/v1/zones/{name}

**Error Scenarios**:
- Zone does not exist
- Zone is a built-in zone
- Permission denied

### reload

Reload a specific zone:

```bash
rndc reload example.com
```

**When Used**: POST /api/v1/zones/{name}/reload

**Error Scenarios**:
- Zone does not exist
- Zone file syntax error
- Permission denied

### status

Get BIND9 server status:

```bash
rndc status
```

**When Used**: GET /api/v1/server/status

**Returns**:
- BIND9 version
- Number of zones
- Server uptime
- Resource usage

### freeze/thaw

Freeze or thaw dynamic zone updates:

```bash
rndc freeze example.com
rndc thaw example.com
```

**When Used**: 
- POST /api/v1/zones/{name}/freeze
- POST /api/v1/zones/{name}/thaw

**Use Cases**:
- Manual zone file editing
- Backup operations
- Maintenance windows

### notify

Trigger zone transfer to secondary servers:

```bash
rndc notify example.com
```

**When Used**: POST /api/v1/zones/{name}/notify

**Triggers**:
- NOTIFY messages to secondaries
- Zone transfer (AXFR/IXFR)

## Zone File Management

### File Creation Workflow

When a zone is created via API:

```mermaid
flowchart TD
    Start[POST /api/v1/zones] --> Validate{Validate Request}

    Validate -->|Invalid| Error400[400 Bad Request]
    Validate -->|Valid| GenFile[Generate Zone File]

    GenFile --> CheckExists{Check if<br/>file exists}
    CheckExists -->|Exists| Error409[409 Conflict]
    CheckExists -->|Not exists| WriteFile[Write Zone File<br/>to Disk]

    WriteFile --> WriteSuccess{Write<br/>Success?}
    WriteSuccess -->|Fail| Error500[500 Internal Error]
    WriteSuccess -->|Success| ExecRNDC[Execute RNDC<br/>addzone command]

    ExecRNDC --> RNDCSuccess{RNDC<br/>Success?}
    RNDCSuccess -->|Fail| Cleanup[Delete zone file]
    Cleanup --> Error502[502 Bad Gateway]
    RNDCSuccess -->|Success| Success201[201 Created]

    style Error400 fill:#ffe1e1
    style Error409 fill:#ffe1e1
    style Error500 fill:#ffe1e1
    style Error502 fill:#ffe1e1
    style Success201 fill:#e1ffe1
```

### File Creation Steps

1. **Validate Request** - Check zone name, SOA record, NS records, etc.
2. **Generate Zone File** - Create BIND9 format zone file content
3. **Write to Disk** - Save to `BIND_ZONE_DIR/{zone_name}.zone`
4. **Execute addzone** - Register zone with BIND9 via RNDC
5. **Cleanup on Failure** - Remove zone file if RNDC command fails

Example zone file generation:

```bind
$TTL 3600
@       IN      SOA     ns1.example.com. admin.example.com. (
                        2024010101 ; Serial
                        3600       ; Refresh
                        1800       ; Retry
                        604800     ; Expire
                        86400 )    ; Negative TTL

@       IN      NS      ns1.example.com.
@       IN      A       192.0.2.1
```

### File Naming Convention

Zone files are named using the pattern:

```
{zone_name}.zone
```

Examples:
- `example.com.zone`
- `sub.example.com.zone`
- `192.in-addr.arpa.zone`

### Shared Volume Requirements

In sidecar deployments, bindcar and BIND9 must share the zone directory:

```yaml
volumes:
- name: zones
  emptyDir: {}

containers:
- name: bind9
  volumeMounts:
  - name: zones
    mountPath: /var/cache/bind
    
- name: bindcar
  volumeMounts:
  - name: zones
    mountPath: /var/cache/bind
```

## Error Handling

### RNDC Command Failures

bindcar maps RNDC errors to HTTP status codes:

| RNDC Error | HTTP Status | Reason |
|------------|-------------|---------|
| `zone already exists` | 409 Conflict | Zone exists |
| `not found` | 404 Not Found | Zone doesn't exist |
| `permission denied` | 502 Bad Gateway | RNDC permission issue |
| `syntax error` | 502 Bad Gateway | Invalid zone file |
| Connection refused | 502 Bad Gateway | BIND9 not running |

### Error Response Format

```json
{
  "error": "Failed to execute RNDC command",
  "details": "rndc: 'addzone' failed: zone already exists"
}
```

### Logging

RNDC operations are logged at multiple levels:

```json
{
  "level": "info",
  "message": "Executing RNDC command",
  "command": "addzone",
  "zone": "example.com"
}
```

```json
{
  "level": "error",
  "message": "RNDC command failed",
  "command": "addzone",
  "zone": "example.com",
  "error": "zone already exists",
  "exit_code": 1
}
```

## Security Considerations

### RNDC Key Authentication

BIND9 uses `rndc.key` for authentication. In Kubernetes:

```yaml
volumes:
- name: rndc-key
  secret:
    secretName: rndc-key
    
containers:
- name: bind9
  volumeMounts:
  - name: rndc-key
    mountPath: /etc/bind/rndc.key
    subPath: rndc.key
    readOnly: true
    
- name: bindcar
  volumeMounts:
  - name: rndc-key
    mountPath: /etc/bind/rndc.key
    subPath: rndc.key
    readOnly: true
```

### File Permissions

Zone directory must be writable by bindcar:

```yaml
securityContext:
  fsGroup: 101  # bind group
  runAsUser: 101
  runAsNonRoot: true
```

### Command Injection Prevention

bindcar validates all zone names to prevent command injection:

- Alphanumeric characters
- Hyphens
- Dots (for subdomains)
- No shell metacharacters

## Performance Characteristics

### Command Execution Time

Typical RNDC command execution times using native protocol:

- `addzone`: 5-30ms (improved from subprocess approach)
- `delzone`: 5-20ms (improved from subprocess approach)
- `reload`: 3-15ms (improved from subprocess approach)
- `status`: 3-10ms (improved from subprocess approach)

Performance benefits of native protocol:
- No subprocess spawning overhead
- Direct TCP communication
- Efficient binary protocol
- Reduced system call overhead

### Concurrency

bindcar handles multiple concurrent RNDC operations:

- Async/await pattern for non-blocking execution
- Native protocol allows multiple concurrent connections
- No explicit locking required
- BIND9 handles internal synchronization
- `spawn_blocking` prevents blocking the async runtime

### Resource Usage

Native RNDC protocol has minimal overhead:

- No persistent connections (connects per command)
- No subprocess spawning
- Minimal memory footprint
- Direct binary protocol (no stdout/stderr parsing)

## Troubleshooting

### RNDC Connection Issues

**Symptom**: 502 errors, "connection refused" or "Failed to execute RNDC command"

**Causes**:
- BIND9 not running
- BIND9 not listening on port 953
- RNDC key/secret mismatch
- Network connectivity issues
- Incorrect RNDC_SERVER address

**Diagnosis**:
```bash
# Check BIND9 is running
ps aux | grep named

# Check BIND9 is listening on port 953
netstat -tuln | grep 953
# or
ss -tuln | grep 953

# Verify RNDC configuration
cat /etc/bind/rndc.conf

# Test connectivity to RNDC port
nc -zv 127.0.0.1 953

# Check bindcar logs for RNDC errors
docker logs bindcar | grep -i rndc
```

### Permission Errors

**Symptom**: 502 errors, "permission denied"

**Causes**:
- Zone directory not writable
- RNDC key not readable
- SELinux/AppArmor restrictions

**Diagnosis**:
```bash
# Check directory permissions
ls -la /var/cache/bind

# Check RNDC key permissions
ls -la /etc/bind/rndc.key

# Test writing to zone directory
touch /var/cache/bind/test.txt
```

### Zone File Syntax Errors

**Symptom**: Reload fails with syntax error

**Causes**:
- Invalid DNS record format
- Missing SOA record
- Invalid TTL values

**Diagnosis**:
```bash
# Check zone file syntax
named-checkzone example.com /var/cache/bind/db.example.com

# View recent logs
tail -f /var/log/syslog | grep named
```

## Best Practices

1. **Validate Early** - Validate zone data before executing RNDC commands
2. **Log Everything** - Log all RNDC operations for audit trail
3. **Handle Errors Gracefully** - Provide clear error messages to API clients
4. **Monitor RNDC Health** - Use `/api/v1/server/status` to monitor BIND9
5. **Use Timeouts** - Set reasonable timeouts for RNDC command execution
6. **Share Volumes Correctly** - Ensure BIND9 and bindcar can both access zone files
7. **Secure RNDC Keys** - Use Kubernetes secrets for rndc.key in production

## Next Steps

- [API Reference](../api-reference/index.md) - Complete API documentation
- [Troubleshooting](../troubleshooting.md) - Common issues and solutions
- [Examples](../examples.md) - Practical use cases
