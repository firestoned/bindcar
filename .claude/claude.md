@.claude/SKILL.md

# Project Guidelines for bindcar

> HTTP REST API + CLI for programmatic BIND9 DNS management via RNDC

**CRITICAL Coding Patterns** (full details in `rules/`):
- **TDD**: Write tests FIRST — `rules/testing.md` + `tdd-workflow` skill
- **After ANY Rust change**: run `cargo-quality` skill (NON-NEGOTIABLE)
- **Early returns / magic numbers / style**: `rules/rust-style.md`

---

## Rust Development Environment

**CRITICAL:** Before executing any `cargo` or Rust-related commands, always run:
```bash
source ~/.zshrc
```

This ensures the Rust toolchain is properly loaded in the shell environment.

---

## Service Mesh

When referencing service mesh in documentation or code, always use **Linkerd** as the example implementation.

---

## 🔍 MANDATORY: Use ripgrep

ALWAYS use `rg` for code search. NEVER use `grep`, `find`, or `lsof`.

- Rust files: `rg -trs "pattern" . -g '!target/'`

---

## 🚫 Docker Operations Restrictions

**NEVER build or push Docker images.** The user manages all image operations.

After code changes: run `cargo fmt`, `cargo clippy`, `cargo test`, then inform the user changes are ready to build and deploy.

---

## 🚨 Plans and Roadmaps → `docs/roadmaps/`

ALL planning documents MUST go in `docs/roadmaps/`. Filenames: **lowercase**, **hyphens only** (no underscores, no uppercase).

```
✅ docs/roadmaps/out-of-cluster-support.md
❌ ROADMAP.md  ❌ docs/roadmaps/OUT_OF_CLUSTER.md  ❌ docs/roadmaps/Phase_3.md
```

---

## 🔧 GitHub Workflows & CI/CD

See `rules/github-workflows.md` for full standards. Key rules:

- **NEVER** replace `firestoned/github-actions` composite actions with direct action calls — update the `firestoned/github-actions` repo instead
- All workflows MUST delegate logic to Makefile targets (no inline bash scripts)
- New workflows MUST support `workflow_call` for reusability

---

## 📝 Documentation Requirements

See `rules/documentation.md` for full workflow.

- Ask "Does documentation need to be updated?" before marking ANY task complete
- Update `.claude/CHANGELOG.md` with `**Author:**` on EVERY code change (MANDATORY — no exceptions)
- Build docs with `make docs` — use `build-docs` skill

---

## 🦀 Rust Workflow

Full style guide: `rules/rust-style.md`. Full testing standards: `rules/testing.md`.

**After ANY `.rs` change:** run `cargo-quality` skill (`cargo fmt` + `cargo clippy` + `cargo test`). Task is NOT complete until all three pass.

### TDD (mandatory)

Write failing tests FIRST, then implement minimum code to pass. See `tdd-workflow` skill.

Test file pattern: `src/foo.rs` → `#[cfg(test)] mod foo_test;` at bottom → `src/foo_test.rs`

> **Note:** This project uses `_test.rs` (singular), not `_tests.rs` (plural).

### Dependency Management

Before adding deps: verify actively maintained (commits in last 6 months), prefer well-known crates, document reason in CHANGELOG.

---

## 🧪 Testing

See `rules/testing.md` for full standards.

- Every public function MUST have unit tests
- Tests in separate `_test.rs` files (never embedded in source)
- Integration tests in `integration-test/` directory
- Run: `cargo-quality` skill. Specific test: `cargo test <name>`. Verbose: `cargo test -- --nocapture`

---

## 📁 File Organization

```
src/
├── main.rs
├── lib.rs
├── auth.rs / auth_test.rs
├── cli.rs / cli_test.rs
├── metrics.rs / metrics_test.rs
├── middleware.rs / middleware_test.rs
├── nsupdate.rs / nsupdate_test.rs
├── rate_limit.rs / rate_limit_test.rs
├── records.rs / records_test.rs
├── rndc.rs / rndc_test.rs
├── rndc_conf_parser.rs / rndc_conf_parser_tests.rs
├── rndc_conf_types.rs / rndc_conf_types_tests.rs
├── rndc_parser.rs / rndc_parser_tests.rs
├── rndc_types.rs / rndc_types_tests.rs
├── types.rs / types_test.rs
└── zones.rs / zones_test.rs

docs/
├── roadmaps/   ← ALL planning docs here (lowercase-hyphen filenames)
└── src/        ← mkdocs source

integration-test/   ← Integration tests
examples/           ← Usage examples
```

---

## 🚫 Things to Avoid

- `unwrap()` in production — use `?` or explicit error handling
- Hardcoded ports or paths — make them configurable
- `sleep()` for synchronization
- Ignoring RNDC command errors
- Magic numbers — use named constants (see `rules/rust-style.md`)

---

## 💡 Helpful Commands

```bash
source ~/.zshrc && cargo run                     # Run locally
cargo test -- --nocapture                        # Verbose test output
make docs                                        # Build documentation
```

Skills: `cargo-quality`, `tdd-workflow`, `update-changelog`, `update-docs`, `build-docs`, `get-multiarch-digest`, `pre-commit-checklist`.

---

## 📋 PR/Commit Checklist

**Run `pre-commit-checklist` skill before EVERY commit. A task is NOT complete until it passes.**

Documentation is NOT optional — it is a critical requirement equal in importance to the code.
