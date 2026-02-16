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

## DNS Record Management

### Add, Update, and Remove Individual Records

```bash
#!/bin/bash
set -e

TOKEN="your-secret-token"
BASE_URL="http://localhost:8080/api/v1"
ZONE="example.com"

# Prerequisites: Zone must be created with dynamic updates enabled
echo "Creating zone with dynamic updates enabled..."
curl -X POST "$BASE_URL/zones" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"zoneName\": \"$ZONE\",
    \"zoneType\": \"primary\",
    \"zoneConfig\": {
      \"ttl\": 3600,
      \"soa\": {
        \"primaryNs\": \"ns1.$ZONE.\",
        \"adminEmail\": \"admin.$ZONE.\",
        \"serial\": 1,
        \"refresh\": 3600,
        \"retry\": 1800,
        \"expire\": 604800,
        \"negativeTtl\": 86400
      },
      \"updateKeyName\": \"update-key\",
      \"records\": [
        {\"name\": \"@\", \"type\": \"NS\", \"value\": \"ns1.$ZONE.\"}
      ]
    }
  }"

# 1. Add A record
echo -e "\n\n1. Adding A record for www..."
curl -X POST "$BASE_URL/zones/$ZONE/records" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "A",
    "value": "192.0.2.100",
    "ttl": 3600
  }'

# 2. Add another A record (same name, different IP)
echo -e "\n\n2. Adding second A record for www (load balancing)..."
curl -X POST "$BASE_URL/zones/$ZONE/records" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "A",
    "value": "192.0.2.101",
    "ttl": 3600
  }'

# 3. Add MX record
echo -e "\n\n3. Adding MX record..."
curl -X POST "$BASE_URL/zones/$ZONE/records" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "@",
    "type": "MX",
    "value": "mail.example.com.",
    "ttl": 3600,
    "priority": 10
  }'

# 4. Update an A record
echo -e "\n\n4. Updating A record (changing IP)..."
curl -X PUT "$BASE_URL/zones/$ZONE/records" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "A",
    "currentValue": "192.0.2.100",
    "newValue": "192.0.2.102",
    "ttl": 7200
  }'

# 5. Remove specific A record
echo -e "\n\n5. Removing specific A record..."
curl -X DELETE "$BASE_URL/zones/$ZONE/records" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "www",
    "type": "A",
    "value": "192.0.2.101"
  }'

# 6. Verify with dig
echo -e "\n\n6. Verifying DNS records..."
dig @localhost www.$ZONE +short
dig @localhost $ZONE MX +short

echo -e "\n\nDone!"
```

### Python: Dynamic Record Management

```python
#!/usr/bin/env python3
import requests
from typing import Optional

BASE_URL = "http://localhost:8080/api/v1"
TOKEN = "your-secret-token"

headers = {
    "Authorization": f"Bearer {TOKEN}",
    "Content-Type": "application/json"
}

class RecordManager:
    """Manage individual DNS records dynamically."""

    def __init__(self, zone_name: str):
        self.zone_name = zone_name
        self.base_path = f"{BASE_URL}/zones/{zone_name}/records"

    def add_record(self, name: str, record_type: str, value: str,
                   ttl: int = 3600, priority: Optional[int] = None):
        """Add a DNS record to the zone."""
        data = {
            "name": name,
            "type": record_type,
            "value": value,
            "ttl": ttl
        }
        if priority is not None:
            data["priority"] = priority

        response = requests.post(self.base_path, headers=headers, json=data)

        if response.status_code == 201:
            print(f"✓ Added {record_type} record: {name} -> {value}")
            return response.json()
        else:
            print(f"✗ Failed to add record: {response.text}")
            return None

    def remove_record(self, name: str, record_type: str, value: str):
        """Remove a specific DNS record."""
        data = {
            "name": name,
            "type": record_type,
            "value": value
        }

        response = requests.delete(self.base_path, headers=headers, json=data)

        if response.status_code == 200:
            print(f"✓ Removed {record_type} record: {name} ({value})")
            return response.json()
        else:
            print(f"✗ Failed to remove record: {response.text}")
            return None

    def update_record(self, name: str, record_type: str,
                      current_value: str, new_value: str, ttl: int = 3600):
        """Update an existing DNS record."""
        data = {
            "name": name,
            "type": record_type,
            "currentValue": current_value,
            "newValue": new_value,
            "ttl": ttl
        }

        response = requests.put(self.base_path, headers=headers, json=data)

        if response.status_code == 200:
            print(f"✓ Updated {record_type} record: {name} {current_value} -> {new_value}")
            return response.json()
        else:
            print(f"✗ Failed to update record: {response.text}")
            return None

# Example usage
manager = RecordManager("example.com")

# Add web servers
manager.add_record("web1", "A", "192.0.2.10")
manager.add_record("web2", "A", "192.0.2.11")
manager.add_record("web3", "A", "192.0.2.12")

# Add round-robin DNS
manager.add_record("www", "A", "192.0.2.10")
manager.add_record("www", "A", "192.0.2.11")
manager.add_record("www", "A", "192.0.2.12")

# Update one server
manager.update_record("web1", "A", "192.0.2.10", "192.0.2.20")

# Remove a failed server from rotation
manager.remove_record("www", "A", "192.0.2.11")

# Add CNAME
manager.add_record("app", "CNAME", "web1.example.com.")
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

    curl -X POST "$BASE_URL/zones/$ZONE_NAME/records" \
      -H "Authorization: Bearer $TOKEN" \
      -H "Content-Type: application/json" \
      -d "{
        \"name\": \"$LAST_OCTET\",
        \"type\": \"PTR\",
        \"value\": \"$HOSTNAME.\",
        \"ttl\": 3600
      }"
}

# Create reverse zones for IP ranges
create_reverse_zone "192.0.2"
create_reverse_zone "192.0.3"

# Add PTR records using dynamic updates
add_ptr_record "192.0.2.1" "ns1.example.com"
add_ptr_record "192.0.2.10" "web1.example.com"
add_ptr_record "192.0.2.20" "mail.example.com"
```

