# Post-quantum cryptography readiness

> **Goal.** Know exactly where bindcar uses cryptography, protect the one
> surface that is harvestable today (the API TLS key exchange) with a hybrid
> post-quantum key agreement, and make sure nothing in bindcar blocks the
> DNSSEC and PKI ecosystems from migrating underneath it.
>
> **Stop condition.** The API transport negotiates a hybrid PQC key exchange
> with capable clients (and falls back cleanly with others), no deprecated
> HMAC algorithm is accepted anywhere, and the DNSSEC code paths are proven
> algorithm-agile against algorithm numbers and mnemonics that do not exist
> yet.

> **Status:** ⛔ Not started. Created 2026-10-04 against `v0.8.2`
> (branch `deps/utoipa-6`). The inventory in this document was verified
> against the tree on that date: rustls 0.23.45 pinned to the `ring`
> provider (`Cargo.toml`), RNDC restricted to SHA-2 HMAC
> (`src/rndc.rs`, `ACCEPTED_RNDC_ALGORITHMS`), TSIG still accepting
> `hmac-md5`/`hmac-sha1` (`src/nsupdate.rs`, `ALLOWED_TSIG_ALGORITHMS`),
> DS computation fixed to SHA-256 digest type 2 (`src/dnssec.rs`).

---

**Created:** 2026-10-04
**Author:** Erick Bourgeois
**Related:** [`05-api-transport-tls.md`](05-api-transport-tls.md),
[`06-tls-certificate-reload.md`](06-tls-certificate-reload.md),
[`07-dnssec-lifecycle.md`](07-dnssec-lifecycle.md)

## Summary

NIST finalized the first post-quantum standards in August 2024: FIPS 203
(ML-KEM, key encapsulation), FIPS 204 (ML-DSA, signatures) and FIPS 205
(SLH-DSA, hash-based signatures). NIST IR 8547 (draft) sets the migration
clock for the ecosystem bindcar lives in: classical public-key algorithms at
the 112-bit level are deprecated after 2030 and disallowed after 2035. In a
regulated banking environment, auditors are already asking for a
cryptographic inventory and a transition plan; this roadmap is both.

The threat model splits bindcar's surfaces into two very different buckets:

- **Confidentiality is at risk now.** A recorded TLS session can be stored
  and decrypted later once a cryptographically relevant quantum computer
  exists ("harvest now, decrypt later"). bindcar's API carries TSIG secrets,
  RNDC-adjacent configuration and zone data, so the TLS **key exchange** is
  the one surface worth fixing early.
- **Authentication is at risk later.** A signature (DNSSEC RRSIG, X.509
  certificate, Cosign attestation, JWT) only needs to outlive its validity
  window. Forging one requires the quantum computer to exist *first*, so
  these surfaces can follow their upstream ecosystems (BIND, IETF dnsop,
  Kubernetes, Sigstore) rather than lead them.
- **Symmetric cryptography survives.** Grover's algorithm at most halves
  effective key strength, and even that is considered impractical at scale.
  HMAC-SHA-256 with a 256-bit secret remains safe; the work here is hygiene
  (minimum secret sizes, dropping MD5/SHA-1), not replacement.

## Cryptographic inventory (verified 2026-10-04)

