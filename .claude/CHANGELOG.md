# Changelog

## [2026-07-04 13:10] - Fix nsupdate builder COPY: remap /lib to /usr/lib (usrmerge)

**Author:** Erick Bourgeois

### Changed
- `docker/Dockerfile`, `docker/Dockerfile.chainguard`: the nsupdate builder now
  stages libs under `/staging/usr/...` only, remapping any `ldd`/`LD_TRACE` dep
  resolved under `/lib` or `/lib64` to `/usr/lib`. Previously `cp --parents`
  created a real `/staging/lib` directory, and `COPY --from=nsupdate /staging/ /`
  failed with `cannot copy to non-directory: .../lib` because the runtime bases'
  `/lib` is a usrmerge **symlink** to `/usr/lib`.

### Why
The first CI build of the distroless image reached the final COPY and failed on
the `/lib` symlink collision. Debian's `bind9-dnsutils` resolves several deps
(e.g. `/lib/x86_64-linux-gnu/…`) under the symlinked `/lib`.

### Verified
Restaged from the Debian builder with the remap: top-level staged dir is `usr`
only (no `/staging/lib`), and the staged `nsupdate 9.20.23` executes in the real
`cc-debian13` base. A11 digest pins intact; `make regression` green.

### Impact
- [x] Fixes the docker build (distroless + chainguard nsupdate bundling)
- [ ] Documentation only

## [2026-07-04 12:40] - Drone integration test uses ISC bind9 image

**Author:** Erick Bourgeois

### Changed
- `integration-test/drone-external-bind9.sh`: default `BIND9_IMAGE` switched from
  `ubuntu/bind9:latest` to the ISC official `internetsystemsconsortium/bind9:9.18`
  (matching `kind-e2e.sh`). The ISC entrypoint is `named -u bind` (no foreground
  flag; its default CMD is replaced when we pass args), so the `docker run` args
  now pass `-g -c /etc/bind/named.conf` (foreground + log to stderr) instead of
  just `-c /etc/bind/named.conf`.

### Why
Standardize both integration harnesses on the canonical, version-pinned ISC
image. `ubuntu/bind9` is a downstream repackage; ISC is upstream.

### Verified
Ran the drone integration test end-to-end against `internetsystemsconsortium/bind9:9.18`:
zone create (201) → SOA present → record add (201) → dig resolves 1.2.3.4 →
delete → absent → zone delete → absent. PASSED.

### Impact
- [ ] Breaking change
- [x] Test harness only
- [ ] Documentation only

## [2026-07-04 12:15] - Bundle nsupdate in the published Chainguard + distroless images

**Author:** Erick Bourgeois

### Changed
- `docker/Dockerfile.chainguard`: added a Wolfi builder stage (`bind-tools`) that
  stages `nsupdate` + its shared-library deps (glibc core excluded — the
  glibc-dynamic runtime provides it), then `COPY --from=nsupdate /staging/ /`
  into the runtime. Wolfi matches the glibc-dynamic runtime's layout so libs land
  in `/usr/lib` (default search path — no `LD_LIBRARY_PATH`, so the A-5 child-env
  scrubbing is unaffected). Builder digest pinned (A11).
- `docker/Dockerfile`: same technique with a Debian 13 builder (`bind9-dnsutils`)
  matching the `cc-debian13` runtime. Builder digest pinned (A11).
- Lib enumeration uses `LD_TRACE_LOADED_OBJECTS=1 <binary>` (the glibc loader's
  built-in trace) rather than `ldd`, which Wolfi does not ship.

### Why
After reverting record management to the `nsupdate` CLI, the runtime image must
contain `nsupdate`. The published Chainguard/distroless images did not, so record
operations failed with HTTP 500 "Failed to spawn nsupdate process" — caught by
the kind e2e (which runs the pushed Chainguard image): zone create (RNDC) passed,
record add 500'd. chef/.local already had `bind-tools`; this closes the gap for
the two published variants.

### Verified
Staged `nsupdate` + non-core libs from the Wolfi builder and executed it inside
the real `cgr.dev/chainguard/glibc-dynamic` base (base glibc + staged bind libs
only) — `nsupdate 9.20.24` ran. `make regression` green; all four FROM lines
across both Dockerfiles are `@sha256`-pinned (A11).

### Impact
- [x] Larger published images (add nsupdate + ~24 bind/openssl libs)
- [x] Fixes record management in the Chainguard/distroless images + the CI e2e
- [ ] Documentation only

## [2026-07-04 11:30] - Fix CI: deploy-validate uses grep, not rg

**Author:** Erick Bourgeois

### Changed
- `scripts/validate-deploy.sh`: replaced `rg` (ripgrep) with POSIX `grep -E` for
  the A5/A11/A12 checks. GitHub Actions runners do not ship ripgrep, so
  `make regression` failed in CI with `rg: command not found` (and the
  `rg ... || fail` falsely reported the PSA check failing). Committed CI scripts
  must be portable; the `rg` convention is for interactive dev search.

### Impact
- [x] CI/CD only (fixes the failing `test` job)


## [2026-07-04 11:00] - Revert hickory migration back to the nsupdate CLI

**Author:** Erick Bourgeois

### Changed
- `src/nsupdate.rs`, `src/nsupdate_test.rs`: restored the subprocess
  implementation (shells out to the `nsupdate` binary with a 0600 TSIG key file
  via `-k`, scrubbed child env, control-char injection guard) from before the
  hickory migration.
- `Cargo.toml` / `Cargo.lock`: removed `hickory-client`, `hickory-proto`, and
  `base64` (kept `sha2`, used by auth). Dependency count 342 → 326.
- `docker/Dockerfile.chef`, `docker/Dockerfile.local`: re-added `bind-tools`
  (provides `nsupdate`); kept the earlier `ENV DISABLE_AUTH` removal.
- `src/records.rs`, `docs/src/user-guide/managing-records.md`: reverted the
  hickory-era wording back to describing `nsupdate`.
- Removed `.cargo/audit.toml` (no longer needed — the RUSTSEC advisories came
  from `hickory-proto`).

### Why
`hickory-proto 0.25.2` carries RUSTSEC-2026-0118 and -0119, and `hickory-client`
0.26 (which pulls the fixed proto ≥0.26.1) is not yet a stable release — so the
`cargo audit` CI job failed with no clean upgrade path. Reverting to the CLI
drops the heavy DNS dependency and its advisories entirely, and `nsupdate` ships
with BIND9. `cargo audit` now passes.

### Impact
- [x] Removes a Rust dependency (hickory) and its RUSTSEC advisories
- [x] Record management again requires the `nsupdate` binary in the image
- [ ] Documentation only

## [2026-07-04 10:15] - Consolidate CI: single `build.yaml` (pr + main + release)

**Author:** Erick Bourgeois

