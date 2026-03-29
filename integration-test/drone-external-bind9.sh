#!/usr/bin/env bash
# integration-test.sh — bindcar drone mode end-to-end integration test
#
# What this tests:
#   1. BIND9 starts with a fresh TSIG key
#   2. bindcar starts in drone mode, connecting to that BIND9 via RNDC
#   3. POST /api/v1/zones      — create primary zone foo.bar
#   4. POST /api/v1/zones/foo.bar/records — add A record test.foo.bar → 1.2.3.4
#   5. dig @127.0.0.1 test.foo.bar A    — verify the record actually resolves
#
# Requirements:
#   - Docker (for BIND9)
#   - openssl, curl, dig
#   - bindcar binary (built via "cargo build")
#
# Ports used (all unprivileged, no root required):
#   5353/udp+tcp — BIND9 DNS
#   9953/tcp     — BIND9 RNDC
#   8080/tcp     — bindcar API
#
# The zone-file directory is mounted at the SAME absolute path inside the
# Docker container so that the file path bindcar passes to "rndc addzone"
# matches what named expects on the container's filesystem.

set -euo pipefail
#set -x

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BIND9_IMAGE="${BIND9_IMAGE:-ubuntu/bind9:latest}"
CONTAINER_NAME="bindcar-drone-integration-test"
BIND9_DNS_PORT="${BIND9_DNS_PORT:-15353}"
BIND9_RNDC_PORT="${BIND9_RNDC_PORT:-9953}"
BINDCAR_PORT="${BINDCAR_PORT:-8080}"

# Fixed temp paths — same path used by both host (bindcar) and container (named).
# Must be under $HOME on macOS: Docker Desktop only shares /Users by default.
TEST_ROOT="${HOME}/.cache/bindcar-drone-test"
ZONE_DIR="${TEST_ROOT}/zones"
BIND9_CONF_DIR="${TEST_ROOT}/bind9-conf"

RNDC_KEY_NAME="rndc-key"
RNDC_ALGORITHM="hmac-sha256"
BINDCAR_BIN="${BINDCAR_BIN:-${REPO_ROOT}/target/debug/bindcar}"
BINDCAR_PID=""

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

log()  { echo "[$(date '+%H:%M:%S')] $*"; }
pass() { echo "[$(date '+%H:%M:%S')] ✓  $*"; }
fail() { echo "[$(date '+%H:%M:%S')] ✗  $*" >&2; exit 1; }

check_deps() {
    local missing=()
    for cmd in docker openssl curl dig; do
        command -v "$cmd" &>/dev/null || missing+=("$cmd")
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
        fail "missing required tools: ${missing[*]}"
    fi
}

wait_for_tcp() {
    local host="$1" port="$2" label="$3" timeout="${4:-30}"
    for i in $(seq 1 "$timeout"); do
        if curl -sf --connect-timeout 1 "http://${host}:${port}/" &>/dev/null 2>&1 || \
           (echo >/dev/tcp/"$host"/"$port") &>/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    fail "$label did not become reachable on ${host}:${port} within ${timeout}s"
}

wait_for_dns() {
    local port="$1" timeout="${2:-30}"
    for i in $(seq 1 "$timeout"); do
        # +tcp: Docker Desktop on macOS does not forward UDP; TCP is always valid for DNS
        if dig @127.0.0.1 -p "$port" +tcp +time=1 +tries=1 . SOA &>/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    return 1
}

wait_for_bindcar() {
    local port="$1" timeout="${2:-20}"
    for i in $(seq 1 "$timeout"); do
        if curl -sf "http://127.0.0.1:${port}/api/v1/health" &>/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    fail "bindcar API port $port did not respond within ${timeout}s"
}

curl_assert() {
    # Usage: curl_assert LABEL EXPECTED_STATUS [curl args...]
    local label="$1" expected="$2"
    shift 2
    local actual
    actual=$(curl -s -o /dev/null -w "%{http_code}" "$@")
    if [[ "$actual" != "$expected" ]]; then
        fail "${label}: expected HTTP ${expected}, got ${actual}"
    fi
    pass "${label}: HTTP ${actual}"
}

dig_assert() {
    # Usage: dig_assert LABEL FQDN TYPE EXPECTED
    # EXPECTED="" asserts the record is absent; "*" asserts present (any value);
    # any other string asserts the record matches exactly.
    local label="$1" fqdn="$2" type="$3" expected="$4"
    local actual
    actual=$(dig @127.0.0.1 -p "${BIND9_DNS_PORT}" +tcp +short "$fqdn" "$type" 2>/dev/null || true)
    if [[ -z "$expected" ]]; then
        if [[ -n "$actual" ]]; then
            fail "${label}: expected no record, got '${actual}'"
        fi
        pass "${label}: absent (as expected)"
    elif [[ "$expected" == "*" ]]; then
        if [[ -z "$actual" ]]; then
            fail "${label}: expected a record but got empty response"
        fi
        pass "${label}: present → ${actual}"
    else
        if [[ "$actual" != "$expected" ]]; then
            fail "${label}: expected '${expected}', got '${actual:-<empty>}'"
        fi
        pass "${label}: ${actual}"
    fi
}