## High Availability DNS Setup

### Primary-Secondary Zone Replication

Configure zone transfers between primary and secondary DNS servers for high availability:

```bash
#!/bin/bash
# create-ha-zone.sh

TOKEN="your-secret-token"
BASE_URL="http://localhost:8080/api/v1"

# Secondary DNS server IPs
SECONDARY_IPS=("10.244.2.101" "10.244.2.102")

echo "Creating HA zone with automatic replication..."

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
        "retry": 600,
        "expire": 604800,
        "negativeTtl": 86400
      },
      "nameServers": ["ns1.example.com.", "ns2.example.com."],
      "nameServerIps": {
        "ns1.example.com.": "10.244.1.101",
        "ns2.example.com.": "10.244.2.101"
      },
      "records": [
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
      ],
      "alsoNotify": ["10.244.2.101", "10.244.2.102"],
      "allowTransfer": ["10.244.2.101", "10.244.2.102"]
    }
  }'

echo -e "\n\nZone created with automatic notification to secondary servers:"
echo "  - Secondary 1: 10.244.2.101"
echo "  - Secondary 2: 10.244.2.102"
echo ""
echo "Zone transfers are allowed from these IPs"
echo "Secondaries will be notified when the zone changes"
```

### Python Helper for Multi-Primary Setup

```python
#!/usr/bin/env python3
"""
Create primary zones with automatic secondary notifications in Kubernetes.
"""
import requests
from kubernetes import client, config

# Load Kubernetes config
config.load_incluster_config()
v1 = client.CoreV1Api()

BINDCAR_URL = "http://bindcar-service:8080/api/v1"
with open("/var/run/secrets/kubernetes.io/serviceaccount/token") as f:
    TOKEN = f.read().strip()

headers = {
    "Authorization": f"Bearer {TOKEN}",
    "Content-Type": "application/json"
}

def get_secondary_ips(namespace="default", label_selector="app=bind9,role=secondary"):
    """Get IPs of all secondary BIND9 pods."""
    pods = v1.list_namespaced_pod(namespace, label_selector=label_selector)
    return [pod.status.pod_ip for pod in pods.items if pod.status.pod_ip]

def create_ha_zone(domain, records, primary_ns_ip):
    """Create a zone configured for high availability."""

    # Discover secondary servers
    secondary_ips = get_secondary_ips()

    if not secondary_ips:
        print("⚠️  Warning: No secondary servers found")

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
                "retry": 600,
                "expire": 604800,
                "negativeTtl": 86400
            },
            "nameServers": [f"ns1.{domain}.", f"ns2.{domain}."],
            "nameServerIps": {
                f"ns1.{domain}.": primary_ns_ip,
                f"ns2.{domain}.": secondary_ips[0] if secondary_ips else "127.0.0.2"
            },
            "records": records,
            "alsoNotify": secondary_ips,
            "allowTransfer": secondary_ips
        }
    }

    response = requests.post(
        f"{BINDCAR_URL}/zones",
        headers=headers,
        json=zone_data
    )

    if response.status_code == 201:
        print(f"✓ Created HA zone: {domain}")
        print(f"  Primary NS: {primary_ns_ip}")
        print(f"  Secondaries: {', '.join(secondary_ips) if secondary_ips else 'none'}")
    else:
        print(f"✗ Failed to create {domain}: {response.text}")

    return response

# Example: Create production zone with HA
create_ha_zone(
    domain="prod.example.com",
    records=[
        {"name": "@", "type": "A", "value": "192.0.2.1"},
        {"name": "www", "type": "A", "value": "192.0.2.10"},
        {"name": "api", "type": "A", "value": "192.0.2.20"},
    ],
    primary_ns_ip="10.244.1.101"
)
```

