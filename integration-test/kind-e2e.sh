#!/usr/bin/env bash
# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT
#
# bindcar end-to-end regression test ON a kind (Kubernetes-in-Docker) cluster.
#
# Deploys a single Pod running BIND9 (unprivileged port 5353) + the bindcar
# sidecar (0.0.0.0:8080 with a shared-secret BIND_API_TOKEN, exercising the
# non-loopback auth startup guard), then drives the same API surface the drone
# test covers, in-cluster:
#   1. GET  /api/v1/health, /api/v1/ready
#   2. POST /api/v1/zones                       — create primary zone foo.bar
#   3. dig  foo.bar SOA                          — present
#   4. POST /api/v1/zones/foo.bar/records        — add A test.foo.bar -> 1.2.3.4
#   5. dig  test.foo.bar A                        — present
#   6. DELETE record + zone, dig                  — absent
# Also applies deploy/rbac.yaml + deploy/networkpolicy.yaml so the shipped
# manifests are validated by a real API server (server-side admission).
#
# Requires: docker, kind, kubectl, curl, dig, openssl.
#
# Image: builds docker/Dockerfile.local into $BINDCAR_IMAGE and kind-loads it.
#   Override with BINDCAR_IMAGE=... and SKIP_IMAGE_BUILD=true to reuse an image.
#
# The kind cluster is created fresh and deleted on exit (KEEP_CLUSTER=true keeps
# it for debugging).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

KIND_CLUSTER="${KIND_CLUSTER:-bindcar-e2e}"
NS="${E2E_NAMESPACE:-bindcar-e2e}"
BINDCAR_IMAGE="${BINDCAR_IMAGE:-bindcar:e2e}"
BIND9_IMAGE="${BIND9_IMAGE:-internetsystemsconsortium/bind9:9.18}"
RNDC_KEY_NAME="rndc-key"
RNDC_ALGORITHM="hmac-sha256"
API_PORT=8080
DNS_PORT=5353
POD="bindcar-e2e"

API_TOKEN="" ; RNDC_SECRET="" ; PF_API_PID="" ; PF_DNS_PID="" ; SUCCESS="false"

log()  { echo "[$(date '+%H:%M:%S')] $*"; }
pass() { echo "[$(date '+%H:%M:%S')] ✓  $*"; }
fail() { echo "[$(date '+%H:%M:%S')] ✗  $*" >&2; dump_diag; exit 1; }

dump_diag() {
  echo "=== diagnostics ===" >&2
  kubectl -n "$NS" get pod "$POD" -o wide >&2 2>/dev/null || true
  echo "--- container states ---" >&2
  kubectl -n "$NS" get pod "$POD" \
    -o jsonpath='{range .status.containerStatuses[*]}{.name}: ready={.ready} started={.started} state={.state}{"\n"}{end}' >&2 2>/dev/null || true
  kubectl -n "$NS" describe pod "$POD" 2>/dev/null | sed -n '/Events:/,$p' >&2 || true
  for c in bind9 bindcar; do
    echo "--- logs: $c (current) ---" >&2
    kubectl -n "$NS" logs "$POD" -c "$c" --tail=80 >&2 2>/dev/null || true
    echo "--- logs: $c (previous, if restarted) ---" >&2
    kubectl -n "$NS" logs "$POD" -c "$c" --previous --tail=40 >&2 2>/dev/null || true
  done
}

