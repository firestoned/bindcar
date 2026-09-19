# TLS certificate hot-reload

> **Goal.** A renewed certificate takes effect without restarting bindcar.
>
> **Stop condition.** Replacing the certificate and key files on disk causes new
> TLS connections to present the new certificate within the poll interval, with
> no dropped connections, no restart, and no window in which a partially-written
> file takes the listener down.

> **Status:** ✅ Shipped 2026-09-18. Every task below is done and both stop
> conditions are demonstrated by `make tls-transport-test`:
>
> - `TlsReloader` (`src/tls.rs`) holds the live `ServerConfig`; `serve_tls()`
>   reads it per accepted connection, so a swap reaches new connections while
>   established ones finish under the certificate they started with.
> - Change detection is a SHA-256 `fingerprint()` over the certificate, key and
>   client CA contents — not mtime — so it works across Kubernetes' `..data`
>   symlink swap.
> - A reload that fails to build is a non-event: it logs a warning and keeps
>   serving the previous certificate, then retries. Proven against the real
>   non-atomic-write case (new cert, stale key → `KeyMismatch`).
> - Poll interval `BIND_TLS_RELOAD_INTERVAL` (default 60s, `0` disables), plus
>   `SIGHUP` for an immediate check on Unix.
> - 7 unit tests in `src/tls_test.rs`; 8 e2e assertions in
>   `integration-test/tls-transport.sh` section 7/7, including that the process
>   pid is unchanged across the rotation.

---

**Created:** 2026-09-18
**Author:** Erick Bourgeois
**Follows:** [`05-api-transport-tls.md`](05-api-transport-tls.md)

## Summary

TLS shipped in roadmap 05, and its user guide documents a limitation:

> **Certificate rotation needs a restart.** bindcar reads the certificate and key
> once, at startup. When cert-manager renews the `Secret` the running process
> keeps the old certificate until it restarts.

That limitation is the whole of this roadmap. Documenting a gap is not the same
as tracking it, and in a deployment with short-lived certificates it is the
difference between rotation being routine and rotation being an outage.

## Why it matters

- **Short-lived certificates are the direction of travel.** A 90-day
  cert-manager certificate with a 15-day `renewBefore` is comfortable; the
  industry is moving toward days, not months. Every renewal currently needs a
  pod rollout of the DNS control plane.
- **A rollout of bindcar is a rollout of the BIND9 pod.** bindcar runs as a
  sidecar, so restarting it to pick up a certificate restarts the operand
  alongside it. Certificate rotation should not be coupled to DNS availability.
- **Silent expiry is the worse failure.** If a renewal lands but nothing
  restarts, bindcar keeps serving the old certificate until it expires — and
  then every bindy zone operation fails at the TLS handshake, with the cause
  several days removed from the change that caused it.

## Current state (verified 2026-09-18 against `main` @ `5233c0b`)

- `tls::build_server_config()` reads the certificate, key and optional client CA
  from disk and returns a `ServerConfig`. It is called once, from `main`.
- `main::serve_tls()` does `TlsAcceptor::from(tls_config)` **before** the accept
  loop, so every connection for the life of the process uses that one config.
- There is no watcher, no polling, no signal handler: `rg -c 'reload|notify|watch'
  src/tls.rs` returns 0.

## Design

### Swap the whole `ServerConfig`, not just the certificate

rustls offers a `ResolvesServerCert` hook that would let the certificate alone be
swapped behind a stable `ServerConfig`. That is the narrower change, but it does
not cover the client CA bundle under mutual TLS, which lives in the
`ClientCertVerifier` baked into the config.

Instead, hold the config in a shared cell and build the `TlsAcceptor` **per
connection**:

```rust
// shared, cheap to clone, cheap to read
type SharedConfig = Arc<RwLock<Arc<rustls::ServerConfig>>>;

loop {
    let (stream, peer) = listener.accept().await?;
    let config = shared.read().expect("tls config lock").clone(); // Arc clone
    let acceptor = TlsAcceptor::from(config);
    // ... spawn as today
}
```

