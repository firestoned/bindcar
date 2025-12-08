# Examples

Practical examples and complete workflows for common bindcar use cases.

## Complete Zone Lifecycle

### Create, Update, and Delete a Zone

```bash
#!/bin/bash
set -e

TOKEN="your-secret-token"
BASE_URL="http://localhost:8080/api/v1"

# 1. Create zone
echo "Creating zone example.com..."
curl -X POST "$BASE_URL/zones" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "zoneName": "example.com",
    "zoneType": "primary",
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
      },
      "records": [
        {
          "name": "@",
          "type": "NS",
          "value": "ns1.example.com."
        },
        {
          "name": "@",
          "type": "A",
          "value": "192.0.2.1"
        },
        {
          "name": "www",
          "type": "A",
          "value": "192.0.2.10"
        }
      ]
    }
  }'

echo -e "\n\n2. Verify zone was created..."
curl "$BASE_URL/zones/example.com" \
  -H "Authorization: Bearer $TOKEN"

echo -e "\n\n3. Check zone status..."
curl "$BASE_URL/zones/example.com/status" \
  -H "Authorization: Bearer $TOKEN"

echo -e "\n\n4. Reload zone..."
curl -X POST "$BASE_URL/zones/example.com/reload" \
  -H "Authorization: Bearer $TOKEN"

echo -e "\n\n5. List all zones..."
curl "$BASE_URL/zones" \
  -H "Authorization: Bearer $TOKEN"

echo -e "\n\n6. Delete zone..."
curl -X DELETE "$BASE_URL/zones/example.com" \
  -H "Authorization: Bearer $TOKEN"

echo -e "\n\nDone!"
```

## Multi-Zone Setup

### Create Multiple Related Zones

```python
#!/usr/bin/env python3
import requests
import json

BASE_URL = "http://localhost:8080/api/v1"
TOKEN = "your-secret-token"

headers = {
    "Authorization": f"Bearer {TOKEN}",
    "Content-Type": "application/json"
}

def create_zone(domain, records):
    """Create a DNS zone with records."""
    zone_data = {
        "zoneName": domain,
        "zoneType": "primary",
        "zoneConfig": {
            "ttl": 3600,
            "soa": {
                "primaryNs": f"ns1.{domain}.",
                "adminEmail": f"admin.{domain}.",
                "serial": 1,
                "refresh": 3600,
                "retry": 1800,
                "expire": 604800,
                "negativeTtl": 86400
            },
            "records": records
        }
    }
    
    response = requests.post(
        f"{BASE_URL}/zones",
        headers=headers,
        json=zone_data
    )
    
    if response.status_code == 201:
        print(f"✓ Created zone: {domain}")
    else:
        print(f"✗ Failed to create {domain}: {response.text}")
    
    return response

# Create main domain
create_zone("example.com", [
    {"name": "@", "type": "NS", "value": "ns1.example.com."},
    {"name": "@", "type": "A", "value": "192.0.2.1"},
    {"name": "ns1", "type": "A", "value": "192.0.2.10"},
    {"name": "ns2", "type": "A", "value": "192.0.2.11"},
    {"name": "www", "type": "A", "value": "192.0.2.20"},
    {"name": "mail", "type": "A", "value": "192.0.2.30"},
    {"name": "@", "type": "MX", "value": "mail.example.com.", "priority": 10},
    {"name": "@", "type": "TXT", "value": "v=spf1 mx -all"}
])

# Create development subdomain
create_zone("dev.example.com", [
    {"name": "@", "type": "NS", "value": "ns1.example.com."},
    {"name": "@", "type": "A", "value": "192.0.2.100"},
    {"name": "api", "type": "A", "value": "192.0.2.101"},
    {"name": "web", "type": "A", "value": "192.0.2.102"}
])

# Create staging subdomain
create_zone("staging.example.com", [
    {"name": "@", "type": "NS", "value": "ns1.example.com."},
    {"name": "@", "type": "A", "value": "192.0.2.200"},
    {"name": "api", "type": "A", "value": "192.0.2.201"},
    {"name": "web", "type": "A", "value": "192.0.2.202"}
])

# List all zones
response = requests.get(f"{BASE_URL}/zones", headers=headers)
print(f"\nTotal zones: {len(response.json()['zones'])}")
print("Zones:", response.json()['zones'])
```

