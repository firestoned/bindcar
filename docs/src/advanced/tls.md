# TLS Transport

bindcar can serve its REST API over TLS, optionally requiring a client
certificate (mutual TLS).

## Why this matters

Every request to bindcar carries a privileged credential — a Kubernetes
ServiceAccount token under TokenReview auth, or the shared `BIND_API_TOKEN`.

Authentication proves *who* the caller is. It does nothing for
**confidentiality**. Over plaintext HTTP that credential is readable by anything
able to observe the link: a CNI with node-level capture, a sidecar holding
`NET_RAW`, a compromised node, or any cluster without pod-network encryption.
The credential exposed is the one authorizing zone creation, reload and
deletion.

!!! note "What is not exposed"
    RNDC/TSIG key *material* never transits this API — only the key **name**
    does. The exposure is the API credential, not your DNS signing keys.

## Quick start

```bash
bindcar run \
  --tls-cert /etc/bindcar/tls/tls.crt \
  --tls-key  /etc/bindcar/tls/tls.key
```

Or by environment variable, which is usually easier in a container:

```bash
BIND_TLS_CERT=/etc/bindcar/tls/tls.crt \
BIND_TLS_KEY=/etc/bindcar/tls/tls.key \
  bindcar run
```

The startup log confirms the transport:

```
bind9 rndc api server listening on 0.0.0.0:8080 (TLS)
```

## Configuration