### Changed
- `.github/workflows/build.yaml`: new single "Build" workflow replacing `pr.yml`,
  `main.yaml`, and `release.yml` (deleted), modelled on the 5-spot pattern. One
  workflow triggered on `pull_request` + `push` (main) + `release` +
  `workflow_dispatch`, with jobs gated by `github.event_name`:
  - always: license-check, verify-commits, extract-version, build, test, security
  - pull_request only: format, clippy
  - non-release (PR + push): docker (build/push), kind-e2e, drone-integration,
    coverage
  - release only: docker-release (semver + Cosign), sign-artifacts, SLSA
    provenance, upload-release-assets, publish-crate
  Top-level `permissions: contents: read`; jobs escalate as needed. `verify-mode`
  and image tagging are selected per event. actionlint-clean (one benign SC2129
  style note — the standard `>> "$GITHUB_OUTPUT"` idiom).
- `README.md`, `docs/src/index.md`: replaced the three CI badges with one Build
  badge. `docs/src/developer-guide/testing.md`: point at `build.yaml`.
- `integration-test/kind-e2e.sh`: added SRV and CAA record add→dig→delete
  assertions (exercise the native master-file RData parser end-to-end).

### Impact
- [ ] Breaking change
- [x] CI/CD only (workflow consolidation)
- [ ] Documentation only

## [2026-07-04 09:20] - Complete the hickory migration: all record types native (SRV/CAA)

**Author:** Erick Bourgeois

### Changed
- `src/nsupdate.rs`: `build_rdata` now uses hickory's master-file record-data
  parser (`RData::try_from_str`) instead of a hand-rolled per-type match. This
  supports **every** record type hickory can parse — including the previously
  unsupported **SRV and CAA** — using the same textual format `nsupdate`
  accepted. Removed the per-type rdata imports/prototype limitation.
- `src/records.rs`: corrected the validation rationale comments — control-char
  rejection now protects the zone-file rendering path (C-2) and DNS-name
  validity, not the (now-removed) nsupdate command-script sink (B-2).
- `docs/src/user-guide/managing-records.md`: clarified that record updates are
  sent natively (in-process DNS client, TSIG-signed), not via the `nsupdate`
  binary — no `bind-tools` needed in the image.

### Notes
- The `NsupdateExecutor` type, `src/nsupdate.rs` module, and `NSUPDATE_*` env
  vars are retained as the public/deployment contract (bindy depends on them);
  only the implementation is native now. `NSUPDATE_TCP` is accepted but ignored
  (native updates always use TCP).

### Impact
- [x] Fixes the SRV/CAA breaking limitation from the prototype (all types work)
- [x] Removes the nsupdate binary dependency from all images (complete)
- [ ] Documentation only

## [2026-07-04 08:50] - kind e2e tears down the cluster on success

**Author:** Erick Bourgeois

### Changed
- `integration-test/kind-e2e.sh`: on a successful run the kind cluster is now
  deleted regardless of whether it was created or reused (previously a *reused*
  cluster survived because teardown was gated on `CREATED_CLUSTER`). Failure
  still leaves the cluster up for investigation; `KEEP_CLUSTER=true` still keeps
  it. Removed the now-dead `CREATED_CLUSTER` variable.

### Impact
- [ ] Breaking change
- [x] CI/CD only (e2e cleanup)
- [ ] Documentation only

## [2026-07-04 08:45] - kind e2e: validate native updater; switch to ISC bind9:9.18

**Author:** Erick Bourgeois

### Changed
- `integration-test/kind-e2e.sh`: switched the DNS server image from
  `ubuntu/bind9:latest` to the ISC official `internetsystemsconsortium/bind9:9.18`
  (canonical, version-pinned). ISC's ENTRYPOINT is `named -u bind` with no
  foreground flag, so the pod args now pass `-g` (the ubuntu image added it
  itself). Also: recreate the pod on a reused cluster (delete before apply) so it
  picks up a freshly-loaded image and regenerated secrets; create `bindy-system`
  before the server-side manifest dry-run and make that step a real gate;
  `imagePullPolicy: IfNotPresent` for bind9; removed backticks from a heredoc
  comment that the shell was command-substituting.

### Verified
Ran the full kind e2e end-to-end (native RFC 2136 path, no nsupdate binary):
zone create → SOA present → **record add (HTTP 201) → dig confirms 1.2.3.4** →
record delete → dig absent → zone delete → absent. The native hickory-client
TSIG-signed UPDATE is accepted by BIND9. Path B works.

### Impact
- [ ] Breaking change
- [x] CI/CD only (e2e harness)
- [ ] Documentation only

## [2026-07-03 08:10] - Prototype: native RFC 2136 record updates (retire the nsupdate subprocess)

**Author:** Erick Bourgeois

### Changed
- `src/nsupdate.rs`: reimplemented `NsupdateExecutor` to perform DNS UPDATE
  (RFC 2136) natively via `hickory-client` with TSIG signing, instead of
  shelling out to the external `nsupdate` binary. Public API unchanged
  (`new`/`add_record`/`remove_record`/`update_record`), so `src/records.rs` and
  `src/main.rs` are untouched. Retired all the subprocess machinery
  (`create_tsig_key_file`, `build_nsupdate_args`, `minimal_child_env`,
  `reject_injection_chars`) — the B-2/B-7/A-5 hardening is moot with no child
  process.
- `Cargo.toml`: added `hickory-client` + `hickory-proto` (feature `dnssec-ring`
  for TSIG HMAC) and `base64` (decode the TSIG secret).
- `src/nsupdate_test.rs`: replaced the subprocess tests with native unit tests
  (TSIG algorithm mapping, RData construction, executor/TSIG validation).
- `docker/Dockerfile.chef`, `docker/Dockerfile.local`: removed `bind-tools` — no
  external `nsupdate` binary is needed anymore. The published **chainguard +
  distroless** images now support record management with no changes at all.
- `Makefile`: `manifest-dry-run` now only runs when a cluster is reachable
  (modern kubectl needs API discovery even for client dry-run) — this also fixes
  the CI Test job (no cluster) which `make regression` would otherwise fail.

### Why
Answers "why is nsupdate required at all": it shouldn't be. Records now use a
native client like zones use native RNDC — removing the per-image binary
dependency, the subprocess, and its injection surface.

### Known limitation (prototype)
- SRV and CAA record types are not yet built natively (`build_rdata` returns an
  error); A/AAAA/CNAME/NS/PTR/TXT/MX are supported. TSIG limited to
  hmac-sha256/384/512 (hickory-supported; matches the RNDC policy).
- The live DNS-UPDATE round-trip (TSIG-on-the-wire against BIND) is not covered
  by unit tests — verify via the kind e2e.

### Impact
- [x] Breaking change (SRV/CAA record types temporarily unsupported)
- [ ] Requires cluster rollout
- [x] Removes a runtime dependency (nsupdate binary) from all images
- [ ] Documentation only

## [2026-07-03 07:35] - Bundle nsupdate in chef image (record management runtime dep)

**Author:** Erick Bourgeois

