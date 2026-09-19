# bindcar: TLS for the HTTP API transport

> **Status:** ✅ Shipped 2026-09-18. Items 1–3 of "Proposed work" are implemented;
> item 4 (cert provisioning docs) is covered by
> [TLS Transport](../../docs/src/advanced/tls.md).
>
> - Optional TLS listener — `--tls-cert`/`--tls-key` (`BIND_TLS_CERT`/`BIND_TLS_KEY`),
>   rustls, TLS 1.2+1.3, plaintext still the default. `src/tls.rs`, `src/main.rs`.
> - Optional mTLS — `--tls-client-ca` (`BIND_TLS_CLIENT_CA`) makes a trusted client
>   certificate mandatory at the handshake.
> - The advertised Swagger URL now follows the active scheme, and bindcar warns at
>   startup when serving plaintext on a non-loopback address.
> - Misconfiguration fails closed: a half-configured listener exits 1 rather than
>   silently serving plaintext.
> - Tests: 13 unit tests in `src/tls_test.rs`; 15 end-to-end assertions in
>   `integration-test/tls-transport.sh` (`make tls-transport-test`, also in `make ci-e2e`).
>
> *Migrated into the repo on 2026-09-18, once the fix shipped — this document was
> deliberately held outside a public repository while the weakness was unremediated.
> The body below is the analysis as originally written on 2026-09-11.*

---

## Summary

bindcar's REST API is served over plaintext HTTP with no TLS option at all. The bindy
operator authenticates to it with a Kubernetes ServiceAccount token, so a privileged
bearer credential crosses the pod network in the clear on every zone operation.

## Current state (verified 2026-09-11 against bindcar 0.7.3 sources)

- `src/main.rs` binds a bare `tokio::net::TcpListener` and calls `axum::serve` on it.
  There is **no** `rustls`, no `TlsAcceptor`, no `axum_server::tls_*`, and no
  certificate/key CLI flag or environment variable anywhere in the crate.
- The startup banner advertises the Swagger UI as `http://<addr>/api/v1/docs`.
- Authentication is real and works — TokenReview (Mode B) or a shared
  `BIND_API_TOKEN` — but authentication is orthogonal to transport confidentiality.
  The credential that proves identity is exactly the thing exposed.
- On the bindy side, `build_api_url` (`src/bind9/zone_ops.rs`) already **honours an
  explicit `https://` prefix**. It only defaults to `http://` when the configured
  server address carries no scheme. So bindy needs no change to *consume* TLS — the
  gap is entirely that bindcar cannot *serve* it.

## Impact

Anything able to observe traffic between the operator pod and the bindcar sidecar can
read the bearer token: a CNI with node-level capture, a sidecar holding `NET_RAW`, a
compromised node, or a cluster without pod-network encryption. The token is
audience-scoped, which limits where it can be replayed, but the audience it is scoped
*to* is the API that creates, reloads and deletes DNS zones.

Note what is **not** at risk: RNDC/TSIG key material itself never transits this API —
only the key *name* does. The exposure is the API credential, not the DNS signing keys.

## Compensating control already in place (bindy side)

`deploy/pod-hardening.yaml` restricts ingress to the bindcar port to the operator pod.
That rule previously had no `from:` selector, so every pod in the cluster could reach
it; it is now operator-only.

This narrows *who can connect*. It does **not** make the transport confidential — an
observer of the link still sees the token. It is mitigation, not a fix, and the
roadmap item stays open until bindcar can terminate TLS.

## Proposed work

1. **Optional TLS listener.** Add `--tls-cert` / `--tls-key` (plus env equivalents)
   and serve via rustls when both are present. Keep plaintext as the default so
   existing deployments are unaffected, and log loudly at startup when serving
   plaintext with authentication enabled.
2. **Optional mTLS.** Accept a `--tls-client-ca` and require a client certificate when
   set. In the bindy topology both ends are in-cluster with stable identities, so mTLS
   is a better fit than server-only TLS and removes the bearer token from the wire
   entirely.
3. **Fix the advertised Swagger URL** to match the active scheme.
4. **Document the cert-provisioning path** — cert-manager issuing a Secret the operator
   mounts is the obvious route for the bindy deployment.

Effort: **M**. Contained in bindcar's server setup; no protocol or handler changes.

## Coordination with bindy

- bindy needs **no code change** to consume TLS (`build_api_url` already passes an
  explicit `https://` through).
- bindy's `Bind9Instance` / `Bind9Cluster` CRDs would need a way to express the
  scheme and the CA bundle for the sidecar endpoint.
- The bindcar version pin in bindy moves only after a bindcar release carries this.
  Note the existing version-coupling hazard: bindcar 0.7.2 sidecars reject the
  `ip:port` endpoint form that 0.7.3 accepts, so version skew between operator and
  sidecar is already a live concern — fold the TLS rollout into that same
  compatibility matrix rather than treating it as independent.

## Why this was not a public issue (historical)

> Resolved: TLS shipped on 2026-09-18, so this document is now in the repo. The
> reasoning below is kept for the audit trail.


The *absence* of TLS is self-evident from bindcar's public source, and "please add TLS"
is an ordinary feature request that would disclose nothing. But an issue useful enough
to act on has to state the consequence: which credential crosses the wire, what it
authorizes, and that no fix exists today. That combination is an exploitation path for
an unremediated weakness in code people are running right now.

This mirrors how the bindy repo already handles the same situation — its security
roadmaps are held outside the repository until the underlying finding lands.

**When TLS ships:** open the issue/PR in bindcar, and migrate this document into the
repo alongside it.

> Done — see the status block at the top of this file.
