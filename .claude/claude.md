@.claude/SKILL.md

# Project Guidelines for bindcar

> HTTP REST API + CLI for programmatic BIND9 DNS management via RNDC

**CRITICAL Coding Patterns** (full details in `rules/`):
- **TDD**: Write tests FIRST — `rules/testing.md` + `tdd-workflow` skill
- **After ANY Rust change**: run `cargo-quality` skill (NON-NEGOTIABLE)
- **Early returns / magic numbers / style**: `rules/rust-style.md`

---

# Project Instructions for Claude Code

> Platform Engineering - Kubernetes Operators & Infrastructure
> Environment: k0rdent / Capital Markets / Multi-cluster
>
> **Service Mesh Standard**: Always use Linkerd as the example service mesh in documentation, examples, and code comments. Do not use generic "service mesh" references or other mesh implementations (Istio, Consul Connect, etc.) unless specifically required.
>
> **CRITICAL Coding Patterns**:
> - **Test-Driven Development (TDD)**: ALWAYS write tests FIRST before implementing functionality. Write failing tests that define the expected behavior, then implement code to make tests pass. This ensures all code is testable and has comprehensive test coverage from the start.
> - **Event-Driven Programming**: In Kubernetes controller development, ALWAYS use or recommend event-driven programming (e.g., "watch" on kube API) as opposed to polling. Controllers must react to cluster state changes efficiently.
> - **Early Returns**: Use as few `else` statements as possible. Return from functions as soon as you can to minimize nesting and improve code clarity (see Early Return / Guard Clause Pattern section).
> - **ALWAYS Run cargo fmt**: At the end of EVERY task or phase involving Rust code, you MUST run `cargo fmt` (and `cargo clippy`, `cargo test`). This is NON-NEGOTIABLE and MANDATORY. See "CRITICAL: Always Run cargo fmt and clippy After Code Changes" section for full requirements.

---

## ⚙️ Claude Code Configuration

### 🚨 CRITICAL: Always Verify CRD Schema Sync

**MANDATORY REQUIREMENT:** Before investigating any Kubernetes-related issue, ALWAYS verify that deployed CRDs match the Rust code definitions.

**Why This Matters:**
- CRD YAML files in `deploy/crds/` are **AUTO-GENERATED** from Rust types in `src/crd.rs`
- If CRDs are not regenerated after code changes, schema mismatches cause silent failures
- Kubernetes API server may accept patches (HTTP 200) but ignore fields not in the CRD schema
- This leads to confusing bugs where code works but data doesn't persist

> **How:** Run the `verify-crd-sync` skill.

**When to Check:**
- ✅ Before investigating reconciliation loops or infinite loops
- ✅ Before debugging "field not appearing in kubectl output" issues
- ✅ After ANY modification to structs in `src/crd.rs`
- ✅ When status patches succeed but data doesn't persist
- ✅ When user reports unexpected controller behavior

**Example Failure Mode:**
```rust
// Rust code has new field
pub struct Bind9InstanceStatus {
    pub zones: Vec<ZoneReference>,  // Added in recent commit
    // ...
}

// But deployed CRD is missing the field
// Result: Patches succeed (200 OK) but zones never appear in kubectl output
// Controller sees empty zones every reconciliation → infinite loop
```

**REMEMBER:** Always verify CRD schema sync BEFORE making assumptions about code logic issues.

---

### 🚨 CRITICAL: Always Review Official Documentation When Unsure

**MANDATORY REQUIREMENT:** When you are unsure of a decision, DO NOT take the easiest or fastest path. ALWAYS review the official documentation for the product, tool, or framework you are working with.

**Why This Matters:**
- Taking shortcuts leads to incorrect implementations that must be fixed later
- Quick assumptions often miss critical configuration options or best practices
- Official documentation is the authoritative source of truth
- Proper research prevents technical debt and rework

**What to Review:**
- **Configuration Documentation**: For tools like MkDocs, mdbook, GitHub Actions, etc.
- **CRD API Reference**: When working with Kubernetes resources
- **Framework Documentation**: When using libraries or frameworks
- **Best Practices Guides**: Official recommendations for tool usage
- **Release Notes**: Changes in behavior between versions

