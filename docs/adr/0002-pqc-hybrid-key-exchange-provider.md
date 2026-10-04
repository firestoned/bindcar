<!--
Copyright (c) 2026 Erick Bourgeois, firestoned
SPDX-License-Identifier: MIT
-->
# 0002: Post-quantum hybrid key exchange via the aws-lc-rs rustls provider

- **Status:** Proposed
- **Date:** 2026-10-04
- **Deciders:** Erick Bourgeois
- **Related:** Execution plan:
  [roadmap 09](../../.github/community/09-post-quantum-cryptography-readiness.md)
  (phase 2). Builds on
  [roadmap 05](../../.github/community/05-api-transport-tls.md) (TLS
  transport) and [roadmap 06](../../.github/community/06-tls-certificate-reload.md)
  (hot-reload). Documentation:
  `docs/src/advanced/crypto-inventory.md`, `docs/src/advanced/tls.md`.

## Context

The API TLS key exchange is bindcar's only surface exposed to
harvest-now-decrypt-later: a recorded handshake and session can be decrypted
retroactively once a cryptographically relevant quantum computer exists.
Roadmap 09's inventory marks it as the one urgent post-quantum item.

The standardized remediation is the hybrid key agreement `X25519MLKEM768`
(draft-ietf-tls-ecdhe-mlkem, combining X25519 with FIPS 203 ML-KEM-768),
already the default in Chrome, Firefox, Go and OpenSSL 3.5. A hybrid group
is at least as strong as its classical half, so enabling it cannot weaken
the handshake for any client.

Constraints verified in the tree:

1. bindcar pins rustls to the **ring** crypto provider
   (`Cargo.toml`), and ring has no ML-KEM support. rustls ships
   `X25519MLKEM768` through its **aws-lc-rs** provider, with the
   `prefer-post-quantum` cargo feature ordering it first.
2. The ring pin exists for one reason, documented in `Cargo.toml`: kube's
   `rustls-tls` feature installs a provider too, and two enabled provider
   features leave rustls with **no compile-time default**, which panics at
   the first `ServerConfig::builder()` ("no process-level CryptoProvider").
   Any provider change must solve this deterministically, not by feature
   luck.
3. `rustls` is only a direct dependency under the `tls` feature, but the
   provider question also exists for `k8s-token-review` builds, where kube
   pulls rustls on its own.
4. The release images cross-build for `x86_64`/`aarch64-unknown-linux-musl`
   (`docker/Dockerfile.chef`). ring compiles C with `cc`; aws-lc-rs
   additionally drives CMake, so the builder stages need `cmake`.

## Decision

1. **Switch the rustls crypto provider from ring to aws-lc-rs, with
   `prefer-post-quantum`.**

   - `rustls` features become `aws-lc-rs`, `prefer-post-quantum`, `std`,
     `tls12`, `logging`; `tokio-rustls` mirrors them. The server then offers
     `X25519MLKEM768` as its most-preferred group while keeping every
     classical group for interoperability, including TLS 1.2.
   - No configuration flag: hybrid key exchange is a transparent transport
     upgrade, the same stance roadmap 05 took on cipher suites.

2. **Install the provider explicitly at process start instead of relying on
   feature-resolved defaults.**

   - A new `tls::ensure_crypto_provider()` installs the aws-lc-rs default
     provider process-wide, treating "already installed" as success, and is
     called from `main()` before any TLS or kube client construction, and
     defensively from `build_server_config()` so library consumers and
     tests cannot panic.
   - This removes the fragile invariant the ring pin protected: whatever
     provider features kube enables, the process default is always ours.
   - `k8s-token-review` adds `dep:rustls` so the install call compiles in
     TokenReview-only builds too.

3. **Prove the negotiation in tests at both levels.**

   - Unit tests handshake a rustls client restricted to `X25519MLKEM768`
     (must succeed and negotiate it), a classical-only client (must fall
     back to X25519), and a TLS 1.2 client (unchanged).
   - The e2e TLS suite (`integration-test/tls-transport.sh`, run by the
     standalone `e2e.yaml` workflow) asserts hybrid negotiation when the
     runner's OpenSSL supports ML-KEM, and records a skip otherwise.

## Consequences

- Recorded bindcar API sessions established with PQC-capable clients are no
  longer decryptable by a future quantum adversary. Classical clients are
  unaffected; they negotiate exactly what they negotiate today.
- The dependency graph swaps `ring` for `aws-lc-rs` in bindcar's own
  features; ring may remain in the graph through kube until kube itself
  migrates, which is harmless under decision 2.
- Build cost: aws-lc-sys compiles more C and needs `cmake` in the
  cross-build stages. Image sizes grow slightly.
- FIPS: aws-lc-rs has a FIPS mode bindcar does not enable; this ADR does
  not take a position on FIPS validation.
- If aws-lc-rs ever becomes unbuildable on a target bindcar must ship,
  the fallback is reverting to ring and losing the hybrid group; decision 2
  makes that a one-line provider swap.