### Changed
- `docker/Dockerfile.chef`: add `bind-tools` (provides `nsupdate`) to the runtime
  stage. bindcar shells out to `nsupdate` for dynamic record add/remove/update
  (`src/nsupdate.rs:168`); without it those endpoints 500 with "Failed to spawn
  nsupdate process". Found by running the kind e2e to the record step.

### Open finding (needs decision)
The **published** images — `docker/Dockerfile.chainguard` and `docker/Dockerfile`
(distroless) — also do NOT include `nsupdate`, so record management is broken in
those deployments too. Distroless has no package manager, so this needs an
explicit approach (COPY the `nsupdate` binary + libs from a builder, or use a
base that bundles bind-tools). Not changed here pending a decision.

### Impact
- [ ] Breaking change
- [x] CI/CD only (chef image build; e2e)
- [ ] Documentation only

## [2026-07-03 07:20] - Fix kind e2e root cause: build a Linux image (not host Mach-O)

**Author:** Erick Bourgeois

### Changed
- `integration-test/kind-e2e.sh`: build the local e2e image via
  `docker/Dockerfile.chef` (compiles a statically-linked musl **Linux** binary
  inside the builder) instead of `docker/Dockerfile.local`, which COPYs the
  host-built binary — on macOS that is a Darwin Mach-O binary, so the container
  crash-looped with `exec /usr/local/bin/bindcar: exec format error` on the
  aarch64 Linux kind node. Also removed backticks from a comment inside the
  unquoted `<<YAML` pod heredoc that the shell was evaluating (`rndc: command not
  found`).

### Why
Ran the kind e2e end-to-end (with permission to `kind load`). bind9 came up
cleanly; bindcar crash-looped purely due to the wrong-architecture binary baked
by Dockerfile.local on a macOS host. CI is unaffected — it uses the real
multi-arch Chainguard image.

### Impact
- [ ] Breaking change
- [x] CI/CD only (local kind e2e image build)
- [ ] Documentation only

## [2026-07-02 11:45] - Fix kind e2e run-only failures + BuildKit lint

**Author:** Erick Bourgeois

### Changed
- `integration-test/kind-e2e.sh`: deploy the pod under an explicit,
  synchronously-created `e2e-runner` ServiceAccount (`automountServiceAccountToken:
  false`) instead of the namespace `default` SA — that SA is provisioned by a
  controller that is slow/starved on a loaded machine, so the pod was rejected
  with "serviceaccount default not found" even after a 76s wait; bump
  `kind create --wait` to 300s and add an explicit
  `kubectl wait --for=condition=Ready nodes` (control-plane Ready timed out on a
  slow machine); drop the extra `-g` from the bind9 args to match the proven
  drone-test invocation (the ubuntu/bind9 entrypoint already runs foreground).
- `docker/Dockerfile.local`, `docker/Dockerfile`, `docker/Dockerfile.chainguard`:
  removed the redundant `ENV DISABLE_AUTH=false` (the binary already defaults it
  to false) which tripped BuildKit's `SecretsUsedInArgOrEnv` lint.

### Why
First real `make regression-full` run surfaced a namespace/ServiceAccount race
and a node-readiness timeout in the kind harness, plus a Docker build warning.

### Impact
- [ ] Breaking change
- [x] CI/CD only (kind e2e harness + image build hygiene)
- [ ] Documentation only

## [2026-07-02 11:15] - Wire kind e2e into CI reusing the pushed image

**Author:** Erick Bourgeois

### Changed
- `.github/workflows/pr.yml`: new `kind-e2e` job that `needs: [docker,
  extract-version]` — it runs AFTER the image is built+pushed and consumes that
  exact artifact (no rebuild). Logs into ghcr, installs kind, and runs
  `make kind-e2e BINDCAR_IMAGE=ghcr.io/<repo>:<tag> SKIP_IMAGE_BUILD=true` using
  the `image-repository-chainguard` / `image-tag-chainguard` outputs.
- `integration-test/kind-e2e.sh`: when `SKIP_IMAGE_BUILD=true`, pulls
  `BINDCAR_IMAGE` if not already local, then `kind load`s it — so the same
  script serves local (build Dockerfile.local) and CI (reuse pushed image).
- `Makefile`: `kind-e2e` now accepts `BINDCAR_IMAGE` / `SKIP_IMAGE_BUILD` (and
  `KIND_CLUSTER`) as overridable vars and forwards them to the script; added
  `?=` defaults (`bindcar:e2e`, `false`). Fixed the `help` target regex to
  include digits so `kind-e2e` (and any numeric target) is listed.

### Why
The kind e2e must exercise the real published image, not a throwaway rebuild —
so it depends on the docker build stage and receives the image name as a make
var, reusing the existing pipeline (binaries → images → e2e) end to end.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] CI/CD only (new kind-e2e PR job consuming the pushed image)
- [ ] Documentation only

## [2026-07-02 10:30] - Restructure test targets: `unit-tests`, `manifest-dry-run`, kind e2e

**Author:** Erick Bourgeois

### Changed
- `Makefile`: renamed `test-all` → `unit-tests` (kept `test-all` as an alias) —
  unit + doctests for both feature sets, **no cluster**. Added `manifest-dry-run`
  (client-side `kubectl apply --dry-run=client -f deploy/`, skipped if kubectl
  absent). `regression` is now the **no-cluster** gate (fmt-check, clippy-all,
  unit-tests, deploy-validate, manifest-dry-run). Added `kind-e2e`;
  `regression-full` = `regression` + `kind-e2e` (was regression + the Docker
  drone test).
- `integration-test/kind-e2e.sh`: new end-to-end suite that runs ON a kind
  cluster — deploys a Pod with BIND9 (unprivileged :5353) + the bindcar sidecar
  (`0.0.0.0:8080` with `BIND_API_TOKEN`, exercising the non-loopback auth
  startup guard), then port-forwards and drives the full zone/record lifecycle
  (create zone → dig SOA → add A → dig → delete → dig absent) plus an
  unauthenticated-request-rejected (401) check, and server-side-validates
  `deploy/rbac.yaml` + `deploy/networkpolicy.yaml`.

### Why
Make the test taxonomy explicit: `unit-tests` (no cluster), `regression` (no
cluster, CI gate), `regression-full` (adds kind e2e). `make test` alone only
ran default features and no cluster-level validation.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] CI/CD only (new/renamed make targets; kind e2e requires docker+kind+kubectl)
- [ ] Documentation only

## [2026-07-02 09:00] - Full regression suite: `make regression`

**Author:** Erick Bourgeois

### Changed
- `Makefile`: added `regression` (and `regression-full`) plus building blocks
  `fmt-check`, `clippy-all`, `test-all`, `deploy-validate`. `make regression`
  runs the entire static+unit suite for **both** feature sets: `cargo fmt
  --check`, clippy (`-D warnings`) for default AND `k8s-token-review`, all
  tests+doctests for default AND `k8s-token-review`, and the deploy-invariant
  checks. `regression-full` additionally runs the drone integration test.
