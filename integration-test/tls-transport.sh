#!/usr/bin/env bash
# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT
#
# tls-transport.sh — end-to-end test for bindcar's TLS API transport.
#
# Exercises the transport itself against a real running bindcar process. The
# unit tests in src/tls_test.rs cover config resolution and PEM loading; this
# script covers what they cannot: that the listener actually completes a TLS
# handshake, that plaintext is refused on a TLS port, that mutual TLS rejects a
# client with no certificate, and that a half-configured listener refuses to
# start instead of silently serving plaintext.
#
# No BIND9 is required — every assertion is against /api/v1/health, which does
# not touch RNDC. That keeps this fast and runnable anywhere.
#
# Requirements:
#   - bindcar binary (built via "cargo build")
#   - openssl (certificate fixtures)
#   - curl with TLS support
#
# Usage:
#   ./integration-test/tls-transport.sh
#   BINDCAR_BIN=target/release/bindcar ./integration-test/tls-transport.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINDCAR_BIN="${BINDCAR_BIN:-${REPO_ROOT}/target/debug/bindcar}"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/bindcar-tls-test.XXXXXX")"

# Ports are high and fixed-offset so a failed run leaves nothing listening on
# anything meaningful. Override with TLS_TEST_PORT_BASE if they collide.
PORT_BASE="${TLS_TEST_PORT_BASE:-18500}"

FAILURES=0
BINDCAR_PID=""

log()  { printf '\033[0;34m[tls-e2e]\033[0m %s\n' "$*"; }
pass() { printf '\033[0;32m  ✓\033[0m %s\n' "$*"; }
fail() { printf '\033[0;31m  ✗\033[0m %s\n' "$*"; FAILURES=$((FAILURES + 1)); }

cleanup() {
    stop_bindcar
    rm -rf "$TEST_ROOT"
}
trap cleanup EXIT

stop_bindcar() {
    if [[ -n "$BINDCAR_PID" ]] && kill -0 "$BINDCAR_PID" 2>/dev/null; then
        kill "$BINDCAR_PID" 2>/dev/null || true
        wait "$BINDCAR_PID" 2>/dev/null || true
    fi
    BINDCAR_PID=""
}

# Start bindcar with the shared baseline env plus any extra KEY=VALUE args.
# Auth is disabled because this script tests the *transport*, not authn.
start_bindcar() {
    local port="$1"; shift
    local logfile="$1"; shift

    env \
        BIND_ZONE_DIR="${TEST_ROOT}/zones" \
        API_PORT="$port" \
        DISABLE_AUTH=true \
        BINDCAR_ALLOW_INSECURE_AUTH=true \
        RNDC_SERVER=127.0.0.1:19953 \
        RNDC_KEY_NAME=test-key \
        RNDC_ALGORITHM=hmac-sha256 \
        RNDC_SECRET="$RNDC_SECRET" \
        RUST_LOG=info \
        "$@" \
        "$BINDCAR_BIN" run &>"$logfile" &
    BINDCAR_PID=$!
}

# Poll until the health endpoint answers over the given scheme, or give up.
wait_for_api() {
    local scheme="$1" port="$2"; shift 2
    local i
    # The listener binds in well under a second; 60 polls is headroom for a slow
    # CI runner, not an expectation. A genuinely rejected handshake still burns
    # the whole budget, so callers print diagnostics on failure.
    for i in $(seq 1 60); do
        if curl -sk "$@" --max-time 2 "${scheme}://127.0.0.1:${port}/api/v1/health" >/dev/null 2>&1; then
            return 0
        fi
        if [[ -n "$BINDCAR_PID" ]] && ! kill -0 "$BINDCAR_PID" 2>/dev/null; then
            return 1   # process died; caller decides whether that is expected
        fi
        sleep 0.25
    done
    return 1
}

log "workspace: $TEST_ROOT"
mkdir -p "${TEST_ROOT}/zones"
RNDC_SECRET="$(openssl rand -base64 32)"

# --- [1/6] Fixtures ---------------------------------------------------------
log "[1/6] generating certificate fixtures"

# Extensions are stated EXPLICITLY rather than inherited from the platform's
# openssl.cnf. The defaults differ between distributions and OpenSSL versions:
# a CA generated without an explicit `basicConstraints=critical,CA:TRUE` is not
# usable as a trust anchor by rustls' path building, and a client leaf without
# `extendedKeyUsage=clientAuth` is not guaranteed to be accepted for client
# auth. Relying on those defaults made this suite pass on macOS and fail on the
# Ubuntu CI runner (PR #124).
cat >"${TEST_ROOT}/ca.ext" <<'EXT'
basicConstraints = critical, CA:TRUE
keyUsage         = critical, keyCertSign, cRLSign
subjectKeyIdentifier = hash
EXT