### Kubernetes StatefulSet with Zone Transfers

```yaml
# bind9-ha-statefulset.yaml
apiVersion: v1
kind: Service
metadata:
  name: bind9-primary
spec:
  selector:
    app: bind9
    role: primary
  ports:
  - name: dns-tcp
    port: 53
    protocol: TCP
  - name: dns-udp
    port: 53
    protocol: UDP
  - name: rndc
    port: 953
---
apiVersion: v1
kind: Service
metadata:
  name: bind9-secondary
spec:
  selector:
    app: bind9
    role: secondary
  ports:
  - name: dns-tcp
    port: 53
    protocol: TCP
  - name: dns-udp
    port: 53
    protocol: UDP
---
# Primary BIND9 server
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: bind9-primary
spec:
  serviceName: bind9-primary
  replicas: 1
  selector:
    matchLabels:
      app: bind9
      role: primary
  template:
    metadata:
      labels:
        app: bind9
        role: primary
    spec:
      containers:
      - name: bind9
        image: ubuntu/bind9:latest
        ports:
        - containerPort: 53
          name: dns-tcp
          protocol: TCP
        - containerPort: 53
          name: dns-udp
          protocol: UDP
        - containerPort: 953
          name: rndc
        volumeMounts:
        - name: zones
          mountPath: /var/cache/bind
        - name: rndc-key
          mountPath: /etc/bind/rndc.key
          subPath: rndc.key

      - name: bindcar
        image: ghcr.io/firestoned/bindcar:latest
        ports:
        - containerPort: 8080
          name: api
        volumeMounts:
        - name: zones
          mountPath: /var/cache/bind
        - name: rndc-key
          mountPath: /etc/bind/rndc.key
          subPath: rndc.key

      volumes:
      - name: zones
        emptyDir: {}
      - name: rndc-key
        secret:
          secretName: rndc-key
---
# Secondary BIND9 servers
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: bind9-secondary
spec:
  serviceName: bind9-secondary
  replicas: 2
  selector:
    matchLabels:
      app: bind9
      role: secondary
  template:
    metadata:
      labels:
        app: bind9
        role: secondary
    spec:
      containers:
      - name: bind9
        image: ubuntu/bind9:latest
        ports:
        - containerPort: 53
          name: dns-tcp
          protocol: TCP
        - containerPort: 53
          name: dns-udp
          protocol: UDP
        volumeMounts:
        - name: zones
          mountPath: /var/cache/bind
        - name: config
          mountPath: /etc/bind/named.conf.local
          subPath: named.conf.local

      volumes:
      - name: zones
        emptyDir: {}
      - name: config
        configMap:
          name: bind9-secondary-config
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: bind9-secondary-config
data:
  named.conf.local: |
    // Secondary zones will be automatically added via zone transfers
    // from primary server
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

## Zone Configuration Updates

### Modifying Zone Transfer Settings

Update `also-notify` and `allow-transfer` settings without recreating the zone.

```bash
#!/bin/bash
set -e

TOKEN="your-secret-token"
BASE_URL="http://localhost:8080/api/v1"
ZONE="example.com"

# Add secondary DNS servers to also-notify list
echo "Adding secondary servers to also-notify..."
curl -X PATCH "$BASE_URL/zones/$ZONE" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alsoNotify": ["10.244.2.101", "10.244.2.102"],
    "allowTransfer": ["10.244.2.101", "10.244.2.102"]
  }'

echo -e "\n\nVerifying zone configuration..."
curl "$BASE_URL/zones/$ZONE/status" \
  -H "Authorization: Bearer $TOKEN"