cleanup() {
    log "--- cleanup ---"
    if [[ -n "$BINDCAR_PID" ]]; then
        kill "$BINDCAR_PID" 2>/dev/null || true
        wait "$BINDCAR_PID" 2>/dev/null || true
    fi
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
    rm -rf "$TEST_ROOT"
}
trap cleanup EXIT INT TERM

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

echo ""
echo "══════════════════════════════════════════════════════"
echo "  bindcar drone integration test"
echo "══════════════════════════════════════════════════════"
echo ""

check_deps

# --- [1/7] Prepare directories ---
log "[1/7] preparing temp directories"
rm -rf "$TEST_ROOT"
mkdir -p "$ZONE_DIR" "$BIND9_CONF_DIR"
# named runs as user 'bind' inside the container; make zone dir world-writable
chmod 777 "$ZONE_DIR"

# --- [2/7] Generate TSIG key ---
log "[2/7] generating TSIG key (openssl rand)"
RNDC_SECRET="$(openssl rand -base64 32)"

# Write key file for named
cat > "${BIND9_CONF_DIR}/rndc.key" <<EOF
key "${RNDC_KEY_NAME}" {
    algorithm ${RNDC_ALGORITHM};
    secret "${RNDC_SECRET}";
};
EOF

# Write named.conf — controls open to any source because Docker NAT changes
# the source IP seen by named (not 127.0.0.1 but the Docker bridge gateway)
cat > "${BIND9_CONF_DIR}/named.conf" <<EOF
include "/etc/bind/rndc.key";

controls {
    inet 0.0.0.0 port 953
        allow { any; }
        keys { "${RNDC_KEY_NAME}"; };
};

include "/etc/bind/named.conf.options";
include "/etc/bind/named.conf.local";
EOF

# Write named.conf.options
# IMPORTANT: directory matches ZONE_DIR so that bindcar's rndc addzone
# file path (absolute, on the host) resolves correctly inside the container
# (which mounts the same directory at the same path).
cat > "${BIND9_CONF_DIR}/named.conf.options" <<EOF
options {
    directory "${ZONE_DIR}";

    // Port 53 is occupied by Docker Desktop's embedded DNS resolver; use 5353.
    // Use { any; } not { 0.0.0.0; } — named binds to real interface IPs, not the
    // wildcard address; 0.0.0.0 matches no interface and results in no DNS listener.
    listen-on port 5353 { any; };
    listen-on-v6 { none; };

    allow-query   { any; };
    allow-transfer { none; };

    // Open for integration testing — Docker NAT makes source IPs unpredictable
    allow-update  { any; };

    // Required for bindcar rndc addzone / delzone
    allow-new-zones yes;

    recursion no;
    dnssec-validation no;
};
EOF

# Empty local zones file
cat > "${BIND9_CONF_DIR}/named.conf.local" <<EOF
// Zones managed dynamically by bindcar via rndc addzone
EOF

# --- [3/7] Start BIND9 ---
log "[3/7] starting BIND9 ($BIND9_IMAGE)"
docker rm -f "$CONTAINER_NAME" 2>/dev/null || true

docker run -d \
    --name  "$CONTAINER_NAME" \
    -p      "${BIND9_DNS_PORT}:5353/udp" \
    -p      "${BIND9_DNS_PORT}:5353/tcp" \
    -p      "${BIND9_RNDC_PORT}:953/tcp" \
    -v      "${BIND9_CONF_DIR}:/etc/bind:ro" \
    -v      "${ZONE_DIR}:${ZONE_DIR}" \
    "$BIND9_IMAGE" \
    -c /etc/bind/named.conf \
    >/dev/null

# Give named a moment to either start or crash on a bad config
sleep 2
if ! docker ps --filter "name=${CONTAINER_NAME}" --filter "status=running" -q | grep -q .; then
    log "=== named failed to start — container logs ==="
    docker logs "$CONTAINER_NAME" 2>&1 | tail -20 >&2
    fail "BIND9 container exited immediately — check named.conf"
fi
pass "BIND9 container is running"

log "waiting for BIND9 DNS on port ${BIND9_DNS_PORT} ..."
if ! wait_for_dns "$BIND9_DNS_PORT" 30; then
    log "=== named container logs (last 40 lines) ==="
    docker logs "$CONTAINER_NAME" 2>&1 | tail -40 >&2
    fail "BIND9 DNS port ${BIND9_DNS_PORT} did not respond within 30s"