`TlsAcceptor::from(Arc<ServerConfig>)` is a newtype wrapper around the `Arc`, so
per-connection construction costs one atomic increment. The lock is held only
long enough to clone the `Arc`, never across an `await`, so
`std::sync::RwLock` is correct here and `tokio::sync::RwLock` is unnecessary.

This covers the certificate, the key **and** the client CA in one mechanism.

### Detect change by content hash, not mtime

Kubernetes projected volumes and `Secret` mounts do not rewrite files in place —
they swap a `..data` symlink. An mtime check on the visible path is unreliable
across that swap, and an inode watch breaks entirely.

Hash the bytes of each configured file (`sha2` is already a dependency) and
compare against the last-seen digest. A change in any of the three triggers a
rebuild.

### Reload must never take the listener down

This is the rule the implementation has to honour above all others. Files can be
observed mid-write: a poll can catch a certificate already replaced while its key
has not been, and the pair will not match.

**On any failure to build a new `ServerConfig`, log at `warn` and keep serving
the existing one.** A reload that fails is a non-event; the next poll retries. A
reload that succeeds swaps atomically. The listener is never reconfigured into a
broken state, and bindcar never exits because a certificate file was briefly
inconsistent.

Note the deliberate asymmetry with startup: at startup, unreadable TLS material
is fatal (roadmap 05's fail-closed rule) because serving plaintext instead would
be a silent downgrade. At reload there is already a working configuration in
memory, so continuing to serve it is strictly safer than either failing or
downgrading.

### Triggers

| Trigger | Behaviour |
|---|---|
| Poll interval | Default 60s. `BIND_TLS_RELOAD_INTERVAL` in seconds; `0` disables reloading entirely. |
| `SIGHUP` | Immediate check, for operators who want rotation to be deterministic rather than eventual. Unix only. |

Polling at 60s costs three file reads and three hashes per minute. That is
nothing next to the cost of a missed rotation.

## Tasks — all complete

- [x] `src/tls.rs`: `TlsReloader` holding the resolved `TlsSettings`, the shared
      config cell and the last-seen digests.
- [x] `src/tls.rs`: `fingerprint(&TlsSettings) -> Result<[u8; 32], TlsError>` —
      a digest over the certificate, key and client CA contents.
- [x] `src/tls.rs`: `reload_if_changed()` — rebuild on digest change, swap on
      success, `warn!` and retain the current config on failure.
- [x] `src/main.rs`: `serve_tls()` takes the shared cell and builds the acceptor
      per connection.
- [x] `src/main.rs`: spawn the poll task; wire `SIGHUP` on Unix.
- [x] `src/cli.rs`: `--tls-reload-interval` / `BIND_TLS_RELOAD_INTERVAL`.
- [x] `src/tls_test.rs`: digest stability; digest changes when any file changes;
      a reload with a valid new pair swaps; a reload with a broken pair keeps the
      old config and does not error out; interval `0` disables.
- [x] `integration-test/tls-transport.sh`: replace the certificate on a running
      listener and assert the **new** issuer is served, that connections during
      the swap keep working, and that writing a corrupt certificate leaves the
      listener serving the previous one.
- [x] `docs/src/advanced/tls.md`: replace the "rotation needs a restart" note
      with the reload semantics, the interval variable and the `SIGHUP` trigger.
- [x] `docs/src/operations/env-vars.md`: `BIND_TLS_RELOAD_INTERVAL`.
- [x] `.claude/CHANGELOG.md`.

## Out of scope

- **Reloading anything other than TLS material.** RNDC credentials, allowlists
  and the zone directory keep their current startup-only semantics. Widening
  this to a general config-reload mechanism is a separate roadmap.
- **An inotify/`notify`-crate watcher.** Polling is fewer dependencies and is
  the approach that actually works against Kubernetes' symlink swap. Revisit
  only if the poll interval proves too coarse in practice.

## Definition of done

- Replacing the certificate and key on a running listener causes new connections
  to present the new certificate within the poll interval, proven by the e2e
  asserting on the served issuer.
- A corrupt or half-written certificate never interrupts service: the e2e writes
  one and asserts the listener still serves the previous certificate.
- `BIND_TLS_RELOAD_INTERVAL=0` restores exactly today's startup-only behaviour.
- `cargo-quality` clean; `make tls-transport-test` green.