echo -e "\n\nNotifying secondary servers..."
curl -X POST "$BASE_URL/zones/$ZONE/notify" \
  -H "Authorization: Bearer $TOKEN"

echo -e "\n\nDone!"
```

### Python: Update Zone Transfer Configuration

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

def modify_zone_transfer_config(zone_name, also_notify=None, allow_transfer=None):
    """Update zone transfer configuration."""
    url = f"{BASE_URL}/zones/{zone_name}"

    data = {}
    if also_notify is not None:
        data["alsoNotify"] = also_notify
    if allow_transfer is not None:
        data["allowTransfer"] = allow_transfer

    if not data:
        print("Error: At least one field must be provided")
        return None

    response = requests.patch(url, headers=headers, json=data)

    if response.status_code == 200:
        result = response.json()
        print(f"✓ Modified zone: {zone_name}")
        print(f"  Message: {result.get('message')}")
        return result
    else:
        print(f"✗ Failed to modify {zone_name}: {response.text}")
        return None

# Example 1: Add secondary servers
print("Example 1: Add secondary servers")
modify_zone_transfer_config(
    zone_name="example.com",
    also_notify=["10.244.2.101", "10.244.2.102"],
    allow_transfer=["10.244.2.101", "10.244.2.102"]
)

# Example 2: Update only also-notify
print("\nExample 2: Update only also-notify")
modify_zone_transfer_config(
    zone_name="example.com",
    also_notify=["10.244.2.101", "10.244.2.102", "10.244.2.103"]
)

# Example 3: Clear also-notify
print("\nExample 3: Clear also-notify")
modify_zone_transfer_config(
    zone_name="example.com",
    also_notify=[]
)

# Example 4: IPv6 addresses
print("\nExample 4: IPv6 addresses")
modify_zone_transfer_config(
    zone_name="example.com",
    also_notify=["2001:db8::1", "2001:db8::2"],
    allow_transfer=["2001:db8::1", "2001:db8::2"]
)
```

### Automated Secondary Server Management

```python
#!/usr/bin/env python3
import requests
import json
from typing import List

BASE_URL = "http://localhost:8080/api/v1"
TOKEN = "your-secret-token"

headers = {
    "Authorization": f"Bearer {TOKEN}",
    "Content-Type": "application/json"
}

class ZoneManager:
    """Manage zone transfer configurations."""

    def __init__(self, base_url: str, token: str):
        self.base_url = base_url
        self.headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json"
        }

    def get_secondary_servers(self) -> List[str]:
        """Get list of secondary DNS servers from your infrastructure."""
        # In production, this would query your infrastructure
        # For example: Kubernetes API, Consul, etcd, etc.
        return [
            "10.244.2.101",
            "10.244.2.102",
            "10.244.2.103"
        ]

    def update_zone_secondaries(self, zone_name: str) -> bool:
        """Update zone to use current secondary servers."""
        secondaries = self.get_secondary_servers()

        if not secondaries:
            print(f"No secondary servers found for {zone_name}")
            return False

        url = f"{self.base_url}/zones/{zone_name}"
        data = {
            "alsoNotify": secondaries,
            "allowTransfer": secondaries
        }

        response = requests.patch(url, headers=self.headers, json=data)

        if response.status_code == 200:
            print(f"✓ Updated {zone_name} with {len(secondaries)} secondaries")
            return True
        else:
            print(f"✗ Failed to update {zone_name}: {response.text}")
            return False

    def sync_all_zones(self) -> None:
        """Update all zones with current secondary servers."""
        # Get all zones
        response = requests.get(
            f"{self.base_url}/zones",
            headers=self.headers
        )

        if response.status_code != 200:
            print(f"Failed to list zones: {response.text}")
            return

        zones = response.json().get("zones", [])
        print(f"Found {len(zones)} zones to update")

        success_count = 0
        for zone in zones:
            if self.update_zone_secondaries(zone):
                success_count += 1

        print(f"\n✓ Updated {success_count}/{len(zones)} zones")

# Usage
if __name__ == "__main__":
    manager = ZoneManager(BASE_URL, TOKEN)

    # Option 1: Update a specific zone
    manager.update_zone_secondaries("example.com")

    # Option 2: Sync all zones (useful for automation)
    # manager.sync_all_zones()
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

- [API Reference](./api.md) - Complete API documentation
- [Troubleshooting](../operations/troubleshooting.md) - Common issues and solutions
- [Contributing](../developer-guide/contributing.md) - Contribute your own examples