## Kubernetes Automation

### GitOps Zone Management

```yaml
# zone-creator-job.yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: create-dns-zones
spec:
  template:
    spec:
      serviceAccountName: bindcar-client
      containers:
      - name: zone-creator
        image: curlimages/curl:latest
        command:
        - /bin/sh
        - -c
        - |
          set -e
          
          # Get service account token
          TOKEN=$(cat /var/run/secrets/kubernetes.io/serviceaccount/token)
          BINDCAR_URL="http://bindcar-service:8080/api/v1"
          
          # Function to create zone
          create_zone() {
            DOMAIN=$1
            echo "Creating zone: $DOMAIN"
            
            curl -X POST "$BINDCAR_URL/zones" \
              -H "Authorization: Bearer $TOKEN" \
              -H "Content-Type: application/json" \
              -d "{
                \"zoneName\": \"$DOMAIN\",
                \"zoneType\": \"master\",
                \"zoneConfig\": {
                  \"ttl\": 3600,
                  \"soa\": {
                    \"primaryNs\": \"ns1.$DOMAIN.\",
                    \"adminEmail\": \"admin.$DOMAIN.\",
                    \"serial\": 1,
                    \"refresh\": 3600,
                    \"retry\": 1800,
                    \"expire\": 604800,
                    \"negativeTtl\": 86400
                  },
                  \"records\": [
                    {\"name\": \"@\", \"type\": \"NS\", \"value\": \"ns1.$DOMAIN.\"},
                    {\"name\": \"@\", \"type\": \"A\", \"value\": \"192.0.2.1\"}
                  ]
                }
              }"
          }
          
          # Create zones from list
          create_zone "prod.example.com"
          create_zone "staging.example.com"
          create_zone "dev.example.com"
          
          echo "All zones created successfully!"
      restartPolicy: OnFailure
---
# ServiceAccount with permissions
apiVersion: v1
kind: ServiceAccount
metadata:
  name: bindcar-client
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: bindcar-client-binding
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: bindcar-api-user
subjects:
- kind: ServiceAccount
  name: bindcar-client
```

### Zone Backup CronJob

```yaml
# zone-backup-cronjob.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: dns-zone-backup
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          serviceAccountName: bindcar-client
          containers:
          - name: backup
            image: curlimages/curl:latest
            volumeMounts:
            - name: backup
              mountPath: /backup
            command:
            - /bin/sh
            - -c
            - |
              set -e
              
              TOKEN=$(cat /var/run/secrets/kubernetes.io/serviceaccount/token)
              BINDCAR_URL="http://bindcar-service:8080/api/v1"
              BACKUP_DIR="/backup/$(date +%Y%m%d)"
              
              mkdir -p "$BACKUP_DIR"
              
              # Get list of zones
              ZONES=$(curl -s "$BINDCAR_URL/zones" \
                -H "Authorization: Bearer $TOKEN" | \
                jq -r '.zones[]')
              
              # Backup each zone
              for ZONE in $ZONES; do
                echo "Backing up $ZONE..."
                curl -s "$BINDCAR_URL/zones/$ZONE" \
                  -H "Authorization: Bearer $TOKEN" \
                  > "$BACKUP_DIR/$ZONE.json"
              done
              
              echo "Backup completed: $BACKUP_DIR"
              ls -lh "$BACKUP_DIR"
          volumes:
          - name: backup
            persistentVolumeClaim:
              claimName: dns-backup-pvc
          restartPolicy: OnFailure
```

## Reverse DNS Setup

### Create PTR Records for IP Ranges