**Examples:**
- ✅ Unsure about MkDocs configuration → Read the [MkDocs Configuration Reference](https://www.mkdocs.org/user-guide/configuration/)
- ✅ Unsure about Kubernetes CRD fields → Read `/deploy/crds/*.crd.yaml` or `src/crd.rs`
- ✅ Unsure about GitHub Actions syntax → Read the [GitHub Actions Documentation](https://docs.github.com/en/actions)
- ✅ Unsure about Rust kube-rs API → Read the [kube-rs API documentation](https://docs.rs/kube/)

**When to Stop and Research:**
- You're about to make an assumption about how something works
- Multiple approaches seem viable but you're not sure which is correct
- You're implementing a feature you haven't used before
- Documentation or configuration syntax is unfamiliar
- You're debugging an issue and the root cause is unclear

**REMEMBER:** Spending 5 minutes reading documentation prevents hours of debugging incorrect implementations. When in doubt, research first, implement second.

---

### 🚨 CRITICAL: Docker Base Image Digests Must Be Multi-Arch

**MANDATORY REQUIREMENT:** When updating Docker base images in Dockerfiles, ALWAYS use multi-arch manifest list digests, NOT platform-specific digests.

**Why This Matters:**
- Platform-specific digests (e.g., AMD64-only) force Docker BuildKit to use QEMU emulation for other architectures
- QEMU emulation often fails with cryptic errors like: `.buildkit_qemu_emulator: Invalid ELF image for this architecture`
- Multi-arch manifest list digests allow Docker BuildKit to select the correct platform-native image automatically
- This ensures builds work correctly for both `linux/amd64` and `linux/arm64` without emulation

> **How:** Run the `get-multiarch-digest` skill.

**Dockerfile Pattern:**
```dockerfile
# ✅ CORRECT - Multi-arch manifest list digest
# NOTE: This digest points to the multi-arch manifest list (supports both AMD64 and ARM64)
FROM debian:12-slim@sha256:74d56e3931e0d5a1dd51f8c8a2466d21de84a271cd3b5a733b803aa91abf4421 AS builder

# ❌ WRONG - Platform-specific digest (AMD64 only)
FROM debian:12-slim@sha256:0836c58489cd1baee1f617ab06a4fb1b908604d4416022173e4da43ff12399de AS builder
```

**Required Dockerfiles:**
When updating base images, you MUST update ALL Dockerfiles that use the same base image:
- ✅ `docker/Dockerfile` (Distroless variant)
- ✅ `docker/Dockerfile.chainguard` (Chainguard variant)
- ✅ `docker/Dockerfile.chef` (cargo-chef variant)
- ✅ `docker/Dockerfile.fast` (fast build variant)
- ✅ `docker/Dockerfile.local` (local development - usually no digest)

**Common Mistake to Avoid:**
```bash
# ❌ WRONG - This gives you the platform-specific digest for your current architecture
docker pull debian:12-slim
docker inspect debian:12-slim | grep -A 5 RepoDigests

# ✅ CORRECT - This gives you the multi-arch manifest list digest
docker buildx imagetools inspect debian:12-slim --raw | sha256sum
```

**REMEMBER:** Multi-arch manifest list digests prevent ARM64 build failures and QEMU emulation errors while maintaining supply chain security through digest pinning.

---

### 🔍 MANDATORY SEARCH TOOL: ripgrep (rg)
**OBLIGATORY RULE:** ALWAYS use `ripgrep` (command: `rg`) as your PRIMARY and FIRST tool for ANY code search, pattern matching, or grepping task. This is NON-NEGOTIABLE.

### 🛠️ Tool Configuration & Usage:
*   **Default Command:** `rg`
*   **Rust-Specific Flags:** For Rust projects, use `rg -trs <PATTERN>` to search only Rust files (`-trs`) recursively.
*   **Exclusions:** Use `-g '!target/'` to ignore the build directory.
*   **Example Search:** `rg -trs "my_function_name" . -g '!target/'`
*   **No grep/lsof/find:** If Claude tries to use `grep`, `lsof`, or other tools, remind it to use `rg` instead.

### 💡 Workflow for Rust Code Tasks:
1.  **Analyze:** Understand the request and identify relevant files/functions.
2.  **Search (rg):** Use `rg -trs "pattern" .` to find code snippets, definitions, or usages.
3.  **Context:** Use search results to build understanding of the codebase structure.
4.  **Generate/Modify:** Provide Rust code based on analysis, ensuring it adheres to best practices.

## 🚫 CRITICAL: Docker and Kubernetes Operations Restrictions

**NEVER build or push Docker images yourself. The user handles all Docker image operations.**

### Allowed kubectl Operations (Read-Only + Annotations):
- ✅ `kubectl get` - Read resources
- ✅ `kubectl describe` - View resource details
- ✅ `kubectl logs` - Read pod logs
- ✅ `kubectl annotate` - Add/modify annotations
- ✅ Any other read-only operations

### FORBIDDEN Operations:
- ❌ `docker build` - NEVER build Docker images
- ❌ `docker push` - NEVER push images to registries
- ❌ `docker tag` - NEVER tag images
- ❌ `kind load` - NEVER load images into kind
- ❌ `kubectl rollout restart` - NEVER restart deployments/pods
- ❌ `kubectl delete pods` - NEVER delete pods to trigger restarts
- ❌ `kubectl apply` - NEVER apply manifests (unless explicitly requested)
- ❌ `kubectl patch` - NEVER patch resources (unless explicitly requested)
- ❌ Any Docker or deployment operations

**Why:**
- The user manages the deployment pipeline
- Building/pushing images can interfere with the user's workflow
- The user knows when and how to deploy changes
- Claude should focus on code changes, not deployment

**What to do instead:**
1. Make code changes and run `cargo fmt`, `cargo clippy`, `cargo test`
2. Inform the user that changes are ready
3. Let the user handle building, pushing, and deploying

**Example:**
```
❌ WRONG:
"Let me build the Docker image and restart the controller..."
docker build -t ...
kubectl rollout restart ...

✅ CORRECT:
"I've fixed the issue in src/reconcilers/dnszone.rs and updated .claude/CHANGELOG.md.
All tests pass. The changes are ready for you to build and deploy."
```

---

## 🚨 Critical TODOs

### CRITICAL: Plans and Roadmaps Location
**Status:** ✅ MANDATORY REQUIREMENT
**Impact:** Documentation organization and discoverability

**ALWAYS add plans or roadmaps to `docs/roadmaps/`, NO WHERE ELSE.**

**Why:**
- **Centralized Location**: All planning documents in one discoverable location
- **Documentation Structure**: Maintains consistent docs/ directory organization
- **Version Control**: Plans tracked alongside code changes
- **Easy Reference**: Developers know exactly where to find planning documents

**Naming Convention:**
- **ALWAYS** use **lowercase** filenames (MANDATORY)
- **ALWAYS** use **hyphens** (`-`) to separate words, NEVER underscores (`_`) (MANDATORY)
- Use descriptive names indicating the purpose
- NO uppercase letters anywhere in the filename
- NO underscores anywhere in the filename

**Examples:**
```
✅ CORRECT:
docs/roadmaps/integration-test-plan.md
docs/roadmaps/phase4-implementation.md
docs/roadmaps/feature-roadmap-2025.md
docs/roadmaps/zones-from-label-selector-support.md
docs/roadmaps/refactoring-complete.md
docs/roadmaps/phase-1-2-implementation-plan.md

❌ WRONG:
INTEGRATION_TEST_PLAN.md (root directory, uppercase, underscores)
ROADMAP.md (root directory, uppercase)
planning/test-plan.md (wrong directory)
docs/roadmaps/ZONES_FROM_LABEL_SELECTOR.md (uppercase, underscores)
docs/roadmaps/Phase_3_Analysis.md (uppercase and underscores)
docs/roadmaps/Refactoring_Complete.md (uppercase and underscores)
docs/roadmaps/REFACTORING-COMPLETE.md (uppercase)
```

**CRITICAL RULE**: Before creating ANY file in `docs/roadmaps/`, verify the filename is:
1. All lowercase letters
2. Words separated by hyphens (`-`)
3. No underscores (`_`)
4. No uppercase letters

**What Goes in docs/roadmaps/:**
- Integration test plans
- Feature implementation roadmaps
- Phase/milestone planning documents
- Release planning documents
- Architecture decision records (ADRs) planning
- Migration plans

**REMEMBER:** Before creating any planning document, ALWAYS put it in `docs/roadmaps/`. Never in the root directory or elsewhere.

---

### Code Quality: Use Global Constants for Repeated Strings
**Status:** 🔄 Ongoing
**Impact:** Code maintainability and consistency

When a string literal appears in multiple places across the codebase, it MUST be defined as a global constant and referenced consistently.

**Why:**
- **Single Source of Truth**: Changes only need to be made in one place
- **Consistency**: Prevents typos and inconsistencies across the codebase
- **Maintainability**: Easier to refactor and update values
- **Type Safety**: Compiler catches usage errors

**When to Create a Global Constant:**
- String appears 2+ times in the same file
- String appears in multiple files
- String represents a configuration value (paths, filenames, keys, etc.)
- String is part of an API contract or protocol

**Examples:**
```rust
// ✅ GOOD - Use constants
const BIND_NAMED_CONF_PATH: &str = "/etc/bind/named.conf";
const NAMED_CONF_FILENAME: &str = "named.conf";

fn build_configmap() {
    data.insert(NAMED_CONF_FILENAME.into(), named_conf);
}

// ❌ BAD - Hardcoded strings
fn build_configmap() {
    data.insert("named.conf".into(), named_conf);
}
```

**Where to Define Constants:**
- Module-level constants: At the top of the file for file-specific use
- Crate-level constants: In a dedicated module (e.g., `src/constants.rs`) for cross-module use
- Group related constants together with documentation

**Verification:**
Before committing, search for repeated string literals:
```bash
# Find potential duplicate strings in Rust files
grep -rn '"[^"]\{5,\}"' src/ | sort | uniq -d
```

### CRITICAL: Always Run cargo fmt and clippy After Code Changes
**Status:** ✅ Required Standard
**Impact:** Code quality, consistency, and CI/CD pipeline success

**MANDATORY REQUIREMENT:** Whenever you add or modify code in tests or source files, run the `cargo-quality` skill before considering the task complete.

> **How:** Run the `cargo-quality` skill.

**Why This Matters:**
- **CI/CD Will Fail**: GitHub Actions will reject commits with formatting violations or clippy warnings
- **Catches Bugs Early**: Clippy identifies common mistakes and non-idiomatic code before they reach production
- **Maintains Consistency**: Ensures uniform code style across the entire project
- **Saves Time**: Prevents wasted CI minutes and failed builds due to formatting issues

**When to Run:**
- ✅ After adding new functions or types
- ✅ After modifying existing code
- ✅ After writing or updating tests
- ✅ Before committing any `.rs` file changes
- ✅ At the end of EVERY task involving Rust code

**What to Do If They Fail:**
- **cargo fmt failures**: Run `cargo fmt` and commit the formatting changes
- **cargo clippy warnings**: Fix ALL warnings - do not ignore them
- **cargo test failures**: Fix the failing tests - the task is NOT complete until all tests pass

**REMEMBER:** A task involving Rust code is **NOT complete** until cargo fmt, cargo clippy, and cargo test all pass successfully.

### High Priority: CRD Code Generation
**Status:** ✅ Implemented
**Impact:** Automated - CRD YAMLs are generated from Rust types

The Rust types in `src/crd.rs` are the **source of truth**. CRD YAML files in `/deploy/crds/` are **auto-generated** from these types.

> **How:** Run the `regen-crds` skill, then the `regen-api-docs` skill (LAST). Use `kubectl replace --force` not `kubectl apply` — the `Bind9Instance` CRD is ~393KB and exceeds the 256KB annotation size limit.

⚠️ **IMPORTANT**: Always run `crddoc` **LAST** after all CRD changes, example updates, and validations are complete. This ensures the API documentation reflects the final, validated state of the CRDs.

See **CRD Development - Rust as Source of Truth** section below for details.

⚠️ **IMPORTANT**: Examples and documentation MUST stay in sync with CRD schemas. After ANY CRD change, you MUST update:
- `/examples/*.yaml` - Ensure all examples can be applied successfully
- `/docs/src/` - Update any documentation that references the CRD fields
- Quickstart guide - Verify all YAML snippets are valid

### CRITICAL: Documentation Examples Must Reference CRDs

**Status:** ✅ Required Standard
**Impact:** Documentation accuracy and user trust

**CRITICAL REQUIREMENTS:**

1. **Before Creating Examples in Documentation:**
   - **ALWAYS** read the relevant CRD YAML files in `/deploy/crds/` OR the Rust types in `src/crd.rs` first
   - **VERIFY** the exact field names and structure from the source of truth
   - **NEVER** guess or assume field names based on convention
   - **NEVER** copy examples from memory or other documentation without verification

2. **When CRDs Change:**
   - **SEARCH** all documentation in `/docs/src/` for examples using the changed CRD
   - **UPDATE** every example to match the new schema
   - **VALIDATE** that all examples can be applied: `kubectl apply --dry-run=client -f examples/`
   - **BUILD** documentation to ensure no broken references: `make docs`

**Why This Matters:**
- Incorrect examples break user trust and cause support burden
- Documentation is often the first touchpoint for users
- Outdated examples lead to failed deployments and frustration
- Schema mismatches are hard to debug for users

**Example of Critical Failure:**
```yaml
# ❌ WRONG - Using spec.config instead of spec.global
apiVersion: bindy.firestoned.io/v1beta1
kind: Bind9GlobalCluster
spec:
  config:  # This field doesn't exist!
    recursion: true

# ✅ CORRECT - Verified against CRD
apiVersion: bindy.firestoned.io/v1beta1
kind: Bind9GlobalCluster
spec:
  global:  # Correct field name from CRD schema
    recursion: true
```

**Workflow for Documentation Examples:**
1. Identify which CRD the example uses
2. Read `/deploy/crds/<crd-name>.crd.yaml` or `src/crd.rs`
3. Verify the exact field names and types
4. Write the example using the verified schema
5. Validate the example: `kubectl apply --dry-run=client`
6. If CRD changes, search docs for ALL examples: `grep -r "kind: <CRDName>" docs/src/`

**Verification Checklist:**
- [ ] Read CRD schema before writing examples
- [ ] All field names match CRD exactly (case-sensitive)
- [ ] All required fields are present
- [ ] No deprecated or removed fields used
- [ ] Examples validate successfully with kubectl

**REMEMBER:** The CRD schema is the contract. Documentation must honor that contract exactly.

---

## 🔧 GitHub Workflows & CI/CD

### CRITICAL: All Workflows Must Be Makefile-Driven

**Status:** ✅ Required Standard
**Impact:** Consistency, maintainability, and local reproducibility

All GitHub Actions workflows MUST delegate complex logic to Makefile targets. Workflows should only:
1. Install required tools (Kind, kubectl, Rust, etc.)
2. Set up environment variables
3. Call Makefile targets

**Why:**
- **Local Reproducibility**: Developers can run the exact same commands locally
- **Consistency**: Same logic runs in CI and locally
- **Maintainability**: Business logic lives in one place (Makefile), not scattered across workflows
- **Testability**: Makefile targets can be tested independently
- **Simplicity**: Workflows become declarative configuration, not complex scripts

**Pattern:**

```yaml
# ✅ GOOD - Workflow delegates to Makefile
jobs:
  integration-test:
    steps:
      - name: Install Kind
        uses: helm/kind-action@v1

      - name: Install kubectl
        uses: azure/setup-kubectl@v4

      - name: Run integration tests
        env:
          IMAGE_TAG: ${{ steps.tag.outputs.tag }}
          REGISTRY: ghcr.io
        run: make kind-integration-test-ci

# ❌ BAD - Complex logic in workflow
jobs:
  integration-test:
    steps:
      - name: Create cluster
        run: |
          kind create cluster --config deploy/kind-config.yaml
          kubectl create namespace dns-system
          kubectl apply -f deploy/crds/
          # ... 50+ lines of bash ...
```

**Requirements:**
- Workflows MUST NOT contain multi-line bash scripts (except simple tool setup)
- **All workflow run commands MUST call Makefile targets** - Never call tools directly (e.g., use `make cargo-deny` not `cargo deny check`)
- All test orchestration MUST be in Makefile targets
- All deployment logic MUST be in Makefile targets
- Makefile targets MUST work identically locally and in CI
- Document Makefile targets with `## comments` for `make help`

**Examples:**
```yaml
# ✅ CORRECT - Use Makefile targets
- name: Run security scans
  run: make cargo-deny

- name: Scan for secrets
  run: make gitleaks

# ❌ WRONG - Direct tool invocation
- name: Run security scans
  run: cargo deny check

- name: Scan for secrets
  run: gitleaks detect --source .
```

**Available Integration Test Targets:**
- `make kind-integration-test` - Run full integration tests with local build
- `make kind-integration-test-ci` - Run integration tests in CI mode (requires IMAGE_TAG env var)

### CRITICAL: Workflows Must Be Reusable and Composable

**Status:** ✅ Required Standard
**Impact:** Maintainability, DRY principles, and workflow consistency

When adding new GitHub Actions workflows, they MUST be designed for reusability and integration with existing workflows.

**Why:**
- **DRY Principle**: Avoid duplicating workflow logic across multiple files
- **Consistency**: Same steps produce same results across different contexts
- **Maintainability**: Update shared logic once, not in every workflow
- **Composability**: Workflows can call other workflows or be called by others
- **Flexibility**: Standalone execution and integration into larger workflows

**Requirements:**

1. **Use Reusable Workflows** (`.github/workflows/*.yml` with `workflow_call`):
   - Define workflows that can be called by other workflows
   - Accept inputs for configuration
   - Define outputs for downstream steps
   - Make them standalone executable (support both `workflow_call` and manual triggers)

2. **Use Composite Actions** (`.github/actions/*/action.yml`):
   - For complex multi-step operations that are used across multiple workflows
   - For shared setup/teardown logic
   - For operations that need to be consistent across workflows

3. **Integration Strategy**:
   - New workflows MUST be callable from existing workflows
   - Existing workflows SHOULD be able to include new workflow steps
   - Avoid creating isolated workflows that duplicate existing logic

**Pattern for Reusable Workflows:**

```yaml
# .github/workflows/integration-test.yml - Reusable workflow
name: Integration Tests

on:
  workflow_call:
    inputs:
      image_tag:
        required: true
        type: string
      registry:
        required: false
        type: string
        default: ghcr.io
    outputs:
      test_result:
        description: "Test execution result"
        value: ${{ jobs.test.outputs.result }}
  workflow_dispatch:
    inputs:
      image_tag:
        required: true
        type: string

jobs:
  test:
    runs-on: ubuntu-latest
    outputs:
      result: ${{ steps.test.outputs.result }}
    steps:
      - uses: actions/checkout@v4
      - name: Run integration tests
        id: test
        env:
          IMAGE_TAG: ${{ inputs.image_tag }}
          REGISTRY: ${{ inputs.registry }}
        run: make kind-integration-test-ci
```

**Pattern for Composite Actions:**

```yaml
# .github/actions/setup-rust/action.yml - Composite action
name: Setup Rust Environment
description: Install Rust toolchain and cache dependencies

inputs:
  toolchain:
    description: 'Rust toolchain version'
    required: false
    default: 'stable'

runs:
  using: composite
  steps:
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: ${{ inputs.toolchain }}

    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo build
      uses: actions/cache@v3
      with:
        path: target
        key: ${{ runner.os }}-cargo-build-${{ hashFiles('**/Cargo.lock') }}
```

**Pattern for Calling Reusable Workflows:**

```yaml
# .github/workflows/ci.yml - Main CI workflow that composes others
name: CI

on: [push, pull_request]

jobs:
  build:
    uses: ./.github/workflows/build.yml
    with:
      rust_version: stable

  integration-test:
    needs: build
    uses: ./.github/workflows/integration-test.yml
    with:
      image_tag: ${{ needs.build.outputs.image_tag }}
      registry: ghcr.io

  security-scan:
    needs: build
    uses: ./.github/workflows/security.yml
    with:
      image_tag: ${{ needs.build.outputs.image_tag }}
```

**Checklist for New Workflows:**

Before adding a new workflow, ask:
- [ ] Can this be added as a job to an existing workflow?
- [ ] Can this be made into a reusable workflow that others can call?
- [ ] Does this duplicate logic from an existing workflow?
- [ ] Can this be extracted into a composite action for reuse?
- [ ] Will existing workflows benefit from calling this workflow?
- [ ] Can this workflow be triggered both standalone and as a called workflow?

**Examples of Good Reusability:**

- ✅ Build workflow that can be called by CI, release, and PR workflows
- ✅ Integration test workflow that works locally and in CI
- ✅ Security scan that runs standalone, in CI, and on schedule
- ✅ Composite action for Kind cluster setup used across multiple workflows

**Examples of Poor Design:**

- ❌ Duplicating the same build steps in CI and release workflows
- ❌ Creating a workflow that can only run in one specific context
- ❌ Hardcoding values that should be inputs
- ❌ Creating standalone workflows that can't be composed

**REMEMBER:** Every new workflow should enhance the ecosystem, not create a silo. Design for reuse from day one.

---

## 🔒 Compliance & Security Context

This codebase operates in a **regulated banking environment**. All changes must be:
- Auditable with clear documentation
- Traceable to a business or technical requirement
- Compliant with zero-trust security principles

**Never commit**:
- Secrets, tokens, or credentials (even examples)
- Internal hostnames or IP addresses
- Customer or transaction data in any form

---

## 📝 Documentation Requirements

### 🚨 CRITICAL: Always Verify if Documentation Needs Updates

**MANDATORY: Before considering ANY task complete, ALWAYS ask yourself: "Does documentation need to be updated?"**

This verification applies to:
- ✅ Code changes (new features, bug fixes, refactoring)
- ✅ CRD modifications (fields added/removed/changed)
- ✅ API changes (function signatures, behavior)
- ✅ Configuration changes (new env vars, settings)
- ✅ Architecture changes (new components, flows)

**If YES to any above:** Documentation MUST be updated according to the checklist below.

---

### Mandatory: Documentation Updates for Code Changes

**CRITICAL: After ANY code change in the `src/` directory, you MUST update all relevant documentation.**

This is a **mandatory step** that must be completed before considering any task complete. Documentation must always reflect the current state of the code.

#### Documentation Update Workflow

When adding, removing, or changing any feature in the Rust source code:

1. **Analyze the Change**:
   - What functionality was added/removed/changed?
   - What are the user-facing impacts?
   - What are the architectural implications?
   - Are there new APIs, configuration options, or behaviors?

2. **Update Documentation** (in this order):
   - **`.claude/CHANGELOG.md`** - Document the change (see format below)
   - **`docs/src/`** - Update all affected documentation pages:
     - User guides that reference the changed functionality
     - Quickstart guides with examples of the changed code
     - Configuration references for new/changed options
     - Troubleshooting guides if behavior changed
   - **`examples/`** - Update YAML examples to reflect changes
   - **Architecture diagrams** - Update if structure/flow changed
   - **API documentation** - Regenerate if CRDs changed (`cargo run --bin crddoc`)
   - **README.md** - Update if getting started steps or features changed

3. **Verify Documentation Accuracy**:
   - Read through updated docs as if you're a new user
   - Ensure all code examples compile and run
   - Verify all YAML examples validate: `kubectl apply --dry-run=client -f examples/`
   - Check that diagrams match current architecture
   - Confirm API docs reflect current CRD schemas

4. **Add Missing Documentation**:
   - If architecture changed, add/update architecture diagrams
   - If new public APIs were added, document them
   - If new configuration options exist, document them with examples
   - If new error conditions exist, document troubleshooting steps
   - If new dependencies were added, document version requirements

#### What Documentation to Update

**For Controller/Reconciler Changes** (`src/reconcilers/`):
- Update reconciliation flow diagrams
- Document new behaviors in user guides
- Update troubleshooting guides for new error conditions
- Add examples showing the new functionality

**For CRD Changes** (`src/crd.rs`):
- Run `cargo run --bin crdgen` to regenerate CRD YAMLs
- Run `cargo run --bin crddoc > docs/src/reference/api.md` to regenerate API docs
- Update ALL examples in `/examples/` that use the changed CRD
- Update quickstart guides with new field examples
- Update configuration reference documentation

**For Core Logic Changes** (`src/bind9.rs`, `src/bind9_resources.rs`, etc.):
- Update architecture documentation explaining the change
- Update API documentation if public interfaces changed
- Add code examples for new public functions
- Update troubleshooting guides for new behaviors

**For New Features**:
- Add feature documentation to `/docs/src/features/`
- Update feature list in README.md
- Add usage examples
- Create architecture diagrams showing how the feature works
- Document configuration options
- Add troubleshooting section

**For Bug Fixes**:
- Update troubleshooting guides with the fix
- Document workarounds (if applicable) in known issues
- Update behavior documentation if expectations changed

#### Documentation Quality Standards

- **Completeness**: All user-visible changes must be documented
- **Accuracy**: Documentation must match the actual code behavior
- **Examples**: Include working examples for all features
- **Clarity**: Write for users who haven't seen the code
- **Diagrams**: Use Mermaid diagrams for complex flows
- **Versioning**: Date all changes in .claude/CHANGELOG.md

#### Building Documentation

**CRITICAL: Always use the Makefile target — never run `mdbook build` directly.**

> **How:** Run the `build-docs` skill (`make docs`).

#### Validation Checklist

Before considering a task complete, verify:
- [ ] .claude/CHANGELOG.md updated with change details
- [ ] All affected documentation pages updated
- [ ] All YAML examples validate successfully
- [ ] API documentation regenerated (if CRDs changed)
- [ ] Architecture diagrams updated (if structure changed)
- [ ] Code examples compile and run
- [ ] README.md updated (if getting started or features changed)
- [ ] No broken links in documentation
- [ ] Documentation reviewed as if reading for the first time
- [ ] **Documentation builds successfully: `make docs`**

### Mandatory: Update Changelog on Every Code Change

After **ANY** code modification, update `.claude/CHANGELOG.md` with the following format:

```markdown
## [YYYY-MM-DD HH:MM] - Brief Title

**Author:** [Author Name]

### Changed
- `path/to/file.rs`: Description of the change

### Why
Brief explanation of the business or technical reason.

### Impact
- [ ] Breaking change
- [ ] Requires cluster rollout
- [ ] Config change only
- [ ] Documentation only
```

**CRITICAL REQUIREMENT**:
- The `**Author:**` line is **MANDATORY** for ALL changelog entries
- This is required for auditing and accountability in a regulated environment
- The author field should contain the name of the person who requested or approved the change
- **NO exceptions** - every changelog entry must have an author attribution
- If the author is unknown, use "Unknown" but investigate to identify the proper author

### Code Comments

All public functions and types **must** have rustdoc comments:

```rust
/// Reconciles the BindZone custom resource.
///
/// # Arguments
/// * `zone` - The BindZone CR to reconcile
/// * `ctx` - Controller context with client and state
///
/// # Errors
/// Returns `ReconcileError` if DNS zone update fails or API is unreachable.
pub async fn reconcile(zone: Arc<BindZone>, ctx: Arc<Context>) -> Result<Action, ReconcileError> {
```

### Architecture Decision Records (ADRs)

For significant design decisions, create `/docs/adr/NNNN-title.md`:

```markdown
# ADR-NNNN: Title

## Status
Proposed | Accepted | Deprecated | Superseded by ADR-XXXX

## Context
What is the issue we're facing?

## Decision
What have we decided to do?

## Consequences
What are the trade-offs?
```

---

## 🦀 Rust Workflow

### CRITICAL: Test-Driven Development (TDD) Workflow

**MANDATORY: ALWAYS write tests FIRST before implementing functionality.**

This project follows strict Test-Driven Development practices. You MUST follow the Red-Green-Refactor cycle for ALL code changes.

> **How:** Follow the `tdd-workflow` skill (RED → GREEN → REFACTOR).

#### TDD Benefits:

- **Design First**: Forces you to think about API and behavior before implementation
- **Complete Coverage**: All code has tests because tests come first
- **Prevents Over-Engineering**: Only write code needed to pass tests
- **Regression Safety**: Refactoring is safe because tests verify behavior
- **Living Documentation**: Tests document expected behavior

#### When to Write Tests First:

- ✅ **New Features**: Write tests defining the feature behavior, then implement
- ✅ **Bug Fixes**: Write a failing test that reproduces the bug, then fix it
- ✅ **Refactoring**: Ensure existing tests pass, add new tests for edge cases
- ✅ **Performance Optimizations**: Write performance tests, then optimize

#### Exceptions to TDD:

TDD is MANDATORY except for:
- Exploratory/prototype code (must be marked as such and removed before merging)
- Simple refactoring that doesn't change behavior (existing tests verify correctness)

**REMEMBER**: If you're writing implementation code before tests, STOP and write tests first!

### After Modifying Any `.rs` File

**CRITICAL: At the end of EVERY task that modifies Rust files, run the `cargo-quality` skill.**

> **How:** Run the `cargo-quality` skill. Fix ALL clippy warnings. Task is NOT complete until all three commands pass.

**CRITICAL: After ANY Rust code modification, you MUST also verify:**

1. **Function documentation is accurate**:
   - Check rustdoc comments match what the function actually does
   - Verify all `# Arguments` match the actual parameters
   - Verify `# Returns` matches the actual return type
   - Verify `# Errors` describes all error cases
   - Update examples in doc comments if behavior changed

2. **Unit tests are accurate and passing**:
   - Check test assertions match the new behavior
   - Update test expectations if behavior changed
   - Ensure all tests compile and run successfully
   - Add new tests for new behavior/edge cases

3. **End-user documentation is updated**:
   - Update relevant files in `docs/` directory
   - Update examples in `examples/` directory
   - Ensure `.claude/CHANGELOG.md` reflects the changes
   - Verify example YAML files validate successfully

### Unit Testing Requirements

**CRITICAL: When modifying ANY Rust code, you MUST update, add, or delete unit tests accordingly:**

1. **Adding New Functions/Methods:**
   - MUST add unit tests for ALL new public functions
   - Test both success and failure scenarios
   - Include edge cases and boundary conditions

2. **Modifying Existing Functions:**
   - MUST update existing tests to reflect changes
   - Add new tests if new behavior or code paths are introduced
   - Ensure ALL existing tests still pass

3. **Deleting Functions:**
   - MUST delete corresponding unit tests
   - Remove or update integration tests that depended on deleted code

---

## Service Mesh

5. **Test Quality Standards:**
   - Use descriptive test names (e.g., `test_reconcile_creates_zone_when_missing`)
   - Follow the Arrange-Act-Assert pattern
   - Mock external dependencies (k8s API, external services)
   - Test error conditions, not just happy paths
   - Ensure tests are deterministic (no flaky tests)

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