- `scripts/validate-deploy.sh`: new static validator asserting the RED-team
  hardening cannot silently regress — YAML validity, PSA `restricted` (A5),
  scoped NetworkPolicy egress / no implicit `0.0.0.0/0` (A6), no
  `system:auth-delegator` roleRef + minimal `bindcar-tokenreview` role (A10),
  digest-pinned production Dockerfiles (A11), SHA-2-only RNDC HMAC (A12). Self-
  installs PyYAML if absent so CI stays a clean `make` call.
- `.github/workflows/pr.yml`: the **Test** job now runs `make regression`
  (was `make test`), so every PR is checked against both feature sets +
  doctests + deploy invariants. This also enforces `cargo fmt --check` in CI
  (the standalone `format` job runs mutating `make fmt`, which never failed on
  unformatted code).

### Why
`make test` / CI ran only the default feature set and never validated the
deploy manifests. `make regression` is the single command that exercises the
full matrix and locks in the security invariants.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] CI/CD only (new `make regression` target; suggest CI call it)
- [ ] Documentation only

## [2026-07-01 16:20] - Bump firestoned/github-actions v1.3.6 → v1.3.7 (cargo login --stdin fix)

**Author:** Erick Bourgeois

### Changed
- `.github/workflows/{main.yaml,release.yml,pr.yml,docs.yaml}`: bumped all 33
  `firestoned/github-actions/*` composite-action refs from
  `53b48325… # v1.3.6` to `d0d51c63… # v1.3.7` (SHA-pinned).

### Why
`cargo login` no longer accepts the `--stdin` flag; the `rust/publish-crate`
action in v1.3.6 ran `cargo login --stdin`, which fails on current toolchains.
v1.3.7 (firestoned/github-actions #21) pipes the token into `cargo login`
without the removed flag. Per repo CI standards, the fix lives in the
`firestoned/github-actions` repo and is consumed here by version bump — the
composite actions are NOT inlined/replaced.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] CI/CD only (unblocks `cargo publish` in the release workflow)
- [ ] Documentation only

## [2026-07-01 15:45] - RED-team sweep remediation (batch 2): MEDIUM + LOW findings A7–A19

**Author:** Erick Bourgeois

### Changed
- `src/auth.rs`: **(A7)** auth failures now return a generic `"Unauthorized"` to the
  client (detail logged server-side only), removing the identity/namespace
  enumeration oracle. **(A8)** the `kube::Client` is built once and cached in a
  `OnceCell` (`cached_kube_client`) instead of rebuilt per request. **(A15)**
  `compare_shared_secret` compares fixed-size SHA-256 digests so timing no longer
  leaks the secret's length.
- `src/rndc.rs`: **(A9)** every `RndcExecutor` zone method re-validates the zone
  name at the sink (`validate_rndc_zone_name`) — defense-in-depth if a caller
  guard regresses. **(A12)** deprecated `hmac-md5`/`hmac-sha1` rejected;
  SHA-2 only (`ACCEPTED_RNDC_ALGORITHMS`). **(A18)** deterministic key selection —
  errors when multiple keys exist with no `default-key` instead of picking an
  arbitrary HashMap entry. **(A19)** rejects an empty TSIG secret. Named the
  `DEFAULT_RNDC_PORT` constant.
- `src/rndc_conf_parser.rs`: **(A16)** `port_number` errors on `u16` overflow
  instead of silently masking to the default. **(A17)** `MAX_INCLUDE_DEPTH` caps
  nested `include` recursion (stack-overflow DoS).
- `src/main.rs`: **(A13)** Swagger UI / OpenAPI spec are now off by default and
  only served when `BIND_ENABLE_DOCS=true` (they are unauthenticated).
- `deploy/rbac.yaml`: **(A10)** replaced the `system:auth-delegator` binding with a
  purpose-built ClusterRole granting only `create tokenreviews` (drops the unused
  `subjectaccessreviews` recon primitive).
- `deploy/networkpolicy.yaml`: **(A14)** documented that `/metrics` shares port
  8080 (already operator-restricted) and how to scope Prometheus scraping.
- `docker/Dockerfile.chainguard`: **(A11)** pinned the base image to its multi-arch
  manifest-list digest
  `@sha256:ea9eab0adc5716fb9937ab60155a31bce9cbc8b56e6f2e21fb9af9218be195b7`
  (OCI image index, amd64+arm64) and updated the `base.name` label to match.
- `Cargo.toml`: added `sha2 = "0.10"` (already in the lock graph transitively) for
  the length-independent shared-secret comparison (A15).
- Tests: added regressions for A9 (`validate_rndc_zone_name`), A12 (weak-HMAC
  reject / SHA-2 accept), A15 (length-independent compare), A16 (port overflow),
  and updated the two existing algorithm tests to the SHA-2-only policy.

### Why
Close the remaining RED-team findings: auth-path DoS/oracle (A7/A8), TSIG/secret
and control-channel hardening (A9/A12/A18/A19), parser robustness (A16/A17),
unauthenticated recon surface (A13), and cluster-privilege minimization (A10).
A20 (`DISABLE_AUTH=true` gate) is already enforced by the existing
`check_startup_auth_posture` startup guard on non-loopback binds.

### Impact
- [x] Breaking change (RNDC keys using `hmac-md5`/`hmac-sha1` now rejected —
      re-key with SHA-2; multi-key `rndc.conf` without `default-key` now errors;
      Swagger UI off unless `BIND_ENABLE_DOCS=true`)
- [ ] Requires cluster rollout
- [x] Config change (RBAC ClusterRole change; new `BIND_ENABLE_DOCS`)
- [ ] Documentation only

## [2026-07-01 15:10] - RED-team sweep remediation: auth confused-deputy chain, TSIG secret disclosure, deploy hardening

**Author:** Erick Bourgeois

