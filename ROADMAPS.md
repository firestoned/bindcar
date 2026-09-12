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

Statuses were verified against `did-code` @ `998bc5a` (bindcar 0.7.3) on 2026-09-12.

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
| [03](.github/community/03-bind9-full-zone-config.md) | BIND9 full zone configuration support | 🔶 | `rndc_types::ZoneConfig` models ~40 zone statement options against the 2 the doc recorded. Not audited option-by-option against the doc's list |
| [04](.github/community/04-standalone-out-of-cluster.md) | Standalone / out-of-cluster bindcar | ✅ | `drone` subcommand (`src/cli.rs:44`) plus custom-kubeconfig TokenReview (`src/auth.rs:458`); covered by `integration-test/drone-external-bind9.sh` |

## Consumer upgrade guides

The bindcar → bindy upgrade guides are not roadmaps and are not tracked here.
They describe what the *bindy operator* must change to consume a bindcar
release, so they live in bindy's own board as **53**–**56**; **56** covers
v0.7.2 → v0.7.4 and is the current one.

## Tracked privately

Roadmap number **05** is assigned but intentionally not published here: it
covers an unremediated transport-security weakness in shipped code, and this
repository is public. It is tracked privately until that work lands. The
number is reserved — do not reuse it; the next new roadmap takes **06**.

## Keeping this current

When a roadmap item's status changes (something lands, something new
starts), update its row here in the same PR/commit that makes the change
— this file is a status board, not documentation of intent. Detailed
task-level tracking stays inside each roadmap doc; this file only tracks
the item-level state.