cat >"${TEST_ROOT}/server.ext" <<'EXT'
basicConstraints = critical, CA:FALSE
keyUsage         = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName   = DNS:localhost, IP:127.0.0.1
EXT

cat >"${TEST_ROOT}/client.ext" <<'EXT'
basicConstraints = critical, CA:FALSE
keyUsage         = critical, digitalSignature, keyEncipherment
extendedKeyUsage = clientAuth
EXT

# Self-signed server certificate (its own trust anchor; curl uses -k anyway).
openssl req -x509 -newkey rsa:2048 -nodes -sha256 \
    -keyout "${TEST_ROOT}/server-key.pem" -out "${TEST_ROOT}/server.pem" \
    -days 1 -subj "/CN=localhost" \
    -extensions v3_req -addext "basicConstraints=critical,CA:FALSE" \
    -addext "keyUsage=critical,digitalSignature,keyEncipherment" \
    -addext "extendedKeyUsage=serverAuth" \
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1" 2>/dev/null

# Trusted CA + a client certificate it signs.
openssl req -x509 -newkey rsa:2048 -nodes -sha256 \
    -keyout "${TEST_ROOT}/ca-key.pem" -out "${TEST_ROOT}/ca.pem" \
    -days 1 -subj "/CN=bindcar-test-ca" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" 2>/dev/null

openssl req -newkey rsa:2048 -nodes -sha256 \
    -keyout "${TEST_ROOT}/client-key.pem" -out "${TEST_ROOT}/client.csr" \
    -subj "/CN=bindy-operator" 2>/dev/null
openssl x509 -req -sha256 -in "${TEST_ROOT}/client.csr" \
    -CA "${TEST_ROOT}/ca.pem" -CAkey "${TEST_ROOT}/ca-key.pem" -CAcreateserial \
    -extfile "${TEST_ROOT}/client.ext" \
    -out "${TEST_ROOT}/client.pem" -days 1 2>/dev/null

# A client cert from a CA the server does NOT trust, to prove the verifier
# checks the chain rather than merely the presence of a certificate.
openssl req -x509 -newkey rsa:2048 -nodes -sha256 \
    -keyout "${TEST_ROOT}/rogue-ca-key.pem" -out "${TEST_ROOT}/rogue-ca.pem" \
    -days 1 -subj "/CN=rogue-ca" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" 2>/dev/null
openssl req -newkey rsa:2048 -nodes -sha256 \
    -keyout "${TEST_ROOT}/rogue-key.pem" -out "${TEST_ROOT}/rogue.csr" \
    -subj "/CN=rogue-client" 2>/dev/null
openssl x509 -req -sha256 -in "${TEST_ROOT}/rogue.csr" \
    -CA "${TEST_ROOT}/rogue-ca.pem" -CAkey "${TEST_ROOT}/rogue-ca-key.pem" -CAcreateserial \
    -extfile "${TEST_ROOT}/client.ext" \
    -out "${TEST_ROOT}/rogue.pem" -days 1 2>/dev/null

# Fail fast and loudly if the platform's openssl produced something unusable,
# rather than surfacing it later as a confusing "valid cert rejected".
if ! openssl x509 -in "${TEST_ROOT}/ca.pem" -noout -text | grep -q "CA:TRUE"; then
    fail "generated CA lacks basicConstraints CA:TRUE — openssl $(openssl version)"
fi
if ! openssl x509 -in "${TEST_ROOT}/client.pem" -noout -text | grep -q "TLS Web Client Authentication"; then
    fail "generated client cert lacks extendedKeyUsage clientAuth — openssl $(openssl version)"
fi
if ! openssl verify -CAfile "${TEST_ROOT}/ca.pem" "${TEST_ROOT}/client.pem" >/dev/null 2>&1; then
    fail "client certificate does not verify against its own CA"
fi

pass "server, CA, client and rogue-client certificates generated (openssl $(openssl version | awk '{print $2}'))"

if [[ ! -x "$BINDCAR_BIN" ]]; then
    log "binary not found — running cargo build ..."
    (cd "$REPO_ROOT" && cargo build) || { fail "cargo build failed"; exit 1; }
