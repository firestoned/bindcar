# Kubernetes Deployment

Deploy bindcar to Kubernetes using the sidecar pattern.

## Prerequisites

- Kubernetes 1.24 or later
- kubectl configured with cluster access
- BIND9 container image

## Sidecar Pattern

bindcar runs alongside BIND9 in the same pod, sharing a volume for zone files.

### Architecture

```
┌─────────────────────────────────────┐
│             Pod                      │
│  ┌──────────────┐  ┌─────────────┐ │
│  │    BIND9     │  │  bindcar    │ │
│  │  Container   │  │  Container  │ │
│  │    :53       │  │   :8080     │ │
│  └──────────────┘  └─────────────┘ │
│         │                │          │
│         └────────────────┘          │
│        emptyDir Volume               │
│       /var/cache/bind                │
└─────────────────────────────────────┘
```

## Basic Deployment

### Pod Specification

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: dns-server
  labels:
    app: dns
spec:
  containers:
  # BIND9 Container
  - name: bind9
    image: ubuntu/bind9:latest
    ports:
    - name: dns-tcp
      containerPort: 53
      protocol: TCP
    - name: dns-udp
      containerPort: 53
      protocol: UDP
    volumeMounts:
    - name: zones
      mountPath: /var/cache/bind
    - name: config
      mountPath: /etc/bind

  # bindcar Container
  - name: bindcar
    image: ghcr.io/firestoned/bindcar:latest
    ports:
    - name: api
      containerPort: 8080
      protocol: TCP
    env:
    - name: BIND_ZONE_DIR
      value: "/var/cache/bind"
    - name: API_PORT
      value: "8080"
    - name: RUST_LOG
      value: "info"
    - name: DISABLE_AUTH
      value: "false"
    volumeMounts:
    - name: zones
      mountPath: /var/cache/bind
    livenessProbe:
      httpGet:
        path: /api/v1/health
        port: 8080
      initialDelaySeconds: 5
      periodSeconds: 10
    readinessProbe:
      httpGet:
        path: /api/v1/ready
        port: 8080
      initialDelaySeconds: 5
      periodSeconds: 5

  volumes:
  - name: zones
    emptyDir: {}
  - name: config
    configMap:
      name: bind9-config
```

### Deployment

For production, use a Deployment for replica management:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: dns-server
spec:
  replicas: 1  # BIND9 typically runs as single instance
  selector:
    matchLabels:
      app: dns
  template:
    metadata:
      labels:
        app: dns
    spec:
      containers:
      - name: bind9
        image: ubuntu/bind9:latest
        # ... (same as pod spec)
      - name: bindcar
        image: ghcr.io/firestoned/bindcar:latest
        # ... (same as pod spec)
```

### Service

Expose bindcar API:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: bindcar-api
spec:
  selector:
    app: dns
  ports:
  - name: api
    port: 8080
    targetPort: 8080
    protocol: TCP
  type: ClusterIP
```

Expose BIND9 DNS:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: dns-service
spec:
  selector:
    app: dns
  ports:
  - name: dns-tcp
    port: 53
    protocol: TCP
  - name: dns-udp
    port: 53
    protocol: UDP
  type: LoadBalancer  # or NodePort
```

## Authentication

### ServiceAccount

Create a ServiceAccount for API clients:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: bindcar-client
  namespace: default
```

### Get Token

```bash
# Kubernetes 1.24+
TOKEN=$(kubectl create token bindcar-client)

# Use token
curl -H "Authorization: Bearer $TOKEN" \
  http://bindcar-api:8080/api/v1/zones
```

### RBAC (Optional)

Limit what the bindcar service account can do:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: bindcar
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: bindcar-role
rules:
- apiGroups: [""]
  resources: ["configmaps"]
  verbs: ["get", "list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: bindcar-binding
subjects:
- kind: ServiceAccount
  name: bindcar
roleRef:
  kind: Role
  name: bindcar-role
  apiGroup: rbac.authorization.k8s.io
```

## ConfigMap for BIND9

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: bind9-config
data:
  named.conf: |
    options {
      directory "/var/cache/bind";
      listen-on { any; };
      listen-on-v6 { any; };
      allow-query { any; };
    };

    controls {
      inet 127.0.0.1 allow { localhost; } keys { "rndc-key"; };
    };

    key "rndc-key" {
      algorithm hmac-sha256;
      secret "your-secret-key-here";
    };
```

## Persistent Storage

For production, use PersistentVolumes:

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: bind9-zones
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
---
# In pod spec:
volumes:
- name: zones
  persistentVolumeClaim:
    claimName: bind9-zones
```

## Resource Limits

```yaml
containers:
- name: bindcar
  resources:
    requests:
      cpu: 100m
      memory: 128Mi
    limits:
      cpu: 500m
      memory: 512Mi
```

## Security Context

Run as non-root:

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  fsGroup: 1000
  readOnlyRootFilesystem: true
```

## Network Policies

Restrict traffic to bindcar:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: bindcar-netpol
spec:
  podSelector:
    matchLabels:
      app: dns
  policyTypes:
  - Ingress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: trusted-namespace
    ports:
    - protocol: TCP
      port: 8080
```

## Complete Example

Deploy everything:

```bash
kubectl apply -f - <<EOF
apiVersion: v1
kind: Namespace
metadata:
  name: dns-system
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: bindcar
  namespace: dns-system
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: dns-server
  namespace: dns-system
spec:
  replicas: 1
  selector:
    matchLabels:
      app: dns
  template:
    metadata:
      labels:
        app: dns
    spec:
      serviceAccountName: bindcar
      containers:
      - name: bind9
        image: ubuntu/bind9:latest
        ports:
        - containerPort: 53
          name: dns
        volumeMounts:
        - name: zones
          mountPath: /var/cache/bind
      - name: bindcar
        image: ghcr.io/firestoned/bindcar:latest
        ports:
        - containerPort: 8080
          name: api
        env:
        - name: BIND_ZONE_DIR
          value: "/var/cache/bind"
        - name: RUST_LOG
          value: "info"
        volumeMounts:
        - name: zones
          mountPath: /var/cache/bind
        livenessProbe:
          httpGet:
            path: /api/v1/health
            port: 8080
        readinessProbe:
          httpGet:
            path: /api/v1/ready
            port: 8080
      volumes:
      - name: zones
        emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: bindcar-api
  namespace: dns-system
spec:
  selector:
    app: dns
  ports:
  - port: 8080
    targetPort: 8080
  type: ClusterIP
---
apiVersion: v1
kind: Service
metadata:
  name: dns-service
  namespace: dns-system
spec:
  selector:
    app: dns
  ports:
  - name: dns-tcp
    port: 53
    protocol: TCP
  - name: dns-udp
    port: 53
    protocol: UDP
  type: LoadBalancer
EOF
```

## Verify Deployment

```bash
# Check pods
kubectl get pods -n dns-system

# Check services
kubectl get svc -n dns-system

# Test health
kubectl port-forward -n dns-system svc/bindcar-api 8080:8080
curl http://localhost:8080/api/v1/health

# Check logs
kubectl logs -n dns-system -l app=dns -c bindcar
```

## Troubleshooting

See [Troubleshooting](../troubleshooting.md) for common issues.

## Next Steps

- [Configuration](../configuration/index.md) - Configure bindcar
- [Monitoring](../monitoring/index.md) - Monitor your deployment
- [Troubleshooting](../troubleshooting.md) - Debug issues