fi
pass "BIND9 is up"

# --- [4/7] Build bindcar (if needed) ---
log "[4/7] ensuring bindcar binary is built"
if [[ ! -x "$BINDCAR_BIN" ]]; then
    log "binary not found — running cargo build ..."
    (cd "$REPO_ROOT" && cargo build 2>&1) || fail "cargo build failed"
fi
pass "bindcar binary ready: $BINDCAR_BIN"

# --- [5/7] Start bindcar drone ---
log "[5/7] starting bindcar drone"

RNDC_SERVER="127.0.0.1:${BIND9_RNDC_PORT}"    \
RNDC_KEY_NAME="${RNDC_KEY_NAME}"                \
RNDC_ALGORITHM="${RNDC_ALGORITHM}"              \
RNDC_SECRET="${RNDC_SECRET}"                    \
NSUPDATE_SERVER="127.0.0.1"                    \
NSUPDATE_PORT="${BIND9_DNS_PORT}"               \
NSUPDATE_TCP="true"                            \
NSUPDATE_KEY_NAME="${RNDC_KEY_NAME}"            \
NSUPDATE_ALGORITHM="${RNDC_ALGORITHM}"          \
NSUPDATE_SECRET="${RNDC_SECRET}"               \
BIND_ZONE_DIR="${ZONE_DIR}"                     \
API_PORT="${BINDCAR_PORT}"                      \
DISABLE_AUTH="true"                             \
RUST_LOG="info"                                 \
    "$BINDCAR_BIN" drone &>"${TEST_ROOT}/bindcar.log" &
BINDCAR_PID=$!

log "waiting for bindcar API on port ${BINDCAR_PORT} ..."
wait_for_bindcar "$BINDCAR_PORT" 20
pass "bindcar is up (pid ${BINDCAR_PID})"

# Confirm readiness endpoint is green
READY=$(curl -sf "http://127.0.0.1:${BINDCAR_PORT}/api/v1/ready" 2>/dev/null || echo '{}')
log "readiness: ${READY}"

# --- [6/8] API: create zone foo.bar and verify SOA ---
log "[6/8] creating zone foo.bar"

curl_assert "POST /api/v1/zones (foo.bar)" "201" \
    -X POST "http://127.0.0.1:${BINDCAR_PORT}/api/v1/zones" \
    -H "Content-Type: application/json" \
    -d "{
          \"zoneName\": \"foo.bar\",
          \"zoneType\": \"primary\",
          \"updateKeyName\": \"${RNDC_KEY_NAME}\",
          \"zoneConfig\": {
            \"ttl\": 3600,
            \"soa\": {
              \"primaryNs\": \"ns1.foo.bar.\",
              \"adminEmail\": \"admin.foo.bar.\"
            },
            \"nameServers\": [\"ns1.foo.bar.\"],
            \"nameServerIps\": {
              \"ns1.foo.bar.\": \"127.0.0.1\"
            },
            \"records\": []
          }
        }"

sleep 1
dig_assert "foo.bar SOA present" "foo.bar" "SOA" "*"

# --- [7/8] Add A record and verify ---
log "[7/8] adding A record: test.foo.bar → 1.2.3.4"

# Note: the JSON field is "type", not "recordType"
# (records.rs uses #[serde(rename = "type")] on the record_type field)
curl_assert "POST /api/v1/zones/foo.bar/records (A)" "201" \
    -X POST "http://127.0.0.1:${BINDCAR_PORT}/api/v1/zones/foo.bar/records" \
    -H "Content-Type: application/json" \
    -d '{
          "name": "test",
          "type": "A",
          "value": "1.2.3.4",
          "ttl": 3600
        }'

sleep 1
dig_assert "test.foo.bar A present" "test.foo.bar" "A" "1.2.3.4"

# --- [8/8] Delete record, verify gone, delete zone, verify gone ---
log "[8/8] deleting A record and zone, verifying removal"

curl_assert "DELETE /api/v1/zones/foo.bar/records (A)" "200" \
    -X DELETE "http://127.0.0.1:${BINDCAR_PORT}/api/v1/zones/foo.bar/records" \
    -H "Content-Type: application/json" \
    -d '{"name": "test", "type": "A", "value": "1.2.3.4"}'

sleep 1
dig_assert "test.foo.bar A absent" "test.foo.bar" "A" ""

curl_assert "DELETE /api/v1/zones/foo.bar" "200" \
    -X DELETE "http://127.0.0.1:${BINDCAR_PORT}/api/v1/zones/foo.bar"

sleep 1
dig_assert "foo.bar SOA absent" "foo.bar" "SOA" ""

echo ""
echo "══════════════════════════════════════════════════════"
echo "  integration test PASSED"
echo "══════════════════════════════════════════════════════"
echo ""