fi

# --- [2/6] Plaintext still works (backward compatibility) -------------------
log "[2/6] plaintext listener (no TLS configured)"
PORT=$((PORT_BASE))
start_bindcar "$PORT" "${TEST_ROOT}/plain.log"

if wait_for_api http "$PORT"; then
    pass "plaintext HTTP health endpoint responds"
else
    fail "plaintext listener did not come up"
fi

if grep -q "serving the API over plaintext HTTP on a non-loopback address" "${TEST_ROOT}/plain.log"; then
    pass "plaintext transport warning is logged"
else
    fail "expected a plaintext transport warning in the log"
fi
stop_bindcar

# --- [3/6] TLS listener -----------------------------------------------------
log "[3/6] TLS listener"
PORT=$((PORT_BASE + 1))
start_bindcar "$PORT" "${TEST_ROOT}/tls.log" \
    BIND_TLS_CERT="${TEST_ROOT}/server.pem" \
    BIND_TLS_KEY="${TEST_ROOT}/server-key.pem"

if wait_for_api https "$PORT"; then
    pass "HTTPS health endpoint responds"
else
    fail "TLS listener did not come up"
fi

HEALTH_BODY="$(curl -sk --max-time 5 "https://127.0.0.1:${PORT}/api/v1/health" || true)"
if grep -q '"status":"healthy"' <<<"$HEALTH_BODY"; then
    pass "HTTPS response body is the expected health payload"
else
    fail "HTTPS health payload was not as expected: ${HEALTH_BODY}"
fi

# The handshake must actually be TLS, not an accidental plaintext fallback.
# NOTE: capture first rather than piping into `grep -q` — under `set -o pipefail`
# grep exits at the first match, curl takes SIGPIPE, and the pipeline reports
# failure even though the assertion passed.
HANDSHAKE="$(curl -sk -v --max-time 5 "https://127.0.0.1:${PORT}/api/v1/health" 2>&1 || true)"
if grep -qE "TLSv1\.[23]" <<<"$HANDSHAKE"; then
    pass "connection negotiates TLS 1.2 or better"
else
    fail "no TLS version observed in the handshake"
fi

# Plaintext against the TLS port must fail — otherwise credentials could still
# be sent in the clear to a listener the operator believes is encrypted.
if curl -s --max-time 5 "http://127.0.0.1:${PORT}/api/v1/health" >/dev/null 2>&1; then
    fail "plaintext HTTP was accepted on the TLS port"
else
    pass "plaintext HTTP is refused on the TLS port"
fi
stop_bindcar

# --- [4/6] Mutual TLS -------------------------------------------------------
log "[4/6] mutual TLS"
PORT=$((PORT_BASE + 2))
start_bindcar "$PORT" "${TEST_ROOT}/mtls.log" \
    BIND_TLS_CERT="${TEST_ROOT}/server.pem" \
    BIND_TLS_KEY="${TEST_ROOT}/server-key.pem" \
    BIND_TLS_CLIENT_CA="${TEST_ROOT}/ca.pem"

if wait_for_api https "$PORT" --cert "${TEST_ROOT}/client.pem" --key "${TEST_ROOT}/client-key.pem"; then
    pass "client presenting a trusted certificate is accepted"
else
    fail "mTLS listener rejected a valid client certificate"
    # Without this the only signal is a 15s timeout, which says nothing about
    # why. Print what both ends saw.
    echo "--- curl (verbose) ---"
    curl -sk -v --max-time 5 \
        --cert "${TEST_ROOT}/client.pem" --key "${TEST_ROOT}/client-key.pem" \
        "https://127.0.0.1:${PORT}/api/v1/health" 2>&1 | sed 's/^/    /' | tail -25
    echo "--- client certificate ---"
    openssl x509 -in "${TEST_ROOT}/client.pem" -noout -subject -issuer -dates -ext basicConstraints,keyUsage,extendedKeyUsage 2>&1 | sed 's/^/    /'
    echo "--- trusted CA ---"
    openssl x509 -in "${TEST_ROOT}/ca.pem" -noout -subject -dates -ext basicConstraints,keyUsage 2>&1 | sed 's/^/    /'
    echo "--- bindcar log ---"
    tail -20 "${TEST_ROOT}/mtls.log" | sed 's/^/    /'
fi

if curl -sk --max-time 5 "https://127.0.0.1:${PORT}/api/v1/health" >/dev/null 2>&1; then
    fail "client with NO certificate was accepted under mTLS"