```bash
#!/bin/bash
# create-ptr-zones.sh

TOKEN="your-secret-token"
BASE_URL="http://localhost:8080/api/v1"

# Function to create reverse DNS zone
create_reverse_zone() {
    local SUBNET=$1  # e.g., "192.0.2"
    local ZONE_NAME="${SUBNET##*.}.${SUBNET%.*}.in-addr.arpa"
    
    echo "Creating reverse zone: $ZONE_NAME"
    
    curl -X POST "$BASE_URL/zones" \
      -H "Authorization: Bearer $TOKEN" \
      -H "Content-Type: application/json" \
      -d "{
        \"zoneName\": \"$ZONE_NAME\",
        \"zoneType\": \"master\",
        \"zoneConfig\": {
          \"ttl\": 3600,
          \"soa\": {
            \"primaryNs\": \"ns1.example.com.\",
            \"adminEmail\": \"admin.example.com.\",
            \"serial\": 1,
            \"refresh\": 3600,
            \"retry\": 1800,
            \"expire\": 604800,
            \"negativeTtl\": 86400
          },
          \"records\": [
            {\"name\": \"@\", \"type\": \"NS\", \"value\": \"ns1.example.com.\"}
          ]
        }
      }"
}

# Function to add PTR record
add_ptr_record() {
    local IP=$1
    local HOSTNAME=$2
    local SUBNET="${IP%.*}"
    local LAST_OCTET="${IP##*.}"
    local ZONE_NAME="${SUBNET##*.}.${SUBNET%.*}.in-addr.arpa"
    
    echo "Adding PTR: $IP -> $HOSTNAME"
    
    # Note: This example shows the concept
    # Actual implementation would need zone update API endpoint
    echo "Would add PTR record: $LAST_OCTET PTR $HOSTNAME."
}

# Create reverse zones for IP ranges
create_reverse_zone "192.0.2"
create_reverse_zone "192.0.3"

# Add PTR records (conceptual - requires zone update API)
add_ptr_record "192.0.2.1" "ns1.example.com"
add_ptr_record "192.0.2.10" "web1.example.com"
add_ptr_record "192.0.2.20" "mail.example.com"
```

## Dynamic Zone Updates

### Service Discovery Integration

```python
#!/usr/bin/env python3
"""
Update DNS zones based on Kubernetes service discovery.
"""
import requests
from kubernetes import client, config

# Load k8s config
config.load_incluster_config()
v1 = client.CoreV1Api()

# bindcar configuration
BINDCAR_URL = "http://bindcar-service:8080/api/v1"
with open("/var/run/secrets/kubernetes.io/serviceaccount/token") as f:
    TOKEN = f.read().strip()

headers = {
    "Authorization": f"Bearer {TOKEN}",
    "Content-Type": "application/json"
}

def get_service_ips(namespace="default"):
    """Get all service ClusterIPs."""
    services = v1.list_namespaced_service(namespace)
    return {
        svc.metadata.name: svc.spec.cluster_ip
        for svc in services.items
        if svc.spec.cluster_ip not in [None, "None"]
    }

def create_internal_dns_zone():
    """Create internal DNS zone for services."""
    services = get_service_ips()
    
    records = [
        {"name": "@", "type": "NS", "value": "ns1.cluster.local."}
    ]
    
    # Add A record for each service
    for svc_name, svc_ip in services.items():
        records.append({
            "name": svc_name,
            "type": "A",
            "value": svc_ip
        })
    
    zone_data = {
        "zoneName": "services.cluster.local",
        "zoneType": "primary",
        "zoneConfig": {
            "ttl": 30,  # Short TTL for dynamic services
            "soa": {
                "primaryNs": "ns1.cluster.local.",
                "adminEmail": "admin.cluster.local.",
                "serial": 1,
                "refresh": 60,
                "retry": 30,
                "expire": 604800,
                "negativeTtl": 30
            },
            "records": records
        }
    }
    
    # Create or update zone
    response = requests.post(
        f"{BINDCAR_URL}/zones",
        headers=headers,
        json=zone_data
    )
    
    if response.status_code in [201, 409]:  # Created or already exists
        print(f"Zone created/updated with {len(services)} services")
        # Reload zone to pick up changes
        requests.post(
            f"{BINDCAR_URL}/zones/services.cluster.local/reload",
            headers=headers
        )
    else:
        print(f"Error: {response.text}")

if __name__ == "__main__":
    create_internal_dns_zone()
```

## Monitoring Integration

### Prometheus Blackbox Exporter

