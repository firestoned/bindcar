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

Statuses were verified against `main` @ `82d4dc5` (bindcar 0.7.3) on 2026-09-18.

### Reference and analysis

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [01](.github/community/01-dnssec-feature-summary.md) | DNSSEC feature summary | 📄 | Record of shipped work. `dnssecPolicy`/`inlineSigning` on `zones::ZoneConfig`; `auto_dnssec`, `key_directory`, `sig_validity_interval` on `rndc_types::ZoneConfig` |

### Architecture and refactoring

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [02](.github/community/02-feature-gate-http-server.md) | Feature-gate the HTTP server | ⛔ | `[features]` has only `default = []` and `k8s-token-review` — no server gate, so library-only consumers still pull the whole stack. Targeted 0.7.0; tree is at 0.7.3 |

### Features

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [03](.github/community/03-bind9-full-zone-config.md) | BIND9 full zone configuration support | ✅ | All 5 phases landed. ~40 options modelled directly plus the `raw_options` catch-all (`src/rndc_types.rs:342`) round-tripping the rest. Remaining items are marked optional in the doc |
| [04](.github/community/04-standalone-out-of-cluster.md) | Standalone / out-of-cluster bindcar | 🔶 | **Phase 1 of 5.** K8s auth done (`build_kube_client`, `src/auth.rs:490`; `drone` subcommand). Phases 2–5 unstarted: no `zone_transport.rs`, no `instance.rs`, no `packaging/`, no new docs pages |

### Security and compliance

| # | Roadmap | Status | Notes |
|---|---|---|---|
| [05](.github/community/05-api-transport-tls.md) | TLS for the HTTP API transport | ✅ | Shipped 2026-09-18. `--tls-cert`/`--tls-key` (rustls, TLS 1.2+1.3) and `--tls-client-ca` for mTLS; plaintext remains the default and warns on non-loopback. Misconfiguration exits 1 rather than downgrading. 13 unit tests + `make tls-transport-test` |

## Consumer upgrade guides

The bindcar → bindy upgrade guides are not roadmaps and are not tracked here.
They describe what the *bindy operator* must change to consume a bindcar
release, so they live in bindy's own board as **53**–**56**; **56** covers
v0.7.2 → v0.7.4 and is the current one.

## Previously tracked privately

Roadmap **05** was held outside this repository while it described an
unremediated transport-security weakness in shipped code — `firestoned/bindcar`
is public. TLS shipped on 2026-09-18, so the document was migrated in and is
indexed above. No numbers are reserved now; the next new roadmap takes **06**.

## Keeping this current

When a roadmap item's status changes (something lands, something new
starts), update its row here in the same PR/commit that makes the change
— this file is a status board, not documentation of intent. Detailed
task-level tracking stays inside each roadmap doc; this file only tracks
the item-level state.
