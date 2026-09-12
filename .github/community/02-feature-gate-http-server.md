# Feature-gate the HTTP server (so type-only consumers can opt out)

> **Status:** ⛔ Not started. `Cargo.toml` `[features]` carries only `default = []`
> and `k8s-token-review` — there is no `http-server` (or equivalent) gate, so a
> library-only consumer still inherits the full server stack. The doc targeted
> 0.7.0; the tree is at 0.7.3 and it did not land.
>
> *Migrated 2026-09-12 from the external roadmap set into `.github/community/`.
> Verified against `did-code` @ `998bc5a`, bindcar 0.7.3.*

---

**Original status (as written):**
**Created:** 2026-05-02
**Target:** bindcar 0.7.0
**Driver:** Eliminate ~30 transitive dependencies (including `rust-embed`, `utoipa-swagger-ui`, `tower-http`, `tower_governor`, duplicate `sha2 0.10.x`) from downstream consumers that only use bindcar's data types.

## Context

bindcar is published as both a binary (HTTP server for BIND9 management) and a library (data types + RNDC executor + parsers). Today every consumer of the library inherits the **full** dependency graph — including the HTTP server stack — even if they only import a struct.

The immediate driver: bindy (the Kubernetes operator) imports `ZoneConfig`, `SoaRecord`, `DnsRecord`, `CreateZoneRequest`, `ZoneResponse`, and `ZONE_TYPE_*` constants — pure data types — but its build pulls:

- `axum` (already a direct bindy dep, but extra version churn risk)
- `tower`, `tower-http` (with the `trace` feature)
- `tower_governor` (rate limiter)
- `utoipa`, `utoipa-swagger-ui` (the latter pulls `rust-embed` → `rust-embed-utils` → `sha2 0.10.x`, which duplicates the `sha2 0.11.x` bindy already uses)

This is a real cost in operator binary size, build time, and supply-chain attack surface.

## Goals

1. Default-on `server` feature preserving today's binary behaviour for `cargo install bindcar` users — **zero breaking change** for binary consumers.
2. `default-features = false` opt-out path for library consumers (bindy and future operators) that only need types and RNDC primitives.
3. Clear documented split so future contributors know where to place new code.
4. Validate via `cargo build --no-default-features` in CI.

## Non-goals

- Splitting into multiple crates (`bindcar-types` + `bindcar`). That's a larger restructure that we can revisit if the consumer count justifies it. Feature gating gets ~95% of the win with much less ceremony.
- Removing `utoipa-swagger-ui` itself or switching to RapiDoc/Scalar/CDN. Independent decision; orthogonal to this work.
- Async/runtime changes.

## Inventory: what is HTTP-coupled today

Files that import `axum::*`, `tower::*`, `tower_governor::*`, `utoipa::ToSchema` / `utoipa::path`, or `tower_http::*`:

| File | HTTP-coupled? | Notes |
|---|---|---|
| `src/auth.rs` | yes | axum middleware, ServiceAccount validation |
| `src/middleware.rs` | yes | axum middleware |
| `src/rate_limit.rs` | yes | `tower_governor` integration |
| `src/types.rs` | yes | `ApiError`, `AppState`, `ErrorResponse` — all axum-bound |
| `src/zones.rs` | **mixed** | structs (lines 30–308) are pure; `#[utoipa::path]` handlers (lines 319+) are HTTP |
| `src/records.rs` | **mixed** | structs (`AddRecordRequest`, `RecordResponse`, etc.) are pure; handlers (lines 242+) are HTTP |
| `src/main.rs` | yes | the binary entry point |

Files that are HTTP-free today:

- `src/cli.rs` (clap-based CLI parser)
- `src/nsupdate.rs` (subprocess executor)
- `src/rndc.rs` (TCP RNDC client)
- `src/rndc_conf_parser.rs`, `src/rndc_conf_types.rs`
- `src/rndc_parser.rs`, `src/rndc_types.rs`
- `src/metrics.rs` — uses `prometheus`; verify whether the `metrics` registry is exposed via axum routes (probably yes — needs gating).