### Changed
- `src/auth.rs`: **(A1)** `validate_token_with_k8s` now enforces audience binding —
  after `status.authenticated == true` it requires a non-empty intersection
  between the requested `spec.audiences` and the returned `status.audiences`
  (new `audiences_compatible` helper). A valid token minted for a different
  audience (e.g. any pod's default SA token) is now rejected instead of accepted.
- `src/auth.rs`: **(A2)** added `TokenReviewConfig::is_authorization_restricted`,
  `check_authorization_posture`, and the `BIND_ALLOW_ANY_SERVICEACCOUNT` env
  (`ALLOW_ANY_SERVICE_ACCOUNT_ENV`). Empty namespace/SA allowlists = allow-all.
- `src/main.rs`: **(A2)** startup now fails closed — with `k8s-token-review`
  enabled and no allowlist, bindcar refuses to start unless
  `BIND_ALLOW_ANY_SERVICEACCOUNT=true` is set (loud warning if so).
- `src/rndc_conf_parser.rs`: **(A3)** `parse_rndc_conf_str` no longer echoes the
  unparsed remainder (which can contain the TSIG `secret "..."`) into errors, and
  now rejects trailing unparsed input with a position-only message instead of
  silently dropping the malformed key (which yielded an empty-secret client).
  `KeyField` gets a redacting `Debug`.
- `src/rndc.rs`, `src/rndc_conf_types.rs`: **(A4)** `RndcConfig` and `KeyBlock` use
  a manual `Debug` that prints `secret: "[REDACTED]"` so the TSIG key cannot leak
  via `{:?}` / panic / `.context()`.
- `src/auth_test.rs`, `src/rndc_conf_parser_tests.rs`, `src/rndc_conf_types_tests.rs`,
  `src/rndc_test.rs`: added regression tests for each (audience-bypass cases,
  fail-closed posture, secret-not-in-error, secret-not-in-Debug).
- `deploy/pod-hardening.yaml`: **(A5)** added an enforced Pod Security Admission
  `restricted` label set on the `bindy-system` Namespace (real, applyable
  backstop vs. the commented securityContext reference). **(A6)** scoped egress so
  no rule is `0.0.0.0/0`: DNS to kube-system, API-server 443/6443 to a
  fail-closed ipBlock placeholder that must be set to the real endpoint.
- `docs/src/operations/env-vars.md`: documented audience enforcement, the
  fail-closed allowlist behavior, and `BIND_ALLOW_ANY_SERVICEACCOUNT`.

### Why
RED-team sweep found a chainable confused-deputy: TokenReview trusted
`authenticated:true` without checking `status.audiences` (A1) and the allowlists
defaulted to allow-all (A2), so any valid cluster SA token granted full DNS
create/delete/modify — DNS hijack / MITM potential in a regulated environment.
A3/A4 close TSIG credential-disclosure paths. A5/A6 turn deploy hardening from
advisory comments into enforced controls.

### Impact
- [x] Breaking change (TokenReview deployments with empty allowlists must set an
      allowlist or `BIND_ALLOW_ANY_SERVICEACCOUNT=true`; tokens must carry the
      configured audience)
- [ ] Requires cluster rollout
- [x] Config change (new env var; deploy manifest egress must set the API-server CIDR)
- [ ] Documentation only

## [2026-07-01 17:30] - Validate zone name on all remaining {name} handlers (CodeQL #2 + sweep completion)

**Author:** Erick Bourgeois

### Changed
- `src/zones.rs`: `reload_zone`, `zone_status`, `freeze_zone`, `thaw_zone`,
  `notify_zone`, `retransfer_zone`, `get_zone`, and `modify_zone` now call
  `validate_zone_name(&zone_name)` before the caller-supplied `{name}` path
  parameter reaches the RNDC executor or is joined into a filesystem path —
  matching the existing guard in `create_zone`/`delete_zone`. Rejected names
  return HTTP 400 (`ApiError::InvalidRequest`); handlers that emit an operation
  metric also record a failed op (`zone_status`/`get_zone` record none).
- `src/zones_test.rs`: added an offline `AppState` builder and eight async tests
  asserting each handler rejects a path-traversal zone name before any sink.

### Why
Every handler taking a `{name}` path parameter now validates it consistently.
CodeQL flagged the freeze/thaw/notify/reload/status handlers
(`rust/path-injection` #2) which forwarded the raw name to an RNDC control
command. A follow-up branch security review found the same gap in
`retransfer_zone` and — more importantly — in `get_zone` and `modify_zone`,
which join the name into a filesystem path (`zone_dir/{name}.zone`) and call
`.exists()`: an unvalidated `../../..` name there was a genuine arbitrary-file
existence oracle (and `get_zone` echoed the resolved path back). The records
handlers (`add`/`remove`/`update_record`) were already covered via
`validate_zone_for_updates` → `validate_zone_name`.

### Impact
- [ ] Breaking change
- [x] API change (malformed zone names now return 400 instead of a 500 from rndc
  or a filesystem probe)
- [ ] Config change only
- [x] Security hardening (closes a path-traversal existence oracle in
  `get_zone`/`modify_zone`)

## [2026-07-01 00:00] - CodeQL rust/path-injection (#1, #3): defense-in-depth barrier on readiness zone-dir probe

**Author:** Erick Bourgeois

### Changed
- `src/zones.rs`: added `is_normalized_zone_dir(path)` — returns `true` only for
  an absolute path with no `..`/`.` traversal components (the invariant
  `resolve_zone_dir` establishes at startup).
- `src/main.rs`: `ready_check` now calls `is_normalized_zone_dir` as a guard
  immediately before `tokio::fs::metadata(&state.zone_dir)`. The metadata sink is
  only reachable after the barrier; an unexpectedly relative/traversal path is
  logged and reported as not-ready instead of being probed.
- `src/zones_test.rs`: added unit tests for the barrier (absolute/relative,
  `..` rejection, embedded `.` acceptance) and a coupling test asserting
  `resolve_zone_dir` output always satisfies `is_normalized_zone_dir`.

### Why
CodeQL `rust/path-injection` alerts #1 (main, `f0c43b8`) and #3
(security-sweep, `2ac1a19`) flag the same code: `state.zone_dir` reaches the
`/api/v1/ready` handler through the axum `State` extractor, which the analyzer
models as untrusted, and feeds `tokio::fs::metadata`. The value is actually
server-configured (`BIND_ZONE_DIR` env var, canonicalized once at startup), so
this is not a real user-controlled path. Rather than dismiss, we add a genuine
defense-in-depth barrier that re-asserts the startup invariant at the point of
use and breaks the tainted dataflow.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [x] Security hardening (no behavior change for a correctly-configured server)

## [2026-06-30 14:00] - RED team remediation: critical + high findings (B-8 zone/rndc injection, rate-limit keying, error/metric disclosure, child env, deploy hardening)

**Author:** Erick Bourgeois

### Changed
- `src/zones.rs`: added `validate_ip_list` (C-1) — `primaries`/`also-notify`/
  `allow-transfer` entries must parse as `IpAddr` before being interpolated into
  the `rndc addzone` config literal, blocking brace-breakout BIND config
  injection. Added `validate_zone_config_content` + `reject_zone_file_control_chars`
  (C-2) — SOA/name-server/glue/record fields embedded in a `create_zone` request
  are now validated (control-char reject + IP parse + the same record checks as
  the add-record endpoint) before `to_zone_file`, blocking `$INCLUDE`/`$GENERATE`
  zone-file directive injection. Both wired into `create_zone`.
- `src/records.rs`: `validate_record_type` made `pub(crate)` for reuse by zones.
- `src/main.rs`, `src/rate_limit.rs`: rate limiter now keys on the real TCP peer
  (`PeerIpKeyExtractor`) instead of the spoofable `SmartIpKeyExtractor`
  (X-Forwarded-For), closing rate-limit evasion and victim-bucket exhaustion (A-1).
- `src/types.rs`: `ApiError::into_response` returns a generic message for all 5xx
  variants (raw rndc/nsupdate stderr and internal paths are logged server-side
  only), closing an information-disclosure oracle (A-3). 4xx client-fault detail
  is unchanged.
- `src/middleware.rs`: `track_metrics` labels metrics with the matched route
  template (`MatchedPath`) instead of the raw request path — removes zone-name
  disclosure via `/metrics` and the unbounded-cardinality memory-exhaustion
  vector (A-4 / C-3).
- `src/main.rs`: the unauthenticated `/ready` and `/metrics` endpoints no longer
  leak internals (C-3). `/ready` returns only `zone_dir: ok|error` /
  `rndc: ok|error` (the path and backend error text are logged server-side via
  the new `ready_check_label` helper); the `/metrics` error path returns a
  generic message. These endpoints stay unauthenticated (kubelet probes /
  Prometheus scraping) but disclose nothing. Added `src/main_test.rs`.
- `src/nsupdate.rs`: the spawned `nsupdate` child env is scrubbed
  (`env_clear()` + `minimal_child_env()` allowlisting only `PATH`), so
  `NSUPDATE_SECRET`/`RNDC_SECRET` no longer leak via `/proc/<pid>/environ` (A-5).
- `docker/Dockerfile`: pinned the distroless runtime base to its multi-arch
  manifest-list digest (K-1) and added an explicit `USER 65532:65532`.
- `deploy/rbac.yaml` (new): least-privilege `system:auth-delegator` RBAC for
  TokenReview (K-2). `deploy/pod-hardening.yaml` (new): pod/container
  `securityContext` reference + egress NetworkPolicy (K-3 / N-1).
- Tests: `src/zones_test.rs` (+7), `src/types_test.rs` (+2), `src/nsupdate_test.rs`
  (+1) covering the new validators, error sanitization, and child-env allowlist.
- `README.md`: documented peer-IP rate-limit keying and the new deploy manifests.

### Why
Remediates the critical and high findings from the 2026-06-29/30 RED team sweep.
C-1/C-2 were the core of the unauthenticated DNS-takeover chain (config + zone
poisoning, `$INCLUDE` file-read oracle); A-1/A-3/A-4 collapse the rate-limit
bypass and the metrics/error information-disclosure surface; A-5 restores the
B-7 secret-confinement goal for the child process; K-1/K-2/K-3 give operators
digest-pinned bases and least-privilege/hardened deployment defaults.

### Impact
- [ ] Breaking change
- [x] API change (malformed embedded records/IP lists in `create_zone` now
  return HTTP 400; 5xx bodies no longer include internal detail)
- [ ] Config change only
- [ ] Documentation only

Verified: `cargo fmt`/`clippy --all-targets --all-features` clean; `cargo test`
286 (default) / 313 (`k8s-token-review`) passing.

## [2026-06-30 12:30] - Bump kube-rs 3.1.0 → 4.0.0

**Author:** Erick Bourgeois

### Changed
- `Cargo.toml`: `kube` dependency `"3.1"` → `"4.0"` (features unchanged:
  `client`, `rustls-tls`). `k8s-openapi` stays at `0.28` — kube 4.0 still targets
  it.
- `Cargo.lock`: `kube`/`kube-client`/`kube-core` `3.1.0` → `4.0.0`.
- `src/auth.rs`: added `..Default::default()` to the `NamedCluster`,
  `NamedAuthInfo`, and `NamedContext` initializers in `build_explicit_kube_client`.
  kube 4.0 added a flattened `other: BTreeMap<String, serde_json::Value>` field
  (unknown-kubeconfig-field round-tripping) to these wrappers, making the prior
  exhaustive initializers fail to compile.

### Why
Stay current with kube-rs. kube 4.0.0 (2026-06-16) is a major release but its
only breaking change touching this codebase is the new `Named*` `other` field;
the watcher/timeout/opt-in-tracing changes in 4.0 do not affect us (we only use
`Client`, `Config::from_custom_kubeconfig`, `Api::all`, and `.create()` for the
TokenReview flow). No `k8s-openapi` bump required.

### Impact
- [ ] Breaking change
- [ ] API change
- [x] Dependency bump (only affects builds with the `k8s-token-review` feature)
- [ ] Documentation only

Verified: `cargo check`/`clippy`/`test` pass both with and without
`--features k8s-token-review` (303 tests with the feature; `K8S_OPENAPI_ENABLED_VERSION=1.32`);
single `kube v4.0.0` in the dependency tree.

## [2026-06-30 00:00] - Canonicalize zone directory at startup (CodeQL path-injection)

**Author:** Erick Bourgeois

### Changed
- `src/zones.rs`: added `resolve_zone_dir`, which canonicalizes the configured
  zone directory, rejects a missing or non-directory path, and returns the
  normalized absolute path.
- `src/main.rs`: replaced the inline zone-directory existence check in
  `start_server` with `zones::resolve_zone_dir`, so `AppState.zone_dir` holds the
  canonicalized path that the `list_zones`/`ready_check` handlers reuse.
- `src/zones_test.rs`: added four unit tests covering canonicalization, `..`
  normalization, missing-path rejection, and non-directory rejection.

### Why
Resolves the two open CodeQL `rust/path-injection` (high) alerts on the
firestoned/bindcar code-scanning dashboard (`src/main.rs:147`,
`src/zones.rs:967`). `BIND_ZONE_DIR` is operator-controlled config, but CodeQL
treats environment variables as untrusted, and the value flowed unmodified into
`tokio::fs::read_dir`/`metadata`. Canonicalizing once at startup is genuine
defense-in-depth (symlinks and `..` resolved against the real filesystem, fail
fast on a bad path) and breaks the env→filesystem taint flow before the value
reaches any sink — the configuration-time counterpart to the existing
`validate_zone_name` guard (B-1) on per-request zone names.

### Impact
- [ ] Breaking change
- [ ] API change
- [x] Config change only (zone directory is now canonicalized; a `BIND_ZONE_DIR`
  that does not exist or is not a directory now fails fast at startup)
- [ ] Documentation only

## [2026-06-29 23:00] - SHA-pin all GitHub Actions + Dependabot SHA updates

**Author:** Erick Bourgeois

### Changed
- `.github/workflows/{docs.yaml,main.yaml,pr.yml,release.yml}`: pinned every
  external action `uses:` ref to a full commit SHA with a `# <tag>` comment
  (102 refs), including the first-party `firestoned/github-actions/*` composite
  actions. SHAs resolved from the live GitHub API.
- `.github/actions/prepare-docker-binaries/action.yaml`: pinned
  `actions/download-artifact@v4` to its commit SHA.
- `.github/dependabot.yml`: documented that the github-actions ecosystem now
  updates SHA pins (Dependabot bumps the SHA + version comment to the newest
  release) and covers the composite action directory.

### Why
Supply-chain hardening: a mutable tag (`@v4`, `@stable`) can be force-pushed to
point at malicious code, whereas a commit SHA is immutable. Pinning to SHA with
a `# vX` comment lets Dependabot keep them current by bumping the SHA itself.
Exceptions: the SLSA reusable workflow stays on a semver tag (required for
provenance verification), and the local `./.github/actions/...` action needs no
pin.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] CI/config change only
- [ ] Documentation only

## [2026-06-29 22:00] - Fix drone integration test for B-4 startup guard

**Author:** Erick Bourgeois

### Changed
- `integration-test/drone-external-bind9.sh`: added `BINDCAR_ALLOW_INSECURE_AUTH="true"`
  to the bindcar drone launch env block.

### Why
The B-4 startup guard refuses to start when bound to a non-loopback interface
(`0.0.0.0`, the default) without real authentication. The drone integration test
intentionally runs with `DISABLE_AUTH=true` in a trusted local environment, which
is exactly the case the explicit operator override exists for. Without the override
the guard exited before the API bound, so the test's "wait for API on 8080" timed
out. Verified locally: without the var bindcar refuses (logs "refusing to start");
with it the HTTP listener binds and `/api/v1/health` returns healthy.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [ ] Config change only
- [x] Test-only change

## [2026-06-29 21:00] - Bump anyhow to 1.0.103 (RUSTSEC-2026-0190)

**Author:** Erick Bourgeois

### Changed
- `Cargo.lock`: `anyhow` 1.0.102 → 1.0.103.

### Why
`cargo audit --deny warnings` failed in CI on RUSTSEC-2026-0190, an unsoundness
advisory in `anyhow::Error::downcast_mut()` (published 2026-06-25) affecting
1.0.102. 1.0.103 is the patched release. `cargo audit` now passes clean.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] Config change only (lockfile / dependency)
- [ ] Documentation only

