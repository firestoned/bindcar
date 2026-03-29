# Claude Skills Reference

Reusable procedural skills extracted from CLAUDE.md. Each skill has a canonical name (kebab-case), trigger conditions, ordered steps, and a verification check. Invoke a skill by name: *"run the cargo-quality skill"* or *"do a pre-commit-checklist"*.

---

## `cargo-quality`

**When to use:**
- After adding or modifying ANY `.rs` file
- Before committing any Rust code changes
- At the end of EVERY task involving Rust code (NON-NEGOTIABLE)

**Steps:**
```bash
# 0. Ensure cargo is in PATH
source ~/.zshrc

# 1. Format
cargo fmt

# 2. Lint with strict warnings (fix ALL warnings)
cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic -A clippy::module_name_repetitions

# 3. Test (ALL tests must pass)
cargo test

# 4. Security audit (optional, if installed)
cargo audit 2>/dev/null || true
```

**Verification:** All three commands exit with code 0. No warnings, no test failures.

---

## `tdd-workflow`

**When to use:**
- Adding any new feature or function
- Fixing a bug
- Refactoring existing code

**Steps:**

**RED — Write failing tests first (before any implementation):**
```bash
# Edit src/<module>_test.rs — add test(s) that define expected behavior
cargo test <test_name>   # Must FAIL at this point
```

**GREEN — Implement minimum code to pass tests:**
```bash
# Edit src/<module>.rs — write simplest code that makes tests pass
cargo test <test_name>   # Must PASS now
```

**REFACTOR — Improve while keeping tests green:**
```bash
# Extract constants, add docs, improve error handling
cargo test               # Must still PASS
cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic -A clippy::module_name_repetitions
```

**Test file pattern:**
- Source: `src/foo.rs` → declare `#[cfg(test)] mod foo_test;` at the bottom
- Tests: `src/foo_test.rs` → contains test functions

> **Note:** This project uses `_test.rs` (singular), not `_tests.rs` (plural).

**Verification:** All tests pass, clippy is clean, test covers success path + error paths + edge cases.

---

## `update-changelog`

**When to use:**
- After ANY code modification (mandatory)

**Steps:**

Open `.claude/CHANGELOG.md` and prepend an entry in this exact format:

```markdown
## [YYYY-MM-DD HH:MM] - Brief Title

**Author:** <Name of requester or approver>

### Changed
- `path/to/file.rs`: Description of the change

### Why
Brief explanation of the technical reason.

### Impact
- [ ] Breaking change
- [ ] API change
- [ ] Config change only
- [ ] Documentation only
```

**Verification:** Entry has `**Author:**` line (MANDATORY — no exceptions), timestamp, and at least one `### Changed` item.

---

## `update-docs`

**When to use:**
- After any code change in `src/`
- After API changes, configuration changes, or new features

**Steps:**
1. Identify what changed (feature, API, behavior, error condition).
2. Update `.claude/CHANGELOG.md` (see `update-changelog` skill).
3. Update affected pages in `docs/src/`:
   - User guides, quickstart guides, configuration references, troubleshooting guides
4. Update `examples/` to reflect behavior changes.
5. Update architecture diagrams if structure changed.
6. If README getting-started or features changed: update `README.md`.
7. Run `build-docs` skill to confirm no broken references.

**Verification checklist:**
- [ ] `.claude/CHANGELOG.md` updated with author
- [ ] All affected `docs/src/` pages updated
- [ ] Examples updated if behavior changed
- [ ] Architecture diagrams match current implementation
- [ ] `make docs` succeeds

---

## `build-docs`

**When to use:**
- After any documentation change
- Before any documentation release
- To verify docs are not broken

**Steps:**
```bash
# Build all documentation components
make docs
```

**Verification:** `make docs` exits 0 with no errors.

---

## `get-multiarch-digest`

**When to use:**
- Before pinning a Docker base image digest in any Dockerfile
- When updating base image versions

**Steps:**
```bash
# Get the multi-arch manifest list digest (NOT platform-specific)
docker buildx imagetools inspect <image>:<tag> --raw | sha256sum | awk '{print "sha256:"$1}'

# Examples:
docker buildx imagetools inspect debian:12-slim --raw | sha256sum | awk '{print "sha256:"$1}'
docker buildx imagetools inspect rust:1.94.0 --raw | sha256sum | awk '{print "sha256:"$1}'
```

Use the digest in Dockerfiles as:
```dockerfile
# NOTE: This digest points to the multi-arch manifest list (supports both AMD64 and ARM64)
FROM debian:12-slim@sha256:<digest> AS builder
```

**Verification:**
```bash
docker buildx imagetools inspect <image>@<digest>
# Output must show BOTH: Platform: linux/amd64 AND Platform: linux/arm64
```

---

## `pre-commit-checklist`

**When to use:**
- Before committing any change (mandatory gate)

**Checklist:**

### If ANY `.rs` file was modified:
- [ ] Tests updated/added/deleted to match changes (TDD — see `tdd-workflow`)
- [ ] All new public functions have tests
- [ ] All deleted functions have tests removed
- [ ] `cargo fmt` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes (fix ALL warnings)
- [ ] `cargo test` passes (ALL tests green)
- [ ] Rustdoc comments on all public items, accurate to actual behavior
- [ ] `docs/src/` updated for user-facing changes

### If API handlers were modified:
- [ ] API reference docs updated in `docs/src/reference/`
- [ ] Examples updated if request/response format changed
- [ ] Error responses documented

### Always:
- [ ] `.claude/CHANGELOG.md` updated with **Author:** line (MANDATORY)
- [ ] `make docs` succeeds
- [ ] No secrets, tokens, credentials, or IP addresses committed
- [ ] No `.unwrap()` in production code

**Verification:** Every checked box above passes. A task is NOT complete until the full checklist is green.
