# Roadmaps

High-level index of bindcar's roadmap documents. Full detail for each item
lives in [`.github/community/`](.github/community/) — this file tracks
what each one is and its current completion status; the detailed task
lists and design rationale live in the linked doc itself.

Architecturally significant work in any roadmap below still goes
**ADR → TDD → implement → docs**, in that order (see
[`.claude/rules/testing.md`](.claude/rules/testing.md) and
[`.claude/rules/documentation.md`](.claude/rules/documentation.md)) — a
roadmap entry describes *what* and *why*, it does not skip the ADR for
*how*.

## Status legend

| Symbol | Meaning |
|---|---|
| ✅ | Done — implemented, tested, in the codebase today |
| 🔶 | In progress — some of it exists, not complete |
| ⛔ | Not started |
| 📄 | Reference doc — not a phase with a completion state |

## Index

Statuses were verified against `v0.8.0` on 2026-09-19.

### Reference and analysis

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [01](.github/community/01-dnssec-feature-summary.md) | DNSSEC feature summary | 📄 | Record of shipped work. `dnssecPolicy`/`inlineSigning` on `zones::ZoneConfig`; `auto_dnssec`, `key_directory`, `sig_validity_interval` on `rndc_types::ZoneConfig` |

### Architecture and refactoring

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [02](.github/community/02-feature-gate-http-server.md) | Feature-gate the HTTP server | ✅ | **All 6 phases done.** `default = ["server", "tls"]`; bindcar sheds 80 of 178 crates under `--no-default-features`, and bindy — now on 0.8.0 with `default-features = false` — shed 31 (290 → 260). Guarded by `make check-no-default-features` in CI |

### Features

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [03](.github/community/03-bind9-full-zone-config.md) | BIND9 full zone configuration support | ✅ | All 5 phases landed. ~40 options modelled directly plus the `raw_options` catch-all (`src/rndc_types.rs:342`) round-tripping the rest. Remaining items are marked optional in the doc |
| [04](.github/community/04-standalone-out-of-cluster.md) | Standalone / out-of-cluster bindcar | 🔶 | **Phase 1 of 5.** K8s auth done (`build_kube_client`, `src/auth.rs:490`; `drone` subcommand). Phases 2–5 unstarted: no `zone_transport.rs`, no `instance.rs`, no `packaging/`, no new docs pages |
| [07](.github/community/07-dnssec-lifecycle.md) | DNSSEC lifecycle on live zones | ✅ | Shipped 2026-09-27 per [ADR-0001](docs/adr/0001-dnssec-lifecycle-transitions.md) (Accepted). PATCH `dnssecPolicy`/`inlineSigning` (merge semantics; `modzone`+`reconfig`; naive removal → 409, unsign via built-in `insecure`), `GET …/ds` (in-process RFC 4509, verified against `dnssec-dsfromkey`), `POST …/dnssec/checkds`, typed `dnssec` status block with per-key rollover timing (unblocks bindy ADR-0006). kind e2e stage 9 + live validation on BIND 9.18.50 |

### Security and compliance

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [05](.github/community/05-api-transport-tls.md) | TLS for the HTTP API transport | ✅ | Shipped 2026-09-18. `--tls-cert`/`--tls-key` (rustls, TLS 1.2+1.3) and `--tls-client-ca` for mTLS; plaintext remains the default and warns on non-loopback. Misconfiguration exits 1 rather than downgrading. 13 unit tests + `make tls-transport-test` |
| [06](.github/community/06-tls-certificate-reload.md) | TLS certificate hot-reload | ✅ | Shipped 2026-09-18. `TlsReloader` swaps the live config on a content-hash change; `BIND_TLS_RELOAD_INTERVAL` (default 60s, `0` disables) plus `SIGHUP`. A failed reload keeps the previous certificate serving. 7 unit tests + 8 e2e assertions |
| [09](.github/community/09-post-quantum-cryptography-readiness.md) | Post-quantum cryptography readiness | ✅ | **Shipped 2026-10-04** (PR #141) per [ADR-0002](docs/adr/0002-pqc-hybrid-key-exchange-provider.md) (Accepted): hybrid `X25519MLKEM768` on the API TLS (rustls provider `ring` to `aws-lc-rs`, verified live against OpenSSL 3.5.7 in e2e stage 8), `hmac-md5`/`hmac-sha1` dropped from TSIG (**breaking** for legacy keys; unreleased, must ride the next release's notes + bindy upgrade guide), DNSSEC algorithm-agility tests, crypto inventory in the docs. CI confirmed musl/image builds. Phase 5 stays open by design: standing watch items (ML-DSA certs, TokenReview JWTs, Sigstore; revisit on NIST IR 8547 final or BIND PQC signing) |

## Consumer upgrade guides

The bindcar → bindy upgrade guides are not roadmaps and are not tracked here.
They describe what the *bindy operator* must change to consume a bindcar
release, so they live in bindy's own board as **53**–**56**; **56** covers
v0.7.2 → v0.7.4 and is the current one.

## Tracked privately

Roadmap **05** was held outside this repository while it described an
unremediated transport-security weakness in shipped code — `firestoned/bindcar`
is public. TLS shipped on 2026-09-18, so the document was migrated in and is
indexed above. Number **08** is reserved the same way today: its document is
held externally until the finding it describes is remediated. The next new
roadmap takes **10**.

## Keeping this current

When a roadmap item's status changes (something lands, something new
starts), update its row here in the same PR/commit that makes the change
— this file is a status board, not documentation of intent. Detailed
task-level tracking stays inside each roadmap doc; this file only tracks
the item-level state.
