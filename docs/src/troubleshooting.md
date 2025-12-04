# Troubleshooting

Common issues and solutions when deploying and using bindcar.

## Quick Diagnostics

### Check Service Health

```bash
# Health check
curl http://localhost:8080/api/v1/health

# Readiness check
curl http://localhost:8080/api/v1/ready

# Server status (requires auth)
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/server/status
```

### Check Logs

```bash
# Docker
docker logs bindcar --tail 100 --follow

# Kubernetes
kubectl logs -l app=dns -c bindcar --tail 100 --follow

# Filter for errors
kubectl logs -l app=dns -c bindcar | jq 'select(.level=="error")'
```

## Authentication Issues

### 401 Unauthorized - Missing Token

**Symptom**:
```json
{
  "error": "Authentication required",
  "details": "Missing Authorization header"
}
```

**Solution**:
```bash
# Ensure Bearer token is provided
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/zones
```

**Verify Token Format**:
- Must start with "Bearer "
- Token follows after space
- No extra whitespace

### 401 Unauthorized - Invalid Token

**Symptom**:
```json
{
  "error": "Authentication failed",
  "details": "Invalid Bearer token"
}
```

**Causes**:
1. Token not in BIND_ALLOWED_TOKENS
2. Token expired (ServiceAccount tokens)
3. Typo in token value

**Solution - Docker**:
```bash
# Set token in environment
docker run -e BIND_ALLOWED_TOKENS="your-secret-token" \
  ghcr.io/firestoned/bindcar:latest
```

**Solution - Kubernetes**:
```bash
# Create new token
TOKEN=$(kubectl create token bindcar-client --duration=24h)

# Use fresh token
curl -H "Authorization: Bearer $TOKEN" \
  http://bindcar:8080/api/v1/zones
```

**Verify Token Configuration**:
```bash
# Check environment variable
kubectl exec -it dns-pod -c bindcar -- env | grep BIND_ALLOWED_TOKENS
```

## RNDC Command Failures

### 502 Bad Gateway - Connection Refused

**Symptom**:
```json
{
  "error": "RNDC command failed",
  "details": "rndc: connect failed: connection refused"
}
```

**Causes**:
1. BIND9 not running
2. RNDC not configured
3. Wrong BIND9 container

**Solution - Check BIND9**:
```bash
# Kubernetes
kubectl exec -it dns-pod -c bind9 -- ps aux | grep named

# Docker
docker exec bind9 ps aux | grep named

# Check BIND9 logs
kubectl logs dns-pod -c bind9 --tail 50
```

**Solution - Verify RNDC**:
```bash
# Test RNDC manually
kubectl exec -it dns-pod -c bindcar -- rndc status

# Check rndc.key exists
kubectl exec -it dns-pod -c bindcar -- ls -la /etc/bind/rndc.key
```

**Solution - Kubernetes Sidecar**:
Ensure BIND9 and bindcar are in the same pod:
```yaml
spec:
  containers:
  - name: bind9
    image: ubuntu/bind9:latest
  - name: bindcar
    image: ghcr.io/firestoned/bindcar:latest
```

### 502 Bad Gateway - Permission Denied

**Symptom**:
```json
{
  "error": "RNDC command failed",
  "details": "rndc: 'addzone' failed: permission denied"
}
```

**Causes**:
1. RNDC key mismatch
2. RNDC controls not configured in named.conf
3. SELinux/AppArmor restrictions

**Solution - Verify RNDC Key**:
```bash
# Check key is shared between containers
kubectl exec dns-pod -c bind9 -- cat /etc/bind/rndc.key
kubectl exec dns-pod -c bindcar -- cat /etc/bind/rndc.key

# Keys must match!
```

**Solution - Check named.conf**:
```bind
# /etc/bind/named.conf should include:
controls {
    inet 127.0.0.1 allow { localhost; } keys { "rndc-key"; };
};

include "/etc/bind/rndc.key";
```

**Solution - Verify Permissions**:
```bash
# RNDC key should be readable
kubectl exec dns-pod -c bindcar -- ls -la /etc/bind/rndc.key
# Should show: -rw-r--r-- or similar
```

### 409 Conflict - Zone Already Exists

**Symptom**:
```json
{
  "error": "Zone already exists",
  "zone": "example.com"
}
```

**Causes**:
1. Zone was previously created
2. Zone exists in named.conf as static zone
3. Previous failed deletion

**Solution - Delete Existing**:
```bash
# Delete zone first
curl -X DELETE \
  -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/zones/example.com

# Then recreate
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @zone.json \
  http://localhost:8080/api/v1/zones
```