## [2026-06-09 13:00] - TSIG key out of argv: nsupdate -k keyfile instead of -y (B-7)

**Author:** Erick Bourgeois

### Changed
- `Cargo.toml`: promoted `tempfile = "3"` from dev-dependency to runtime dependency
  (well-maintained, std-adjacent crate; creates 0600 temp files on Unix).
- `src/nsupdate.rs`: replaced `-y algorithm:keyname:secret` with `-k <keyfile>`.
  Added `create_tsig_key_file` (0600 `NamedTempFile`, removed immediately after
  nsupdate exits), `build_tsig_key_file_content` (renders the BIND key file with
  strict validation: key name allowlist, algorithm normalized + checked against
  the known HMAC list, secret restricted to base64 charset), and
  `build_nsupdate_args` (pure, testable argv builder).
- `src/nsupdate_test.rs`: `tsig_keyfile_tests` (8 tests) — argv never contains the
  secret, `-y` is gone, keyfile is 0600, content/normalization correct, file
  removed after use, malicious key-file fields rejected.

### Why
B-7 (2026-06-09 audit): the TSIG secret was passed on the nsupdate command line,
making it world-readable via `/proc/<pid>/cmdline` and process listings to any
process sharing the pod/host.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [ ] Documentation only

## [2026-06-09 12:00] - Real authentication by default: shared secret + startup guard + NetworkPolicy (B-4)