| CLI flag | Environment variable | Meaning |
|---|---|---|
| `--tls-cert` | `BIND_TLS_CERT` | PEM certificate chain, **leaf first** |
| `--tls-key` | `BIND_TLS_KEY` | PEM private key (PKCS#8, PKCS#1 or SEC1) |
| `--tls-client-ca` | `BIND_TLS_CLIENT_CA` | PEM CA bundle; setting it enables mutual TLS |

TLS is **opt-in**. With no certificate configured bindcar serves plaintext HTTP
exactly as it always has, so upgrading does not change the behaviour of an
existing deployment.

A certificate chain may contain intermediates; put the leaf first. Empty string
values count as unset, so an unset variable that expands to `""` in a shell
wrapper or manifest will not half-configure the listener.

## Mutual TLS

Setting `--tls-client-ca` requires every client to present a certificate that
chains to that bundle. A client with no certificate, or one signed by any other
CA, is rejected during the TLS handshake — before any request is parsed.

```bash
bindcar run \
  --tls-cert       /etc/bindcar/tls/tls.crt \
  --tls-key        /etc/bindcar/tls/tls.key \
  --tls-client-ca  /etc/bindcar/tls/ca.crt
```

```
mutual TLS enabled; clients must present a certificate trusted by the configured CA
```

In a Kubernetes topology where both ends have stable in-cluster identities —
the [bindy](https://github.com/firestoned/bindy) operator talking to its bindcar
sidecar, for example — mTLS is a better fit than server-only TLS, because the
client certificate can replace the bearer token entirely rather than merely
wrapping it.

!!! warning "mTLS authenticates the transport, not the API"
    A valid client certificate satisfies the TLS handshake. It does **not**
    replace bindcar's API authentication: `DISABLE_AUTH`, `BIND_API_TOKEN` and
    TokenReview behave exactly as before. Do not turn API auth off because mTLS
    is on.

## Misconfiguration fails closed

A half-configured listener is a startup error, never a silent downgrade to
plaintext. That failure mode — an operator believing TLS is on when it is
not — is the one worth being loud about.

| Situation | Result |
|---|---|
| Neither cert nor key set | Plaintext HTTP (the default) |
| Both set | TLS |
| Only one set | **Startup fails**, exit code 1 |
| Client CA set with no cert/key | **Startup fails**, exit code 1 |
| Cert or key missing/unreadable/malformed | **Startup fails**, exit code 1 |

```console
$ bindcar run --tls-cert /etc/bindcar/tls/tls.crt
Error: invalid TLS configuration

Caused by:
    TLS is half-configured: --tls-cert / BIND_TLS_CERT was set but
    --tls-key / BIND_TLS_KEY was not; set both to enable TLS, or neither
    to serve plaintext
```

## The plaintext warning

When bindcar serves plaintext on a **non-loopback** address it warns at every
start:

```
serving the API over plaintext HTTP on a non-loopback address; API credentials
cross the network in the clear. Set --tls-cert/--tls-key
(BIND_TLS_CERT/BIND_TLS_KEY) to enable TLS.
```

Loopback binds (`127.0.0.0/8`, `localhost`, `::1`) are exempt — that traffic
never leaves the host.

## Certificates in Kubernetes

[cert-manager](https://cert-manager.io/) is the straightforward route. Issue a
certificate into a `Secret` and mount it into the bindcar container:

```yaml
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: bindcar-tls
  namespace: dns-system
spec:
  secretName: bindcar-tls
  duration: 2160h      # 90d
  renewBefore: 360h    # 15d
  issuerRef:
    name: cluster-ca-issuer
    kind: ClusterIssuer
  commonName: bindcar.dns-system.svc
  dnsNames:
    - bindcar.dns-system.svc
    - bindcar.dns-system.svc.cluster.local
```

```yaml
    - name: bindcar
      image: ghcr.io/firestoned/bindcar:v0.7.4
      env:
        - name: BIND_TLS_CERT
          value: /etc/bindcar/tls/tls.crt
        - name: BIND_TLS_KEY
          value: /etc/bindcar/tls/tls.key
        # For mTLS, also mount the issuing CA and point at it:
        # - name: BIND_TLS_CLIENT_CA
        #   value: /etc/bindcar/tls/ca.crt
      volumeMounts:
        - name: bindcar-tls
          mountPath: /etc/bindcar/tls
          readOnly: true
  volumes:
    - name: bindcar-tls
      secret:
        secretName: bindcar-tls
        defaultMode: 0400
```

## Certificate rotation

bindcar re-reads the certificate, key and client CA on an interval and swaps
them in without restarting. A cert-manager renewal is picked up on its own.

| CLI flag | Environment variable | Default | Meaning |
|---|---|---|---|
| `--tls-reload-interval` | `BIND_TLS_RELOAD_INTERVAL` | `60` | Seconds between checks. `0` disables reloading. |

```
watching TLS certificate material for renewals every 60s (BIND_TLS_RELOAD_INTERVAL=0 to disable)
reloaded TLS certificate material; new connections will use it
```

Send `SIGHUP` to check immediately instead of waiting out the interval:

```bash
kill -HUP "$(pidof bindcar)"
```

**Established connections are unaffected.** The configuration is read once per
accepted connection, so a connection in flight finishes under the certificate it
started with and only new connections pick up the renewal. Nothing is dropped.

**A failed reload is a non-event.** Certificate files are not replaced
atomically — a check can easily observe a new certificate alongside a
not-yet-replaced key. When the new material does not load, bindcar logs a warning
and *keeps serving the certificate it already has*, then retries on the next
interval:

```
TLS material changed but the new configuration is not usable, continuing with
the previous certificate: TLS private key from /etc/bindcar/tls/tls.key was
rejected: keys may not be consistent: KeyMismatch
```

This is deliberately different from startup, where unreadable TLS material is
fatal. At startup, continuing would mean silently serving plaintext; during a
reload there is already a working configuration in memory, so keeping it is
safer than either failing or downgrading.

!!! note "Detection is by content, not timestamp"
    Kubernetes does not rewrite mounted `Secret` files in place — it swaps a
    `..data` symlink. bindcar hashes the file contents rather than checking
    mtime, which is what makes renewal detection work on a projected volume.

Set `BIND_TLS_RELOAD_INTERVAL=0` to restore the startup-only behaviour of
bindcar 0.7.x, where a renewal requires a pod rollout.

## Health and readiness probes

Kubernetes probes must use the matching scheme once TLS is on. The certificate
is typically issued for the Service DNS name, not the pod IP the kubelet dials,
so `scheme: HTTPS` alone is not enough — the kubelet does not verify the
certificate for probes, which is what makes this work:

```yaml
livenessProbe:
  httpGet:
    path: /api/v1/health
    port: 8080
    scheme: HTTPS
readinessProbe:
  httpGet:
    path: /api/v1/ready
    port: 8080
    scheme: HTTPS
```

Under **mutual TLS** the kubelet cannot present a client certificate, so
`httpGet` probes fail. Use an `exec` probe with a client certificate instead,
or keep probes on a separate plaintext loopback listener.

## Client examples

```bash
# Server TLS with a private CA
curl --cacert /etc/bindcar/tls/ca.crt \
     -H "Authorization: Bearer $TOKEN" \
     https://bindcar.dns-system.svc:8080/api/v1/zones

# Mutual TLS
curl --cacert /etc/bindcar/tls/ca.crt \
     --cert /etc/client/tls.crt --key /etc/client/tls.key \
     https://bindcar.dns-system.svc:8080/api/v1/zones
```

## Interaction with a service mesh

If [Linkerd](https://linkerd.io/) already provides automatic mTLS between pods,
the credential is encrypted in transit and bindcar's own TLS is redundant for
mesh-internal traffic. Enabling it anyway is defence in depth and costs a
handshake; it matters most when traffic can bypass the mesh — a probe from the
kubelet, a debug port-forward, or a caller outside the mesh.

## Protocol details

- **TLS versions:** 1.2 and 1.3, via [rustls](https://github.com/rustls/rustls).
  SSLv3, TLS 1.0 and TLS 1.1 are not implemented and cannot be enabled.
- **ALPN:** advertises `h2` then `http/1.1`; HTTP/2 and HTTP/1.1 are both served.
- **Crypto provider:** `ring`, pinned to match the provider used by the
  Kubernetes client under the `k8s-token-review` feature.

## Testing

The transport has end-to-end coverage that runs without BIND9 or a cluster:

```bash
make tls-transport-test
```

It asserts a real TLS handshake, that plaintext is refused on a TLS port, that
mTLS rejects both a missing and an untrusted client certificate, and that every
misconfiguration above exits non-zero. It is also part of `make ci-e2e`.

## See also

- [Authentication & Authorization](auth.md) — who may call the API
- [Security Overview](security.md)
- [Environment Variables](../operations/env-vars.md)