| # | Surface | Where | Algorithms today | Quantum exposure | Action |
|---|---|---|---|---|---|
| 1 | API TLS key exchange | `src/tls.rs`, rustls 0.23.45, `ring` provider, TLS 1.2+1.3 | X25519 / ECDHE / FFDHE as negotiated | **Harvest-now-decrypt-later. The only urgent item.** | Phase 2: hybrid ML-KEM |
| 2 | API TLS server cert + mTLS client certs | `--tls-cert`, `--tls-client-ca` | RSA / ECDSA per deployed PKI | Forgery only after a CRQC exists | Phase 5: track LAMPS / CA ecosystem |
| 3 | RNDC control channel | `src/rndc.rs` | HMAC SHA-224/256/384/512 (MD5, SHA-1 rejected) | Symmetric: fine with 256-bit secrets | Phase 3: document secret-size floor |
| 4 | TSIG for dynamic updates | `src/nsupdate.rs` | HMAC incl. **`hmac-md5`, `hmac-sha1` still accepted** | Symmetric, but MD5/SHA-1 are classically weak already | Phase 3: reject, matching RNDC |
| 5 | DNSSEC: DS computation | `src/dnssec.rs` (RFC 4509, digest type 2) | SHA-256 digest over DNSKEY | Hash preimage: not quantum-broken | Phase 4: prove algorithm agility |
| 6 | DNSSEC: zone signing | BIND's, not ours; bindcar passes `dnssecPolicy` names and parses `rndc dnssec -status` | ECDSA P-256 / Ed25519 etc. per policy | Waits on IETF + BIND; no PQC DNSSEC algorithm is standardized | Phase 4: track, do not lead |
| 7 | API auth: shared secret | `src/auth.rs` (SHA-256 + constant-time compare) | Symmetric comparison | Fine | None |
| 8 | API auth: TokenReview JWTs | `src/auth.rs` (`k8s-token-review`) | Cluster-controlled signing keys | Kubernetes' migration, not bindcar's | Phase 5: watch |
| 9 | Supply chain: Cosign, SLSA, GPG commits | release workflow | ECDSA (Sigstore), Ed25519/RSA (GPG) | Upstream ecosystems | Phase 5: watch |

## Phases

### Phase 1: Publish the inventory

Make the inventory above a living document that audits can cite, instead of
a snapshot buried in a roadmap.

- [ ] Add `docs/src/security/crypto-inventory.md`: the table above, plus the
      harvest-now-decrypt-later rationale and the external timeline anchors
      (FIPS 203/204/205, NIST IR 8547 2030/2035 dates).
- [ ] Link it from the docs nav and from the threat-model page.
- [ ] Add a line item to the release checklist: re-verify the inventory when
      a release touches `Cargo.toml` crypto dependencies or any file in
      column "Where".

**Definition of done:** `make docs` builds the page; the table cites file
paths that exist in the tree.

### Phase 2: Hybrid post-quantum key exchange for the API TLS