else
    pass "client with no certificate is rejected"
fi

if curl -sk --max-time 5 \
        --cert "${TEST_ROOT}/rogue.pem" --key "${TEST_ROOT}/rogue-key.pem" \
        "https://127.0.0.1:${PORT}/api/v1/health" >/dev/null 2>&1; then
    fail "client with an untrusted certificate was accepted under mTLS"
else
    pass "client with an untrusted certificate is rejected"
fi

if grep -q "mutual TLS enabled" "${TEST_ROOT}/mtls.log"; then
    pass "mutual TLS is announced at startup"
else
    fail "expected a mutual-TLS startup log line"
fi
stop_bindcar

# --- [5/6] Fail-closed on half-configured TLS -------------------------------
log "[5/6] fail-closed misconfiguration"

# Certificate without key: must exit non-zero rather than serve plaintext.
set +e
env BIND_ZONE_DIR="${TEST_ROOT}/zones" API_PORT=$((PORT_BASE + 3)) \
    DISABLE_AUTH=true BINDCAR_ALLOW_INSECURE_AUTH=true \
    RNDC_SERVER=127.0.0.1:19953 RNDC_KEY_NAME=test-key RNDC_ALGORITHM=hmac-sha256 \
    RNDC_SECRET="$RNDC_SECRET" \
    BIND_TLS_CERT="${TEST_ROOT}/server.pem" \
    "$BINDCAR_BIN" run >"${TEST_ROOT}/halfconf.log" 2>&1
HALF_RC=$?
set -e

if [[ $HALF_RC -ne 0 ]]; then
    pass "cert without key exits non-zero (rc=${HALF_RC})"
else
    fail "cert without key started anyway — a silent plaintext downgrade"
fi

if grep -q "half-configured" "${TEST_ROOT}/halfconf.log"; then
    pass "half-configured TLS reports an actionable error"
else
    fail "expected a 'half-configured' error message"
fi

# Client CA with no server key pair: must refuse rather than ignore the flag.
set +e
env BIND_ZONE_DIR="${TEST_ROOT}/zones" API_PORT=$((PORT_BASE + 4)) \
    DISABLE_AUTH=true BINDCAR_ALLOW_INSECURE_AUTH=true \
    RNDC_SERVER=127.0.0.1:19953 RNDC_KEY_NAME=test-key RNDC_ALGORITHM=hmac-sha256 \
    RNDC_SECRET="$RNDC_SECRET" \
    BIND_TLS_CLIENT_CA="${TEST_ROOT}/ca.pem" \
    "$BINDCAR_BIN" run >"${TEST_ROOT}/caonly.log" 2>&1
CA_RC=$?
set -e

if [[ $CA_RC -ne 0 ]]; then
    pass "client CA without a key pair exits non-zero (rc=${CA_RC})"
else
    fail "client CA without a key pair was silently ignored"
fi

# --- [6/6] Unreadable material ----------------------------------------------
log "[6/6] unreadable TLS material"
set +e
env BIND_ZONE_DIR="${TEST_ROOT}/zones" API_PORT=$((PORT_BASE + 5)) \
    DISABLE_AUTH=true BINDCAR_ALLOW_INSECURE_AUTH=true \
    RNDC_SERVER=127.0.0.1:19953 RNDC_KEY_NAME=test-key RNDC_ALGORITHM=hmac-sha256 \
    RNDC_SECRET="$RNDC_SECRET" \
    BIND_TLS_CERT="${TEST_ROOT}/does-not-exist.pem" \
    BIND_TLS_KEY="${TEST_ROOT}/server-key.pem" \
    "$BINDCAR_BIN" run >"${TEST_ROOT}/missing.log" 2>&1
MISSING_RC=$?
set -e

if [[ $MISSING_RC -ne 0 ]]; then
    pass "missing certificate file exits non-zero (rc=${MISSING_RC})"
else
    fail "missing certificate file did not prevent startup"
fi

# --- Summary ----------------------------------------------------------------
echo
if [[ $FAILURES -eq 0 ]]; then
    printf '\033[0;32m==> all TLS transport checks passed\033[0m\n'
    exit 0
fi
printf '\033[0;31m==> %d TLS transport check(s) failed\033[0m\n' "$FAILURES"
log "logs preserved for inspection in: $TEST_ROOT"
trap - EXIT
stop_bindcar
exit 1