cleanup() {
  log "--- cleanup ---"
  [[ -n "$PF_API_PID" ]] && kill "$PF_API_PID" 2>/dev/null || true
  [[ -n "$PF_DNS_PID" ]] && kill "$PF_DNS_PID" 2>/dev/null || true
  if [[ "${KEEP_CLUSTER:-false}" == "true" ]]; then
    log "KEEP_CLUSTER=true — leaving cluster '$KIND_CLUSTER' running"
  elif [[ "$SUCCESS" == "true" ]]; then
    # On success, always tear down (whether we created or reused the cluster) so
    # the e2e leaves no state behind. Set KEEP_CLUSTER=true to keep it.
    log "test passed — deleting kind cluster '$KIND_CLUSTER'"
    kind delete cluster --name "$KIND_CLUSTER" 2>/dev/null || true
  else
    # On failure, leave the cluster up for investigation.
    log "test FAILED — leaving kind cluster '$KIND_CLUSTER' up for investigation:"
    log "    kubectl --context kind-$KIND_CLUSTER -n $NS get pods"
    log "    kubectl --context kind-$KIND_CLUSTER -n $NS logs $POD -c bindcar"
    log "    kubectl --context kind-$KIND_CLUSTER -n $NS logs $POD -c bind9"
    log "  delete when done:  kind delete cluster --name $KIND_CLUSTER"
  fi
}
trap cleanup EXIT INT TERM