**Author:** Erick Bourgeois

### Changed
- `Cargo.toml`: added `subtle = "2.6"` (constant-time comparison).
- `src/auth.rs`: shared-secret auth layer (`BIND_API_TOKEN`) with constant-time
  `compare_shared_secret`/`validate_shared_secret`; added `is_loopback_host`,
  `has_real_auth`, and the pure `check_startup_auth_posture` guard decision. The
  `authenticate` middleware enforces the shared secret before optional TokenReview.
- `src/cli.rs`: added global `--i-know-this-is-insecure` flag.
- `src/main.rs`: startup guard refusing to run presence-only/disabled auth on a
  non-loopback bind unless overridden; configurable `BIND_API_ADDRESS`.
- `deploy/networkpolicy.yaml`: restrict the API (TCP 8080) to the bindy operator.
- `src/auth_test.rs`: `b4_auth_posture_tests` (loopback detection, constant-time
  match/reject, all guard branches).

### Why
B-4 (2026-06-09 audit): presence-only auth accepted any non-empty token and
`DISABLE_AUTH` only warned, silently exposing the privileged API to any in-cluster pod.

### Impact
- [x] Breaking change
- [ ] API change
- [ ] Config change only
- [ ] Documentation only

## [2026-06-09 11:00] - Input validation: path traversal + nsupdate/RNDC injection (B-1, B-2, B-3)

**Author:** Erick Bourgeois

### Changed
- `src/zones.rs`: `validate_zone_name` (strict DNS grammar — blocks path traversal,
  B-1) and `validate_rndc_identifier` (allowlist — blocks RNDC quote-breakout, B-3);
  wired into `create_zone`/`delete_zone`.
- `src/records.rs`: control-character rejection in `validate_record_value` (incl.
  TXT/CAA/SRV) + new `validate_record_name`; zone name validated in
  `validate_zone_for_updates`.
- `src/nsupdate.rs`: `reject_injection_chars` defense-in-depth at the nsupdate sink
  (zone/name/value) — blocks newline command injection (B-2).
- Tests in `src/zones_test.rs`, `src/records_test.rs`, `src/nsupdate_test.rs`.

### Why
B-1/B-2 (Criticals) + B-3 from the 2026-06-09 audit: unsanitized request fields
reached filesystem paths, the nsupdate command stream, and rndc config literals.

### Impact
- [x] Breaking change (malformed names/fields now rejected with HTTP 400)
- [ ] API change
- [ ] Config change only
- [ ] Documentation only

## [2026-03-29 21:00] - Move BYOB9 roadmap to forage project

**Author:** Erick Bourgeois

### Changed
- Moved `docs/roadmaps/byob9-bring-your-own-bind9.md` → `~/dev/forage/docs/roadmaps/byob9-bring-your-own-bind9.md`

### Why
The roadmap describes the forage binary, not bindcar. It belongs in the forage repository.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [x] Documentation only

## [2026-03-29 20:00] - Fix zone directory permissions for BIND9 container

**Author:** Erick Bourgeois

### Changed
- `integration-test/drone-external-bind9.sh`: Added `chmod 777 "$ZONE_DIR"` after `mkdir -p` so the `bind` user inside the container can write to the zone directory

### Why
On the CI runner the zone directory is created by the `runner` user. The BIND9 container runs `named` as user `bind` (uid 101), which has no write access to the host-owned directory. named aborts with `permission denied` when it can't write to its configured `directory`.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [x] CI/CD only

## [2026-03-29 19:00] - Fix drone integration test binary path

**Author:** Erick Bourgeois

### Changed
- `.github/workflows/pr.yml`: Fixed `chmod` and `BINDCAR_BIN` paths in drone integration test — artifact upload preserves the full relative path `target/x86_64-unknown-linux-gnu/release/bindcar`, so after downloading to `/tmp/bindcar-bin` the binary is at `/tmp/bindcar-bin/target/x86_64-unknown-linux-gnu/release/bindcar`, not `/tmp/bindcar-bin/bindcar`

### Why
`chmod: cannot access '/tmp/bindcar-bin/bindcar': No such file or directory` — `upload-artifact` preserves the workspace-relative path structure inside the artifact zip, so `download-artifact` mirrors that structure under the destination directory.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [x] CI/CD only