The one change that closes the harvestable window. rustls supports
`X25519MLKEM768` (the hybrid group standardized for TLS 1.3 in
draft-ietf-tls-ecdhe-mlkem, already default in Chrome, Firefox, Go and
OpenSSL 3.5) through the **aws-lc-rs** provider. The `ring` provider bindcar
pins today has no ML-KEM support, so this phase is first a provider
migration. That is architecturally significant: **it needs an ADR before
code** (the `Cargo.toml` comment pinning `ring` exists precisely to keep
bindcar's provider aligned with what kube installs).

- [ ] ADR: migrate the rustls crypto provider from `ring` to `aws-lc-rs`.
      Must resolve:
  - [ ] kube 4.x coexistence: install the provider explicitly at startup
        via `CryptoProvider::install_default()` instead of relying on
        feature-resolved defaults, so kube's TLS stack and ours cannot
        diverge (the failure mode the current `ring` pin guards against).
  - [ ] Build impact: aws-lc-rs compiles C (needs `cc`, sometimes `cmake`);
        verify the musl static binary (`make e2e-image` path) and the
        Chainguard/distroless images still build on slate and in CI.
  - [ ] `--no-default-features` and feature-matrix builds stay green
        (`make check-no-default-features`, roadmap 02 guard).
- [ ] TDD: tests first, asserting (a) a client offering `X25519MLKEM768`
      negotiates it, (b) a classical-only client still completes the
      handshake with X25519, (c) TLS 1.2 behavior is unchanged.
- [ ] Implement: provider swap + enable the hybrid group (rustls
      `prefer-post-quantum` ordering).
- [ ] Extend `make tls-transport-test` and the e2e TLS suite with a PQC
      negotiation assertion (openssl 3.5+ or a rustls test client, since the
      system curl may not offer the group).
- [ ] Verify `TlsReloader` (roadmap 06) is provider-agnostic: hot-reload
      still swaps configs under the new provider.
- [ ] Docs: `docs/src/` TLS page gains a "post-quantum" section: what is
      negotiated, with which clients, and that no flag is needed.

**Definition of done:** an `X25519MLKEM768`-capable client provably
negotiates the hybrid group against a bindcar built from `main`; classical
clients are unaffected; all images build.

### Phase 3: Symmetric hygiene

No quantum urgency, but the inventory exposed drift and these are cheap.

- [ ] TDD + implement: drop `hmac-md5` and `hmac-sha1` from
      `ALLOWED_TSIG_ALGORITHMS` in `src/nsupdate.rs`, matching the SHA-2-only
      policy `src/rndc.rs` already enforces. **Breaking change** for anyone
      with a legacy TSIG key: call it out in the changelog and release
      notes, and have bindy's consumer upgrade guide (bindy board) cover it.
- [ ] Document the secret-size floor in the docs: RNDC and TSIG secrets
      should be 256-bit, which is what `rndc-confgen` and `tsig-keygen`
      already emit by default. Decide in review whether to log a startup
      warning for shorter secrets (observability only, no hard failure).

**Definition of done:** an `hmac-sha1` TSIG key file is rejected with a
clear error; docs state the floor.

### Phase 4: DNSSEC algorithm agility

PQC DNSSEC is not standardized: ML-DSA and SLH-DSA signatures are large
enough to break UDP response-size assumptions, and the IETF dnsop work
(including Merkle-tree-ladder style proposals) is still research. BIND
signs; bindcar must merely *not break* when BIND one day reports an
algorithm it has never seen.

- [ ] TDD: add tests feeding unknown algorithm numbers (e.g. a hypothetical
      `248`) and unknown mnemonics (e.g. `ML-DSA-44`) through:
  - [ ] `src/dnssec.rs` DNSKEY parsing and DS computation (the SHA-256
        digest is algorithm-independent and must pass through any `u8`),
  - [ ] the `rndc dnssec -status` key parser (mnemonic is a free string,
        must round-trip unrecognized values),
  - [ ] the zone-status `dnssec` block serialization.
- [ ] Fix anything those tests flush out; expectation is pass-through, not
      an allowlist.
- [ ] Add a tracking note (this file's status block) for: IETF dnsop PQC
      drafts, BIND release notes. Review at each BIND 9.2x minor bindcar
      pins in e2e.

**Definition of done:** the tests above pass; no code path rejects or
mangles an unknown DNSSEC algorithm identifier.

### Phase 5: Watch items (no bindcar code)

Explicitly out of scope until upstreams move; listed so the audit trail
shows they were considered, not forgotten.

- [ ] mTLS / server certificates with ML-DSA: wait for the LAMPS X.509
      profiles and real CA issuance; bindcar already treats certificates as
      opaque PEM, so this should be provider support only.
- [ ] Kubernetes TokenReview JWT signing: the cluster's key, the cluster's
      migration.
- [ ] Sigstore/Cosign and GPG PQC: follow upstream; release workflow
      re-verified when they switch.
- [ ] Revisit this roadmap when NIST IR 8547 finalizes or BIND ships a PQC
      signing algorithm, whichever is first.

## Sequencing and effort

Phase 2 is the only substantive engineering item and is independent of the
others. Phases 1 and 3 are small and can land in one PR each. Phase 4 is a
test-writing exercise. Suggested order: 1 → 3 → 4 → 2 (ADR for phase 2 can
be written in parallel with the rest).

## References

- FIPS 203 (ML-KEM), FIPS 204 (ML-DSA), FIPS 205 (SLH-DSA), NIST, 2024-08-13
- NIST IR 8547 (draft): Transition to Post-Quantum Cryptography Standards
- draft-ietf-tls-ecdhe-mlkem: X25519MLKEM768 hybrid key agreement for TLS 1.3
- rustls provider documentation: `aws-lc-rs`, `prefer-post-quantum`
- RFC 4509: use of SHA-256 in DNSSEC DS RRs (what `src/dnssec.rs` implements)
- IETF dnsop working group: post-quantum DNSSEC problem statements and drafts