check_deps() {
  local missing=()
  for c in docker kind kubectl curl dig openssl; do
    command -v "$c" >/dev/null 2>&1 || missing+=("$c")
  done
  [[ ${#missing[@]} -eq 0 ]] || fail "missing required tools: ${missing[*]}"
  docker info >/dev/null 2>&1 || fail "docker daemon is not running"
}

curl_assert() {
  # curl_assert LABEL EXPECTED_STATUS [curl args...] — token added automatically.
  local label="$1" expected="$2"; shift 2
  local actual
  actual=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer ${API_TOKEN}" "$@")
  [[ "$actual" == "$expected" ]] || fail "${label}: expected HTTP ${expected}, got ${actual}"
  pass "${label}: HTTP ${actual}"
}

dig_assert() {
  # dig_assert LABEL FQDN TYPE EXPECTED  ("" = absent, "*" = any, else exact)
  local label="$1" fqdn="$2" type="$3" expected="$4" actual
  actual=$(dig @127.0.0.1 -p "$DNS_PORT" +tcp +short "$fqdn" "$type" 2>/dev/null || true)
  if [[ -z "$expected" ]]; then
    [[ -z "$actual" ]] || fail "${label}: expected no record, got '${actual}'"
    pass "${label}: absent (as expected)"
  elif [[ "$expected" == "*" ]]; then
    [[ -n "$actual" ]] || fail "${label}: expected a record but got empty"
    pass "${label}: present → ${actual}"
  else
    [[ "$actual" == "$expected" ]] || fail "${label}: expected '${expected}', got '${actual:-<empty>}'"
    pass "${label}: ${actual}"
  fi
}

echo ""
echo "══════════════════════════════════════════════════════"
echo "  bindcar kind e2e regression test"
echo "══════════════════════════════════════════════════════"
echo ""

check_deps

# --- [1/9] Obtain the bindcar image -----------------------------------------
# Local dev builds Dockerfile.local; CI passes SKIP_IMAGE_BUILD=true + a
# BINDCAR_IMAGE that an earlier pipeline stage already built and pushed, so the
# e2e reuses the exact artifact under test instead of rebuilding.
if [[ "${SKIP_IMAGE_BUILD:-false}" != "true" ]]; then
  log "[1/9] building bindcar Linux image ($BINDCAR_IMAGE) via Dockerfile.chef"
  # Compile inside the builder image so the binary is a Linux ELF for the build
  # platform. Dockerfile.local just COPYs the HOST binary, which on macOS is a
  # Darwin Mach-O executable -> 'exec format error' in the Linux kind node.
  # Dockerfile.chef builds a statically-linked musl binary for the node arch.
  docker build -f docker/Dockerfile.chef -t "$BINDCAR_IMAGE" .
else
  log "[1/9] SKIP_IMAGE_BUILD=true — reusing pre-built image $BINDCAR_IMAGE"
fi
# Ensure the image is in the local docker daemon (pull a registry image built by
# a prior CI stage) so `kind load docker-image` can find it.
if ! docker image inspect "$BINDCAR_IMAGE" >/dev/null 2>&1; then
  log "image not present locally — pulling $BINDCAR_IMAGE"
  docker pull "$BINDCAR_IMAGE"
fi

# --- [2/9] Create kind cluster ----------------------------------------------
if kind get clusters 2>/dev/null | grep -qx "$KIND_CLUSTER"; then
  log "[2/9] reusing existing kind cluster $KIND_CLUSTER"
else
  log "[2/9] creating kind cluster $KIND_CLUSTER"
  kind create cluster --name "$KIND_CLUSTER" --wait 300s
fi
kubectl cluster-info --context "kind-${KIND_CLUSTER}" >/dev/null || fail "kind cluster not reachable"

# kind's --wait can time out on slow machines while the node still comes up
# moments later; gate explicitly on node readiness before deploying anything.
log "waiting for node Ready"
kubectl wait --for=condition=Ready nodes --all --timeout=300s || fail "kind node did not become Ready"

log "loading image into kind"
kind load docker-image "$BINDCAR_IMAGE" --name "$KIND_CLUSTER"

# --- [3/9] Namespace + secrets ----------------------------------------------
log "[3/9] creating namespace and credentials"
kubectl create namespace "$NS" --dry-run=client -o yaml | kubectl apply -f -

# Use an explicit ServiceAccount rather than the namespace's 'default' SA. The
# default SA is provisioned asynchronously by a controller that can be slow or
# starved on a loaded machine (pods are then rejected with "serviceaccount
# default not found"). A SA we apply exists synchronously via the API server.
log "creating e2e ServiceAccount"
kubectl -n "$NS" create serviceaccount e2e-runner --dry-run=client -o yaml | kubectl apply -f -

RNDC_SECRET="$(openssl rand -base64 32)"
API_TOKEN="$(openssl rand -hex 24)"

# BIND9 config (named + control channel + dynamic zones), all in one Secret
# since it carries the TSIG key. Zone dir is a shared emptyDir at /var/cache/bind.
kubectl -n "$NS" create secret generic bind9-config \
  --dry-run=client -o yaml \
  --from-literal=secret="${RNDC_SECRET}" \
  --from-literal=rndc.key="key \"${RNDC_KEY_NAME}\" { algorithm ${RNDC_ALGORITHM}; secret \"${RNDC_SECRET}\"; };" \
  --from-literal=named.conf="include \"/etc/bind/rndc.key\";
controls { inet 127.0.0.1 port 953 allow { 127.0.0.1; } keys { \"${RNDC_KEY_NAME}\"; }; };
include \"/etc/bind/named.conf.options\";
include \"/etc/bind/named.conf.local\";" \
  --from-literal=named.conf.options="options {
    directory \"/var/cache/bind\";
    listen-on port ${DNS_PORT} { any; };
    listen-on-v6 { none; };
    allow-query { any; };
    allow-transfer { none; };
    allow-update { any; };
    allow-new-zones yes;
    recursion no;
    dnssec-validation no;
};" \
  --from-literal=named.conf.local="// zones managed dynamically by bindcar" \
  | kubectl apply -f -

kubectl -n "$NS" create secret generic bindcar-api-token \
  --dry-run=client -o yaml --from-literal=token="$API_TOKEN" | kubectl apply -f -

# --- [4/9] Deploy the bind9 + bindcar pod -----------------------------------
log "[4/9] deploying bind9 + bindcar pod"
# On a reused cluster the pod would be "unchanged" and keep running the OLD
# image (same tag) and the OLD secret values injected at its creation. Delete it
# first so BOTH containers are recreated with the freshly-loaded image and the
# regenerated TSIG/API-token secrets.
kubectl -n "$NS" delete pod "$POD" --ignore-not-found --wait --timeout=90s
kubectl -n "$NS" apply -f - <<YAML
apiVersion: v1
kind: Pod
metadata:
  name: ${POD}
  labels: { app: bindcar-e2e }
spec:
  serviceAccountName: e2e-runner
  automountServiceAccountToken: false
  # Always (not Never): both containers start simultaneously, so if bindcar
  # briefly loses the race for bind9's RNDC port at startup, the kubelet restarts
  # it until bind9 is up — image-agnostic (no shell wrapper needed for distroless).
  restartPolicy: Always
  volumes:
    - name: bind-config
      secret: { secretName: bind9-config }
    - name: zone-dir
      emptyDir: {}
  # Make the shared zone dir writable by both the bindcar user and named's
  # 'bind' user (mirrors the drone test's chmod 777 on the shared volume).
  initContainers:
    - name: chmod-zone-dir
      image: busybox:1.36
      command: ["sh", "-c", "chmod 0777 /var/cache/bind"]
      volumeMounts:
        - { name: zone-dir, mountPath: /var/cache/bind }
  containers:
    - name: bind9
      image: ${BIND9_IMAGE}
      # Reuse the image once pulled — avoids re-pulling on every restart, which
      # is flaky on a loaded machine / slow network.
      imagePullPolicy: IfNotPresent
      # The ISC image ENTRYPOINT is 'named -u bind' (no foreground flag), so we
      # add -g: run named in the foreground (required for a container) and log to
      # stderr so kubectl logs shows its output. This overrides the default CMD,
      # which would log to a file via -f.
      args: ["-g", "-c", "/etc/bind/named.conf"]
      volumeMounts:
        - { name: bind-config, mountPath: /etc/bind }
        - { name: zone-dir, mountPath: /var/cache/bind }
    - name: bindcar
      image: ${BINDCAR_IMAGE}
      imagePullPolicy: IfNotPresent
      command: ["/usr/local/bin/bindcar", "drone"]
      # Share the zone dir with named so the file bindcar writes is visible to
      # named at the same absolute path used by rndc addzone.
      volumeMounts:
        - { name: zone-dir, mountPath: /var/cache/bind }
      env:
        - { name: RNDC_SERVER,       value: "127.0.0.1:953" }
        - { name: RNDC_KEY_NAME,     value: "${RNDC_KEY_NAME}" }
        - { name: RNDC_ALGORITHM,    value: "${RNDC_ALGORITHM}" }
        - name: RNDC_SECRET
          valueFrom: { secretKeyRef: { name: bind9-config, key: secret } }
        - { name: NSUPDATE_SERVER,   value: "127.0.0.1" }
        - { name: NSUPDATE_PORT,     value: "${DNS_PORT}" }
        - { name: NSUPDATE_TCP,      value: "true" }
        - { name: NSUPDATE_KEY_NAME, value: "${RNDC_KEY_NAME}" }
        - { name: NSUPDATE_ALGORITHM, value: "${RNDC_ALGORITHM}" }
        - name: NSUPDATE_SECRET
          valueFrom: { secretKeyRef: { name: bind9-config, key: secret } }
        - { name: BIND_ZONE_DIR,     value: "/var/cache/bind" }
        - { name: API_PORT,          value: "${API_PORT}" }
        # Real shared-secret auth: bindcar binds 0.0.0.0, so the non-loopback
        # startup guard requires this (Mode A). The test sends it as Bearer.
        - name: BIND_API_TOKEN
          valueFrom: { secretKeyRef: { name: bindcar-api-token, key: token } }
        - { name: RUST_LOG, value: "info" }
      ports:
        - { containerPort: ${API_PORT} }
      readinessProbe:
        httpGet: { path: /api/v1/health, port: ${API_PORT} }
        initialDelaySeconds: 2
        periodSeconds: 2
YAML

# --- [5/9] Wait for readiness -----------------------------------------------
log "[5/9] waiting for pod Ready (fail-fast on container error)"
deadline=$((SECONDS + 240))
while :; do
  ready=$(kubectl -n "$NS" get pod "$POD" \
    -o jsonpath='{.status.conditions[?(@.type=="Ready")].status}' 2>/dev/null || true)
  [[ "$ready" == "True" ]] && break
  # Fail fast (with logs) on a container terminated non-zero...
  terminated=$(kubectl -n "$NS" get pod "$POD" \
    -o jsonpath='{range .status.containerStatuses[*]}{.name}={.state.terminated.exitCode} {end}' 2>/dev/null || true)
  if echo "$terminated" | grep -Eq '=[1-9]'; then
    fail "a container terminated with a non-zero exit code: ${terminated}"
  fi
  # ...or persistently crash-looping (a real error, not the transient startup
  # race that restartPolicy:Always self-heals). >=5 restarts = give up + show logs.
  restarts=$(kubectl -n "$NS" get pod "$POD" \
    -o jsonpath='{range .status.containerStatuses[*]}{.name}={.restartCount} {end}' 2>/dev/null || true)
  if echo "$restarts" | grep -Eq '=[5-9]|=[1-9][0-9]'; then
    fail "a container is crash-looping (restarts: ${restarts})"
  fi
  phase=$(kubectl -n "$NS" get pod "$POD" -o jsonpath='{.status.phase}' 2>/dev/null || true)
  [[ "$phase" == "Failed" ]] && fail "pod entered Failed phase"
  (( SECONDS >= deadline )) && fail "pod did not become Ready within 240s"
  sleep 3
done
pass "pod is Ready"

# --- [6/9] Validate shipped manifests server-side ---------------------------
log "[6/9] server-side validation of deploy/ manifests"
# rbac.yaml / networkpolicy.yaml are namespaced to bindy-system; create it so the
# server-side dry-run can resolve those namespaced references.
kubectl create namespace bindy-system --dry-run=client -o yaml | kubectl apply -f - >/dev/null
kubectl apply --dry-run=server -f deploy/rbac.yaml >/dev/null \
  || fail "deploy/rbac.yaml failed server-side validation"
pass "deploy/rbac.yaml admits (server dry-run)"
kubectl apply --dry-run=server -f deploy/networkpolicy.yaml >/dev/null \
  || fail "deploy/networkpolicy.yaml failed server-side validation"
pass "deploy/networkpolicy.yaml admits (server dry-run)"

# --- [7/9] Port-forward ------------------------------------------------------
log "[7/9] port-forwarding API :${API_PORT} and DNS :${DNS_PORT}"
kubectl -n "$NS" port-forward "pod/${POD}" "${API_PORT}:${API_PORT}" >/dev/null 2>&1 &
PF_API_PID=$!
kubectl -n "$NS" port-forward "pod/${POD}" "${DNS_PORT}:${DNS_PORT}" >/dev/null 2>&1 &
PF_DNS_PID=$!

for _ in $(seq 1 20); do
  curl -sf -H "Authorization: Bearer ${API_TOKEN}" "http://127.0.0.1:${API_PORT}/api/v1/health" >/dev/null 2>&1 && break
  sleep 1
done
curl_assert "GET /api/v1/health" "200" "http://127.0.0.1:${API_PORT}/api/v1/health"
curl_assert "GET /api/v1/ready"  "200" "http://127.0.0.1:${API_PORT}/api/v1/ready"

# Auth regression: a request WITHOUT the token must be rejected.
noauth=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:${API_PORT}/api/v1/zones")
[[ "$noauth" == "401" ]] || fail "unauthenticated GET /api/v1/zones: expected 401, got ${noauth}"
pass "unauthenticated request rejected: HTTP 401"

# --- [8/9] Zone + record lifecycle ------------------------------------------
log "[8/9] zone + record lifecycle"
curl_assert "POST /api/v1/zones (foo.bar)" "201" \
  -X POST "http://127.0.0.1:${API_PORT}/api/v1/zones" \
  -H "Content-Type: application/json" \
  -d "{\"zoneName\":\"foo.bar\",\"zoneType\":\"primary\",\"updateKeyName\":\"${RNDC_KEY_NAME}\",\"zoneConfig\":{\"ttl\":3600,\"soa\":{\"primaryNs\":\"ns1.foo.bar.\",\"adminEmail\":\"admin.foo.bar.\"},\"nameServers\":[\"ns1.foo.bar.\"],\"nameServerIps\":{\"ns1.foo.bar.\":\"127.0.0.1\"},\"records\":[]}}"
sleep 1
dig_assert "foo.bar SOA present" "foo.bar" "SOA" "*"

curl_assert "POST /records (A test.foo.bar)" "201" \
  -X POST "http://127.0.0.1:${API_PORT}/api/v1/zones/foo.bar/records" \
  -H "Content-Type: application/json" \
  -d '{"name":"test","type":"A","value":"1.2.3.4","ttl":3600}'
sleep 1
dig_assert "test.foo.bar A present" "test.foo.bar" "A" "1.2.3.4"

# SRV — a multi-field type; exercises the native master-file RData parser.
curl_assert "POST /records (SRV _sip._tcp)" "201" \
  -X POST "http://127.0.0.1:${API_PORT}/api/v1/zones/foo.bar/records" \
  -H "Content-Type: application/json" \
  -d '{"name":"_sip._tcp","type":"SRV","value":"10 5 5060 sipserver.foo.bar.","ttl":3600}'
sleep 1
dig_assert "_sip._tcp.foo.bar SRV present" "_sip._tcp.foo.bar" "SRV" "10 5 5060 sipserver.foo.bar."

# CAA — a quoted value; exercises native master-file quoting.
curl_assert "POST /records (CAA foo.bar)" "201" \
  -X POST "http://127.0.0.1:${API_PORT}/api/v1/zones/foo.bar/records" \
  -H "Content-Type: application/json" \
  -d '{"name":"@","type":"CAA","value":"0 issue \"letsencrypt.org\"","ttl":3600}'
sleep 1
dig_assert "foo.bar CAA present" "foo.bar" "CAA" '0 issue "letsencrypt.org"'

# --- [9/9] Teardown of the zone ---------------------------------------------
log "[9/9] delete record + zone, verify removal"
curl_assert "DELETE /records (A)" "200" \
  -X DELETE "http://127.0.0.1:${API_PORT}/api/v1/zones/foo.bar/records" \
  -H "Content-Type: application/json" \
  -d '{"name":"test","type":"A","value":"1.2.3.4"}'
sleep 1
dig_assert "test.foo.bar A absent" "test.foo.bar" "A" ""

curl_assert "DELETE /records (SRV)" "200" \
  -X DELETE "http://127.0.0.1:${API_PORT}/api/v1/zones/foo.bar/records" \
  -H "Content-Type: application/json" \
  -d '{"name":"_sip._tcp","type":"SRV","value":"10 5 5060 sipserver.foo.bar."}'
sleep 1
dig_assert "_sip._tcp.foo.bar SRV absent" "_sip._tcp.foo.bar" "SRV" ""

curl_assert "DELETE /records (CAA)" "200" \
  -X DELETE "http://127.0.0.1:${API_PORT}/api/v1/zones/foo.bar/records" \
  -H "Content-Type: application/json" \
  -d '{"name":"@","type":"CAA","value":"0 issue \"letsencrypt.org\""}'
sleep 1
dig_assert "foo.bar CAA absent" "foo.bar" "CAA" ""

curl_assert "DELETE /api/v1/zones/foo.bar" "200" \
  -X DELETE "http://127.0.0.1:${API_PORT}/api/v1/zones/foo.bar"
sleep 1
dig_assert "foo.bar SOA absent" "foo.bar" "SOA" ""

SUCCESS="true"
echo ""
echo "══════════════════════════════════════════════════════"
echo "  kind e2e regression test PASSED"
echo "══════════════════════════════════════════════════════"
echo ""