## [2026-03-29 18:00] - Fix CI test and drone integration test failures

**Author:** Erick Bourgeois

### Changed
- `.github/workflows/pr.yml`: Updated `K8S_OPENAPI_ENABLED_VERSION` from `"1.31"` to `"1.32"` to match the merged `k8s-openapi = "0.27"` Cargo.toml dependency that enables `v1_32` feature
- `.github/workflows/pr.yml`: Added `apt-get update` before `apt-get install` in drone integration test job to prevent 404 mirror failures
- `.github/workflows/release.yml`: Updated `K8S_OPENAPI_ENABLED_VERSION` from `"1.31"` to `"1.32"` to match

### Why
Merging `main` into `out-of-cluster` brought in `kube 3.0` and `k8s-openapi 0.27` with `v1_32` feature. The CI env var `K8S_OPENAPI_ENABLED_VERSION: "1.31"` enabled `v1_31` while Cargo.toml enabled `v1_32` — k8s-openapi panics when both are active. The drone test apt install failed because the runner's package mirror had a stale/missing entry for `bind9-utils`.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [x] CI/CD only

## [2026-03-26 00:02] - Drone integration test in PR CI

**Author:** Erick Bourgeois

### Changed
- `integration-test/drone-external-bind9.sh`: `BINDCAR_BIN` is now overridable via env var (`${BINDCAR_BIN:-${REPO_ROOT}/target/debug/bindcar}`) so CI can point to a pre-built artifact
- `Makefile`: Added `drone-integration-test-ci` target — runs the integration test script directly without `cargo build` (binary supplied via `BINDCAR_BIN`)
- `.github/workflows/pr.yml`: Added `drone-integration-test` job — runs after `build`, downloads `bindcar-linux-amd64` artifact, installs `dnsutils` for `dig`, then calls `make drone-integration-test-ci`

### Why
Wire the drone integration test into the PR gate so every PR is automatically validated end-to-end: BIND9 starts in Docker, bindcar drone manages it, zone and A record are created, and DNS resolution is confirmed with dig.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] New CI gate only

## [2026-03-27 00:00] - External BIND9 example config and drone integration test

**Author:** Erick Bourgeois

### Added
- `examples/external-bind9/setup.sh`: Bare-metal/VM setup script — generates TSIG key, copies config files to `/etc/bind/`, sets permissions, validates with `named-checkconf`
- `examples/external-bind9/integration-test.sh`: End-to-end integration test — starts BIND9 in Docker, starts `bindcar drone`, creates zone `foo.bar`, adds A record `test.foo.bar → 1.2.3.4`, verifies resolution with `dig`
- `Makefile`: Added `drone-integration-test` target

### Why
The `examples/external-bind9/` config files needed both a production setup path (setup.sh)
and an automated test that proves the full drone-mode flow works: BIND9 accepting rndc
addzone, nsupdate for record creation, and actual DNS resolution. This is the key integration
test for Phase 1 of the standalone out-of-cluster roadmap.

### Design notes
- Zone dir is mounted at the **same absolute path** (`/tmp/bindcar-drone-test/zones`) in
  both host and Docker container so `rndc addzone file <path>` resolves correctly from named's
  perspective without Phase 2's `BIND_ZONE_DIR_REMOTE`.
- BIND9 controls and `allow-update` are opened to `any` in the test config because Docker NAT
  changes the source IP from named's perspective (not 127.0.0.1).
- `openssl rand -base64 32` generates the TSIG key — no dependency on BIND9 tools.
- `DISABLE_AUTH=true` — this test validates zone/record management, not Kubernetes auth.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] New test infrastructure only

## [2026-03-26 00:01] - CLI subcommands: `run` (sidecar) and `drone` (standalone)

**Author:** Erick Bourgeois

### Changed
- `Cargo.toml`: Added `clap = { version = "4", features = ["derive"] }` dependency
- `src/cli.rs`: NEW — `Cli` struct and `Commands` enum (`Run`, `Drone`) with `resolved_command()` defaulting to `Run`
- `src/cli_test.rs`: NEW — 8 TDD tests for CLI parsing (subcommand parsing, default resolution, unknown subcommand error, help text)
- `src/lib.rs`: Added `pub mod cli` and `#[cfg(test)] mod cli_test`
- `src/main.rs`: Extracted `init_tracing()` and `start_server(&Commands)` from monolithic `main()`. `main()` now parses CLI and dispatches. `start_server` logs the mode in the startup banner ("sidecar mode" or "drone mode - standalone")

### Why
Users need a clear CLI contract for the two operating modes (sidecar vs standalone drone).
`bindcar` with no args continues to behave identically to `bindcar run` for backwards
compatibility with existing process supervisors and Kubernetes entrypoints. The `drone`
subcommand establishes the entry point for all future Phase 2–4 standalone features
(SSH zone transport, multi-instance config, systemd packaging).

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [x] Default behavior unchanged (`bindcar` still starts the sidecar server)
- [ ] Documentation only

## [2026-03-26 00:00] - Phase 1: Out-of-Cluster Kubernetes Authentication (TDD)

**Author:** Erick Bourgeois

### Changed
- `src/auth.rs`: Added `KubeAuthMode` enum, `detect_kube_auth_mode()`, `build_explicit_kube_client()`, and `build_kube_client()` under `#[cfg(feature = "k8s-token-review")]`
- `src/auth.rs`: `validate_token_with_k8s()` now calls `build_kube_client()` instead of `Client::try_default()` directly
- `src/auth_test.rs`: Added 11 new TDD tests covering `KubeAuthMode` detection (7 tests) and `build_explicit_kube_client` file error handling (3 tests), all written RED-first
- `src/main.rs`: Startup log now shows the resolved Kubernetes auth mode when `k8s-token-review` feature is compiled in

### Why
Phase 1 of the standalone out-of-cluster roadmap (`docs/roadmaps/standalone-out-of-cluster.md`). Enables bindcar to run on bare-metal/VM hosts that have no kubeconfig file but do have a token file and CA cert (e.g., obtained via projected volumes or secrets copied to the host). The explicit env-var path (`KUBE_API_SERVER` + `KUBE_TOKEN_PATH` + `KUBE_CA_CERT_PATH`) mirrors the HashiCorp Vault Kubernetes auth pattern.

### New Environment Variables (under `k8s-token-review` feature)
| Variable | Description |
|----------|-------------|
| `KUBE_API_SERVER` | Kubernetes API server URL (e.g., `https://api.prod.example.com:6443`) |
| `KUBE_TOKEN_PATH` | Path to a ServiceAccount token file |
| `KUBE_CA_CERT_PATH` | Path to the cluster CA certificate (PEM) |

All three must be set together. Partial configuration logs a warning and falls back to `try_default()`.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [ ] Config change only
- [x] New optional feature — zero impact on existing deployments; `KUBE_*` vars are not set in existing environments so `try_default()` path is unchanged