## Design

### Cargo.toml

```toml
[features]
default = ["server"]

# Full HTTP server: routes, middleware, swagger UI, rate limiter.
server = [
    "dep:axum",
    "dep:tower",
    "dep:tower-http",
    "dep:utoipa",
    "dep:utoipa-swagger-ui",
    "dep:tower_governor",
]

k8s-token-review = ["server", "dep:kube", "dep:k8s-openapi"]

[dependencies]
# Always-on: types, parsers, RNDC client, CLI.
serde      = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono     = "0.4"
tracing    = "0.1"
anyhow     = "1.0"
thiserror  = "2.0"
tokio      = { version = "1", features = ["full"] }
clap       = { version = "4", features = ["derive"] }
nom        = "8"
rndc       = "0.1.5"
prometheus = "0.14"
lazy_static = "1.5"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Server-only:
axum              = { version = "0.8",  optional = true }
tower             = { version = "0.5",  optional = true }
tower-http        = { version = "0.6",  features = ["trace"], optional = true }
utoipa            = { version = "5",    features = ["axum_extras"], optional = true }
utoipa-swagger-ui = { version = "9",    features = ["axum"], optional = true }
tower_governor    = { version = "0.8",  optional = true }

# k8s-token-review only:
kube         = { version = "3.1", features = ["client", "rustls-tls"], optional = true }
k8s-openapi  = { version = "0.27", default-features = false, optional = true }
```

