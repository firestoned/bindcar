# Cryptographic Inventory

Where bindcar uses cryptography, what algorithms are involved, and how each
surface is affected by the migration to post-quantum cryptography (PQC).
This page is the auditable companion to
[roadmap 09](https://github.com/firestoned/bindcar/blob/main/.github/community/09-post-quantum-cryptography-readiness.md),
which tracks the work items.

## Why this page exists

NIST finalized the first post-quantum standards in August 2024: FIPS 203
(ML-KEM, key encapsulation), FIPS 204 (ML-DSA, signatures) and FIPS 205
(SLH-DSA, hash-based signatures). NIST IR 8547 (draft) sets the migration
timeline for classical public-key cryptography: algorithms at the 112-bit
security level are deprecated after 2030 and disallowed after 2035.

The threat model splits into three buckets:

- **Confidentiality is at risk now.** A recorded TLS session can be stored
  today and decrypted once a cryptographically relevant quantum computer
  (CRQC) exists ("harvest now, decrypt later"). The API carries TSIG
  secrets and zone data, so the TLS **key exchange** is the surface worth
  fixing early.
- **Authentication is at risk later.** A signature (DNSSEC RRSIG, X.509
  certificate, JWT, Cosign attestation) only needs to outlive its validity
  window, and forging one requires the CRQC to exist first. These surfaces
  follow their upstream ecosystems (BIND, IETF dnsop, Kubernetes, Sigstore).
- **Symmetric cryptography survives.** Grover's algorithm at most halves
  effective key strength, and even that is considered impractical at scale.
  HMAC-SHA-256 with a 256-bit secret remains safe.

## Inventory

| # | Surface | Where | Algorithms | Quantum exposure | Plan |
|---|---|---|---|---|---|
| 1 | API TLS key exchange | `src/tls.rs` (rustls with aws-lc-rs, TLS 1.2+1.3) | Hybrid `X25519MLKEM768` preferred; classical ECDHE / X25519 for other clients | Harvest-now-decrypt-later: remediated for PQC-capable clients (ADR-0002) | Done; see [TLS Transport](./tls.md) |
| 2 | API TLS server certificate, mTLS client certificates | `--tls-cert`, `--tls-client-ca` | RSA / ECDSA per the deployed PKI | Forgery only once a CRQC exists | Follows the CA ecosystem (ML-DSA X.509 profiles) |
| 3 | RNDC control channel | `src/rndc.rs` | HMAC SHA-224/256/384/512 (MD5 and SHA-1 rejected) | Symmetric: safe with 256-bit secrets | Keep the 256-bit floor (below) |
| 4 | TSIG for dynamic updates | `src/nsupdate.rs` | HMAC SHA-2 family (MD5 and SHA-1 rejected) | Symmetric: safe with 256-bit secrets | Done; keep the 256-bit floor (below) |
| 5 | DNSSEC DS computation | `src/dnssec.rs` (RFC 4509, digest type 2) | SHA-256 digest over the DNSKEY | Hash preimage: not quantum-broken | None needed; algorithm-agnostic by design |
| 6 | DNSSEC zone signing | BIND's, not bindcar's; bindcar passes `dnssecPolicy` names and parses `rndc dnssec -status` | ECDSA P-256, Ed25519 etc. per policy | No PQC DNSSEC algorithm is standardized yet | Algorithm agility proven by tests; track IETF dnsop and BIND |
| 7 | API shared-secret auth | `src/auth.rs` | SHA-256 + constant-time comparison | Symmetric: fine | None |
| 8 | Kubernetes TokenReview JWTs | `src/auth.rs` (`k8s-token-review` feature) | Cluster-controlled signing keys | Kubernetes' migration | Watch |
| 9 | Supply chain (Cosign, SLSA, GPG) | Release workflow | ECDSA (Sigstore), Ed25519/RSA (GPG) | Upstream ecosystems | Watch |

## Symmetric secret sizes

RNDC and TSIG authentication is HMAC, which is symmetric cryptography: a
quantum adversary gains at most a square-root speedup, so a 256-bit secret
retains at least 128-bit post-quantum strength.

**Use 256-bit secrets.** This is what the BIND tooling already generates by
default:

```bash
rndc-confgen -a -A hmac-sha256   # 256-bit RNDC key
tsig-keygen -a hmac-sha256       # 256-bit TSIG key
```

bindcar rejects `hmac-md5` and `hmac-sha1` for RNDC. Their classical
weaknesses, not quantum ones, are the reason; there is no configuration to
re-enable them.

## Keeping this current

Re-verify this inventory whenever a release:

- changes a cryptography dependency in `Cargo.toml` (`rustls`,
  `tokio-rustls`, `rustls-pki-types`, `sha2`, `subtle`), or
- touches any file named in the "Where" column above.

Update the table in the same PR, and reflect status changes in roadmap 09.