**Solution - Check Static Zones**:
```bash
# Check if zone is in named.conf
kubectl exec dns-pod -c bind9 -- grep "example.com" /etc/bind/named.conf

# Static zones cannot be managed via RNDC
# Remove from named.conf and restart BIND9
```

### 502 Bad Gateway - Zone File Syntax Error

**Symptom**:
```json
{
  "error": "RNDC command failed",
  "details": "zone example.com/IN: loading from master file failed: syntax error"
}
```

**Causes**:
1. Invalid DNS record format
2. Missing SOA record
3. Invalid characters in zone file

**Solution - Validate Zone Data**:
```bash
# Check zone file syntax
kubectl exec dns-pod -c bind9 -- \
  named-checkzone example.com /var/cache/bind/db.example.com
```

**Solution - Fix Request Data**:
Ensure zone creation request has valid SOA:
```json
{
  "zoneName": "example.com",
  "zoneType": "master",
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
    }
  }
}
```

**Common Mistakes**:
- Missing trailing dots in FQDN: `ns1.example.com.`
- Invalid email format: Use `admin.example.com.` not `admin@example.com`
- Invalid TTL values: Must be positive integers

## Zone File Issues

### Zone Directory Not Writable

**Symptom**:
```json
{
  "ready": false,
  "checks": {
    "zone_directory": "failed"
  }
}
```

**Causes**:
1. Wrong permissions on zone directory
2. Directory doesn't exist
3. Container running as wrong user

**Solution - Check Permissions**:
```bash
# Check directory exists and is writable
kubectl exec dns-pod -c bindcar -- ls -ld /var/cache/bind

# Should show: drwxrwxr-x or similar
```

**Solution - Fix Permissions**:
```yaml
# Set correct fsGroup in pod
securityContext:
  fsGroup: 101  # bind group
  runAsUser: 101
  runAsNonRoot: true
```

**Solution - Create Directory**:
```bash
# Create zone directory
kubectl exec dns-pod -c bindcar -- mkdir -p /var/cache/bind

# Set permissions
kubectl exec dns-pod -c bindcar -- chmod 775 /var/cache/bind
```

### Zone File Not Found

**Symptom**:
404 error when accessing zone, but zone was created successfully.

**Causes**:
1. Zone file deleted manually
2. Shared volume not configured
3. Wrong BIND_ZONE_DIR

**Solution - Verify Volume Mount**:
```yaml
# Ensure both containers mount the same volume
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

**Solution - Check Files**:
```bash
# List zone files
kubectl exec dns-pod -c bindcar -- ls -la /var/cache/bind

# Verify BIND9 sees the same files
kubectl exec dns-pod -c bind9 -- ls -la /var/cache/bind
```

## Service Unavailable

### 503 Service Unavailable - Not Ready

**Symptom**:
```json
{
  "ready": false,
  "checks": {
    "zone_directory": "ok",
    "rndc_binary": "failed"
  }
}
```

**Causes**:
1. RNDC binary not found
2. RNDC binary not executable
3. Container image missing dependencies

**Solution - Verify RNDC Binary**:
```bash
# Check rndc exists
kubectl exec dns-pod -c bindcar -- which rndc