```yaml
# prometheus-blackbox-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: blackbox-config
data:
  blackbox.yml: |
    modules:
      http_2xx:
        prober: http
        http:
          preferred_ip_protocol: ip4
          valid_status_codes: [200]
      
      http_health:
        prober: http
        http:
          method: GET
          valid_status_codes: [200]
          fail_if_not_matches_regexp:
            - '"healthy":\s*true'
---
# ServiceMonitor for health checks
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: bindcar-health
spec:
  selector:
    matchLabels:
      app: dns
  endpoints:
  - port: api
    path: /api/v1/health
    interval: 30s
---
# PrometheusRule for alerting
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: bindcar-alerts
spec:
  groups:
  - name: bindcar
    interval: 30s
    rules:
    - alert: BindcarDown
      expr: up{job="bindcar"} == 0
      for: 5m
      annotations:
        summary: "bindcar is down"
        description: "bindcar has been down for 5 minutes"
    
    - alert: BindcarHighErrorRate
      expr: |
        rate(http_requests_total{job="bindcar",status=~"5.."}[5m])
        / rate(http_requests_total{job="bindcar"}[5m]) > 0.1
      for: 5m
      annotations:
        summary: "High error rate in bindcar"
        description: "More than 10% of requests are failing"
```

## Testing and Validation

### Integration Test Suite

```bash
#!/bin/bash
# test-bindcar-integration.sh

set -e

TOKEN="test-token"
BASE_URL="http://localhost:8080/api/v1"
FAILED=0

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

test_case() {
    local NAME=$1
    shift
    echo -n "Testing: $NAME... "
    if "$@" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
        ((FAILED++))
    fi
}

# Test health endpoint
test_case "Health check" \
    curl -f -s "$BASE_URL/health"

# Test readiness endpoint
test_case "Readiness check" \
    curl -f -s "$BASE_URL/ready"

# Test authentication required
test_case "Auth required" \
    bash -c "! curl -f -s '$BASE_URL/zones'"

# Test zone creation
test_case "Create zone" \
    curl -f -s -X POST "$BASE_URL/zones" \
        -H "Authorization: Bearer $TOKEN" \
        -H "Content-Type: application/json" \
        -d '{
            "zoneName": "test.example.com",
            "zoneType": "primary",
            "zoneConfig": {
                "ttl": 3600,
                "soa": {
                    "primaryNs": "ns1.test.example.com.",
                    "adminEmail": "admin.test.example.com.",
                    "serial": 1,
                    "refresh": 3600,
                    "retry": 1800,
                    "expire": 604800,
                    "negativeTtl": 86400
                }
            }
        }'

# Test zone listing
test_case "List zones" \
    curl -f -s "$BASE_URL/zones" \
        -H "Authorization: Bearer $TOKEN"

# Test zone retrieval
test_case "Get zone" \
    curl -f -s "$BASE_URL/zones/test.example.com" \
        -H "Authorization: Bearer $TOKEN"

# Test zone reload
test_case "Reload zone" \
    curl -f -s -X POST "$BASE_URL/zones/test.example.com/reload" \
        -H "Authorization: Bearer $TOKEN"

# Test duplicate creation fails
test_case "Duplicate fails" \
    bash -c "! curl -f -s -X POST '$BASE_URL/zones' \
        -H 'Authorization: Bearer $TOKEN' \
        -H 'Content-Type: application/json' \
        -d '{
            \"zoneName\": \"test.example.com\",
            \"zoneType\": \"master\",
            \"zoneConfig\": {
                \"ttl\": 3600,
                \"soa\": {
                    \"primaryNs\": \"ns1.test.example.com.\",
                    \"adminEmail\": \"admin.test.example.com.\",
                    \"serial\": 1,
                    \"refresh\": 3600,
                    \"retry\": 1800,
                    \"expire\": 604800,
                    \"negativeTtl\": 86400
                }
            }
        }'"

# Test zone deletion
test_case "Delete zone" \
    curl -f -s -X DELETE "$BASE_URL/zones/test.example.com" \
        -H "Authorization: Bearer $TOKEN"

# Test deleted zone not found
test_case "Deleted zone 404" \
    bash -c "! curl -f -s '$BASE_URL/zones/test.example.com' \
        -H 'Authorization: Bearer $TOKEN'"

# Summary
echo ""
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}$FAILED tests failed${NC}"
    exit 1
fi
```

## Next Steps

- [API Reference](./api-reference/index.md) - Complete API documentation
- [Troubleshooting](./troubleshooting.md) - Common issues and solutions
- [Contributing](./contributing.md) - Contribute your own examples