Rationale:
- `default = ["server"]` keeps `cargo install bindcar` and existing library users unchanged.
- `k8s-token-review` already implies the server stack (it's an axum middleware), so make it depend on `server` to avoid invalid feature combinations.
- `prometheus` stays always-on because the metrics registry can be queried programmatically; only the `/metrics` route needs gating.
- `tracing-subscriber` is binary-side init but small; leave always-on for simplicity.

### Module structure

Move the boundary by **splitting the mixed files**, not by sprinkling `#[cfg]` inside them. This is cleaner for new contributors:

```
src/
├── lib.rs                       (gates re-exports by feature)
├── main.rs                      (#![cfg(feature = "server")] at top)
│
│ # Always-on
├── cli.rs
├── nsupdate.rs
├── rndc.rs
├── rndc_conf_parser.rs
├── rndc_conf_types.rs
├── rndc_parser.rs
├── rndc_types.rs
├── zones_types.rs               (NEW — extract from zones.rs lines 30–308)
├── records_types.rs             (NEW — extract from records.rs structs)
├── metrics.rs                   (registry stays; see note)
│
│ # Server-gated
├── auth.rs
├── middleware.rs
├── rate_limit.rs
├── types.rs                     (ApiError, AppState, ErrorResponse — axum-bound)
├── zones.rs                     (HTTP handlers — keeps utoipa::path fns; re-exports types from zones_types)
└── records.rs                   (HTTP handlers — same pattern)
```

`lib.rs` becomes:

```rust
// Always available
pub mod cli;
pub mod nsupdate;
pub mod rndc;
pub mod rndc_conf_parser;
pub mod rndc_conf_types;
pub mod rndc_parser;
pub mod rndc_types;
pub mod zones_types;
pub mod records_types;

pub use rndc::{parse_rndc_conf, RndcConfig, RndcExecutor};
pub use nsupdate::NsupdateExecutor;
pub use rndc_conf_parser::{parse_rndc_conf_file, parse_rndc_conf_str};
pub use rndc_conf_types::{KeyBlock, OptionsBlock, RndcConfFile, ServerAddress, ServerBlock};
pub use zones_types::{
    DnsRecord, SoaRecord, ZoneConfig,
    CreateZoneRequest, ModifyZoneRequest, ZoneResponse,
    ServerStatusResponse, ZoneInfo, ZoneListResponse,
    ZONE_TYPE_PRIMARY, ZONE_TYPE_SECONDARY,
};
pub use records_types::{
    AddRecordRequest, RecordResponse, RemoveRecordRequest, UpdateRecordRequest,
};

// Server-only
#[cfg(feature = "server")] pub mod auth;
#[cfg(feature = "server")] pub mod middleware;
#[cfg(feature = "server")] pub mod rate_limit;
#[cfg(feature = "server")] pub mod types;
#[cfg(feature = "server")] pub mod zones;
#[cfg(feature = "server")] pub mod records;
#[cfg(feature = "server")] pub mod metrics;

#[cfg(feature = "server")]
pub use types::{ApiError, AppState, ErrorResponse};
```

Keep the existing `pub use bindcar::ZoneConfig` etc. paths working from outside the crate. Document the rename of internal modules in the CHANGELOG, but the **publicly re-exported symbols stay at the same `bindcar::Foo` paths** so consumer code is unaffected.

### What about the `#[derive(ToSchema)]` on type structs?

`utoipa::ToSchema` derive is what couples zone/record types to utoipa today. After the split, the type structs in `zones_types.rs` should NOT derive `ToSchema` directly — instead, mirror the OpenAPI schemas in `zones.rs` HTTP handlers:

Option A — feature-gate the derive:
```rust
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct ZoneConfig { ... }
```
This is the smallest diff and keeps OpenAPI generation working when the server feature is on. Recommended.

Option B — strip ToSchema from types entirely and define mirror schemas in `zones.rs`. More duplication; only worth it if utoipa version churn becomes painful.

Go with Option A.

### `metrics.rs`

Inspect whether `metrics.rs` registers a router or only constructs a `prometheus::Registry`. If it has axum integration (e.g. exposes a route handler), feature-gate the route function with `#[cfg(feature = "server")]` while keeping the registry always-available. Pure metric counters/gauges should not be gated — bindy might want to publish bindcar-defined metric names from its own metrics endpoint.

## Migration plan

### Phase 1 — split mixed files (no feature gate yet)

1. Create `src/zones_types.rs`, move pure structs + constants out of `zones.rs`. Re-export from `zones.rs` so handlers still compile.
2. Same for `src/records_types.rs`.
3. Run `cargo-quality` skill (fmt + clippy + test). Confirm no behavioural change.
4. Commit. This is a pure refactor — safe to merge independently.

### Phase 2 — introduce the feature flag

1. Edit `Cargo.toml` per the design above. `default = ["server"]` so nothing changes for default users.
2. Add `#[cfg(feature = "server")] pub mod ...` gates in `lib.rs`.
3. Add `#![cfg(feature = "server")]` at the top of `main.rs` (so `cargo build --no-default-features` doesn't try to build the binary). Alternatively, keep `main.rs` always buildable but stub it to a friendly error when the feature is off — depends on how `cargo install bindcar --no-default-features` should behave (probably: error message saying `--features server` is required).
4. Apply the `#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]` pattern to type structs.
5. Move `axum`, `tower`, `tower-http`, `utoipa`, `utoipa-swagger-ui`, `tower_governor` to `optional = true`.

### Phase 3 — verify

```bash
cargo build                         # default features (server on)
cargo build --no-default-features   # types + RNDC + parsers only
cargo build --all-features          # server + k8s-token-review
cargo test                          # default features
cargo test --no-default-features    # type-only tests
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
```

Each invocation must succeed cleanly.

### Phase 4 — CI

Add a `cargo build --no-default-features` job to the bindcar CI matrix so we don't regress.

### Phase 5 — release

1. Bump `Cargo.toml` to `0.7.0` (minor — adding features is non-breaking; module reorg is breaking only for code that imported `bindcar::auth::Foo` directly, which we'll call out).
2. CHANGELOG entry with `**Author:**` (per project convention) listing:
   - new features `server`, `k8s-token-review`
   - default features unchanged
   - module rename if any (`zones_types`, `records_types`)
   - migration note for type-only consumers
3. `cargo publish`.

### Phase 6 — adopt in bindy

In `/Users/erick/dev/bindy/Cargo.toml`:

```toml
bindcar = { version = "0.7", default-features = false }
```

Verify with `cargo tree -i sha2:0.10.9` — the only remaining 0.10 should disappear, since `rust-embed-utils` is no longer in bindy's tree.

Then bump bindy's CHANGELOG, run cargo-quality + cargo audit, push.

## Risk analysis

| Risk | Mitigation |
|---|---|
| Module split breaks downstream code that imports from internal paths (`bindcar::zones::SoaRecord`) | Keep `pub use` re-exports in `lib.rs` at the same paths. The public `bindcar::SoaRecord` etc. stay stable. Internal-path imports were always allowed but never documented. |
| `cargo build --no-default-features` succeeds locally but breaks on CI due to a missed `#[cfg]` | Add the `--no-default-features` build to CI in Phase 4 so future PRs catch it. |
| Some downstream consumer opted into `default-features = false` already and relied on a feature being silently absent | Unlikely — bindcar 0.6 has no feature flags besides `k8s-token-review`. Document the change clearly in CHANGELOG. |
| `utoipa::ToSchema` removal from type structs in some forgotten path | The `cfg_attr` approach (Option A) keeps the derive present whenever the server feature is on. Risk applies only if a consumer tried to use ToSchema *without* the server feature, which has no use case. |
| `prometheus` registry leak — bindy starts depending on counters/gauges that bindcar then moves behind the feature | Audit `metrics.rs` carefully in Phase 1; document which metric names are public API vs. server-only. |

## Verification checklist

- [ ] `cargo build --no-default-features` succeeds
- [ ] `cargo build` (default) succeeds and produces the same binary as today (smoke-test a few API calls)
- [ ] `cargo build --all-features` succeeds
- [ ] `cargo test` and `cargo test --no-default-features` both pass
- [ ] `cargo clippy -- -D warnings` clean under both feature sets
- [ ] `cargo doc --no-deps` builds cleanly under both feature sets
- [ ] Public symbol paths (`bindcar::ZoneConfig`, `bindcar::DnsRecord`, etc.) unchanged
- [ ] In bindy, `cargo tree -i sha2 | wc -l` drops by the expected amount; `rust-embed`, `utoipa-swagger-ui`, `tower-http`, `tower_governor` no longer appear in `cargo tree`
- [ ] In bindy, full test suite still passes after switching to `default-features = false`
- [ ] CHANGELOG updated with `**Author:**` per project rule

## Estimate

- Phase 1 (split mixed files): 1–2 hours
- Phase 2 (introduce feature): 1–2 hours
- Phase 3–4 (verify + CI): 30–60 min
- Phase 5 (release): 15 min
- Phase 6 (adopt in bindy): 15 min + verification

**Total: half a day of focused work.**

## Out-of-scope follow-ups

Documented here so they don't get lost:

1. **Split into `bindcar-types` + `bindcar`** — revisit if more consumers (operators, controllers, CLIs) need the types-only library. Once 3+ consumers are on `default-features = false`, the case for a separate crate strengthens.
2. **Drop `utoipa-swagger-ui`** — independent of this work. Could move to RapiDoc, Scalar, or a CDN-served Swagger UI shell. Would eliminate `rust-embed` from the bindcar binary too.
3. **Upstream PR to `pyrossh/rust-embed`** bumping `rust-embed-utils` to `sha2 = "0.11"`. Helps the broader ecosystem; complementary to this work.

## Tracking

- Owner: Erick
- Linked downstream item: `/Users/erick/dev/bindy/docs/roadmaps/hickory-client-stable-upgrade.md` — different scope, but same broader goal of trimming bindy's dep graph.
- Target release: bindcar 0.7.0
- Re-evaluation if blocked: 2026-06-01