# Should output: /usr/sbin/rndc
```

**Solution - Fix Container Image**:
Ensure container includes BIND9 utilities:
```dockerfile
FROM ubuntu:22.04
RUN apt-get update && \
    apt-get install -y bind9-utils && \
    rm -rf /var/lib/apt/lists/*
```

## Docker-Specific Issues

### Container Won't Start

**Symptom**:
Container exits immediately after starting.

**Solution - Check Logs**:
```bash
# View container logs
docker logs bindcar

# Run in foreground to see errors
docker run --rm \
  -e RUST_LOG=debug \
  -v /var/cache/bind:/var/cache/bind \
  ghcr.io/firestoned/bindcar:latest
```

**Common Causes**:
- Missing zone directory mount
- Invalid environment variables
- Port 8080 already in use

### Volume Mount Issues

**Symptom**:
Zone files not persisting, or bindcar can't write files.

**Solution - Verify Mounts**:
```bash
# Check volume is mounted
docker inspect bindcar | jq '.[0].Mounts'

# Should show zone directory mount
```

**Solution - Fix Permissions**:
```bash
# Create directory with correct permissions
mkdir -p /var/cache/bind
chmod 775 /var/cache/bind

# Run with correct user
docker run --user 101:101 \
  -v /var/cache/bind:/var/cache/bind \
  ghcr.io/firestoned/bindcar:latest
```

## Kubernetes-Specific Issues

### Pod CrashLoopBackOff

**Symptom**:
Pod repeatedly crashes and restarts.

**Solution - Check Logs**:
```bash
# Get previous container logs
kubectl logs dns-pod -c bindcar --previous

# Describe pod for events
kubectl describe pod dns-pod
```

**Common Causes**:
- Liveness probe failing too early
- Missing volume mounts
- Missing RNDC key secret
- OOMKilled (out of memory)

**Solution - Adjust Probes**:
```yaml
livenessProbe:
  httpGet:
    path: /api/v1/health
    port: 8080
  initialDelaySeconds: 10  # Increase delay
  failureThreshold: 5      # Allow more failures
```

### Service Not Accessible

**Symptom**:
Can't reach bindcar API from outside the pod.

**Solution - Check Service**:
```bash
# Verify service exists
kubectl get svc bindcar-service

# Check endpoints
kubectl get endpoints bindcar-service

# Port forward for testing
kubectl port-forward svc/bindcar-service 8080:8080
```

**Solution - Verify Selector**:
```yaml
# Service selector must match pod labels
apiVersion: v1
kind: Service
metadata:
  name: bindcar-service
spec:
  selector:
    app: dns  # Must match pod label
  ports:
  - port: 8080
    targetPort: 8080
```

### Secrets Not Mounted

**Symptom**:
RNDC key or tokens not available in container.

**Solution - Verify Secret**:
```bash
# Check secret exists
kubectl get secret rndc-key

# View secret (base64 encoded)
kubectl get secret rndc-key -o yaml
```

**Solution - Fix Mount**:
```yaml
volumes:
- name: rndc-key
  secret:
    secretName: rndc-key
    defaultMode: 0400

containers:
- name: bindcar
  volumeMounts:
  - name: rndc-key
    mountPath: /etc/bind/rndc.key
    subPath: rndc.key
    readOnly: true
```

## Performance Issues

### Slow API Response Times

**Symptom**:
API requests take longer than expected (>100ms).

**Solution - Check RNDC Performance**:
```bash
# Time RNDC commands directly
kubectl exec dns-pod -c bindcar -- time rndc status

# Should complete in <50ms typically
```

**Solution - Check Resource Limits**:
```yaml
resources:
  limits:
    memory: "256Mi"  # May need more
    cpu: "500m"
  requests:
    memory: "128Mi"
    cpu: "100m"
```

**Solution - Enable Debug Logging Temporarily**:
```bash
# Restart with debug logging
kubectl set env deployment/dns RUST_LOG=debug

# Check for slow operations
kubectl logs -l app=dns -c bindcar | \
  jq 'select(.duration_ms > 100)'
```

### High Memory Usage

**Symptom**:
Container OOMKilled or high memory usage.

**Solution - Check Metrics**:
```bash
# Check current usage
kubectl top pod dns-pod

# Review logs for memory issues
kubectl logs dns-pod -c bindcar --previous
```

**Solution - Increase Limits**:
```yaml
resources:
  limits:
    memory: "512Mi"  # Increased
  requests:
    memory: "256Mi"
```

## Getting More Help

### Enable Debug Logging

```bash
# Docker
docker run -e RUST_LOG=debug bindcar:latest

# Kubernetes
kubectl set env deployment/dns RUST_LOG=debug -c bindcar
```

### Collect Diagnostics

```bash
#!/bin/bash
# Save diagnostics to file
{
  echo "=== Service Status ==="
  kubectl get pods -l app=dns
  
  echo -e "\n=== bindcar Logs ==="
  kubectl logs -l app=dns -c bindcar --tail=100
  
  echo -e "\n=== BIND9 Logs ==="
  kubectl logs -l app=dns -c bind9 --tail=100
  
  echo -e "\n=== Pod Describe ==="
  kubectl describe pod -l app=dns
  
  echo -e "\n=== RNDC Test ==="
  kubectl exec -l app=dns -c bindcar -- rndc status
  
  echo -e "\n=== Zone Files ==="
  kubectl exec -l app=dns -c bindcar -- ls -la /var/cache/bind
} > bindcar-diagnostics.txt
```

### Report Issues

If you encounter a bug or need help:

1. Check [GitHub Issues](https://github.com/firestoned/bindcar/issues)
2. Search for similar problems
3. Open a new issue with:
   - bindcar version
   - Deployment method (Docker/Kubernetes)
   - Error messages and logs
   - Steps to reproduce

## Next Steps

- [Monitoring](./monitoring/index.md) - Set up logging and monitoring
- [Contributing](./contributing.md) - Report bugs or contribute fixes
- [Examples](./examples.md) - Working examples and use cases
