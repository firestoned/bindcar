#!/usr/bin/env bash
# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT
#
# Deploy-manifest & security-invariant regression checks.
#
# Guards the hardening fixes (RED-team sweep A5/A6/A10/A11/A12) against silent
# regression. Asserts, over the shipped manifests and Dockerfiles:
#   * every deploy/*.yaml and .github/workflows/*.y*ml parses as valid YAML
#   * pod-hardening enforces Pod Security Admission "restricted"
#   * every NetworkPolicy egress rule is scoped (no implicit 0.0.0.0/0)
#   * RBAC does NOT bind the over-broad system:auth-delegator role
#   * production Dockerfiles pin their base image by @sha256 digest
#   * the RNDC executor rejects the deprecated md5/sha1 HMAC algorithms
#
# Exits non-zero on the first failed invariant. Pure static checks — no cluster,
# no Docker, no network required.
set -euo pipefail

cd "$(dirname "$0")/.."

# Manifest-aware checks need PyYAML. GitHub ubuntu runners ship it; install
# defensively (covering PEP-668 "externally managed" envs) if a minimal
# environment lacks it, so the workflow stays a clean `make regression` call.
if ! python3 -c "import yaml" 2>/dev/null; then
  echo "  · PyYAML not found — installing"
  python3 -m pip install --quiet --disable-pip-version-check pyyaml 2>/dev/null \
    || python3 -m pip install --quiet --break-system-packages pyyaml 2>/dev/null \
    || { echo "  ✗ PyYAML is required for deploy validation but could not be installed" >&2; exit 1; }
fi

fail() {
  echo "  ✗ $1" >&2
  exit 1
}
ok() { echo "  ✓ $1"; }

echo "[1/6] YAML validity (deploy + workflows)"
python3 - <<'PY' || fail "YAML parse error"
import glob, sys, yaml
bad = []
for f in glob.glob("deploy/*.yaml") + glob.glob(".github/workflows/*.y*ml") + glob.glob(".github/**/*.y*ml", recursive=True):
    try:
        list(yaml.safe_load_all(open(f)))
    except Exception as e:
        bad.append(f"{f}: {e}")
if bad:
    print("\n".join(bad)); sys.exit(1)
PY
ok "all YAML parses"

echo "[2/6] Pod Security Admission = restricted (A5)"
# POSIX grep (not rg) — CI runners do not ship ripgrep.
grep -Eq 'pod-security\.kubernetes\.io/enforce:[[:space:]]*restricted' deploy/pod-hardening.yaml \
  || fail "deploy/pod-hardening.yaml missing enforced PSA restricted label (A5)"
ok "PSA restricted enforced"

echo "[3/6] NetworkPolicy egress is scoped — no implicit 0.0.0.0/0 (A6)"
python3 - <<'PY' || fail "unscoped egress rule found (A6)"
import glob, sys, yaml
offenders = []
for f in glob.glob("deploy/*.yaml"):
    for doc in yaml.safe_load_all(open(f)):
        if not isinstance(doc, dict) or doc.get("kind") != "NetworkPolicy":
            continue
        for rule in (doc.get("spec", {}).get("egress") or []):
            # A bare ports rule with no `to:` selector allows egress to all
            # destinations. DNS (53) is intentionally broad for authoritative
            # DNS ingress but egress must be scoped.
            if "to" not in rule:
                offenders.append(f"{f}: egress rule without `to:` -> {rule.get('ports')}")
if offenders:
    print("\n".join(offenders)); sys.exit(1)
PY
ok "all egress rules scoped"

echo "[4/6] RBAC does not bind system:auth-delegator (A10)"
python3 - <<'PY' || fail "RBAC invariant failed (A10)"
import sys, yaml
docs = list(yaml.safe_load_all(open("deploy/rbac.yaml")))
# Inspect the actual roleRef of each binding (NOT comment text).
bindings = [d for d in docs if isinstance(d, dict) and d.get("kind") == "ClusterRoleBinding"]
for b in bindings:
    ref = (b.get("roleRef") or {}).get("name")
    if ref == "system:auth-delegator":
        print("ClusterRoleBinding binds over-broad system:auth-delegator"); sys.exit(1)
# The minimal role must exist and grant only create on tokenreviews.
roles = [d for d in docs if isinstance(d, dict) and d.get("kind") == "ClusterRole"]
tr = [r for r in roles if r.get("metadata", {}).get("name") == "bindcar-tokenreview"]
if not tr:
    print("missing minimal bindcar-tokenreview ClusterRole"); sys.exit(1)
for rule in tr[0].get("rules", []):
    if rule.get("resources") != ["tokenreviews"] or sorted(rule.get("verbs", [])) != ["create"]:
        print(f"bindcar-tokenreview grants more than create tokenreviews: {rule}"); sys.exit(1)
PY
ok "minimal TokenReview ClusterRole in place (no auth-delegator)"

echo "[5/6] Production Dockerfiles pin base image by digest (A11)"
for df in docker/Dockerfile docker/Dockerfile.chainguard; do
  # Every FROM line (ignoring comments) must carry an @sha256: digest.
  if grep -E '^FROM ' "$df" | grep -qv '@sha256:'; then
    fail "$df has a FROM without an @sha256 digest pin (A11)"
  fi
  ok "$df base image digest-pinned"
done

echo "[6/6] RNDC rejects deprecated md5/sha1 HMAC (A12)"
grep -Eq 'ACCEPTED_RNDC_ALGORITHMS.*=.*"sha224".*"sha256".*"sha384".*"sha512"' src/rndc.rs \
  || fail "src/rndc.rs ACCEPTED_RNDC_ALGORITHMS drifted from SHA-2-only (A12)"
if grep -Eq 'ACCEPTED_RNDC_ALGORITHMS.*"(md5|sha1)"' src/rndc.rs; then
  fail "src/rndc.rs re-enabled a weak HMAC algorithm (A12)"
fi
ok "RNDC control channel is SHA-2 only"

echo "deploy invariants OK"
