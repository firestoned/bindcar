# Testing Standards

## CRITICAL: Test-Driven Development (TDD) Workflow

**MANDATORY: ALWAYS write tests FIRST before implementing functionality.**

This project follows strict Test-Driven Development practices. You MUST follow the Red-Green-Refactor cycle for ALL code changes.

> **How:** Follow the `tdd-workflow` skill (RED → GREEN → REFACTOR).

### TDD Benefits

- **Design First**: Forces you to think about API and behavior before implementation
- **Complete Coverage**: All code has tests because tests come first
- **Prevents Over-Engineering**: Only write code needed to pass tests
- **Regression Safety**: Refactoring is safe because tests verify behavior
- **Living Documentation**: Tests document expected behavior

### When to Write Tests First

- ✅ **New Features**: Write tests defining the feature behavior, then implement
- ✅ **Bug Fixes**: Write a failing test that reproduces the bug, then fix it
- ✅ **Refactoring**: Ensure existing tests pass, add new tests for edge cases
- ✅ **Performance Optimizations**: Write performance tests, then optimize

### Exceptions to TDD

TDD is MANDATORY except for:
- Exploratory/prototype code (must be marked as such and removed before merging)
- Simple refactoring that doesn't change behavior (existing tests verify correctness)

**REMEMBER**: If you're writing implementation code before tests, STOP and write tests first!

---

## After Modifying Any `.rs` File

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

---

## Unit Testing Requirements

**MANDATORY: Every public function MUST have corresponding unit tests.**

**CRITICAL: When modifying ANY Rust code, you MUST update, add, or delete unit tests accordingly:**

### 1. Adding New Functions/Methods

- MUST add unit tests for ALL new public functions
- Test both success and failure scenarios
- Include edge cases and boundary conditions

### 2. Modifying Existing Functions

- MUST update existing tests to reflect changes
- Add new tests if new behavior or code paths are introduced
- Ensure ALL existing tests still pass

### 3. Deleting Functions

- MUST delete corresponding unit tests
- Remove or update integration tests that depended on deleted code

### 4. Refactoring Code

- Update test names and assertions to match refactored code
- Verify test coverage remains the same or improves
- If refactoring changes function signatures, update ALL tests

### 5. Test Quality Standards

- Use descriptive test names (e.g., `test_create_zone_returns_error_when_rndc_fails`)
- Follow the Arrange-Act-Assert pattern
- Mock external dependencies (RNDC, file system) where needed
- Test error conditions, not just happy paths
- Ensure tests are deterministic (no flaky tests)

### 6. Test File Organization

**CRITICAL: ALWAYS place tests in separate `_test.rs` files, NOT embedded in the source file.**

This is the **required pattern** for this codebase. Do NOT embed tests directly in source files.

**Correct Pattern:** `src/foo.rs` → declare `#[cfg(test)] mod foo_test;` at the bottom → `src/foo_test.rs` → contains test functions.

**Examples in This Codebase:**
- `src/main.rs` → `src/main_test.rs` (if needed)
- `src/auth.rs` → `src/auth_test.rs`
- `src/zones.rs` → `src/zones_test.rs`
- `src/rndc.rs` → `src/rndc_test.rs`
- `src/types.rs` → `src/types_test.rs`

> **Note:** This project uses `_test.rs` (singular), not `_tests.rs` (plural).

### Test Coverage Requirements

- **Success path:** Test the primary expected behavior
- **Failure paths:** Test error handling for each possible error type
- **Edge cases:** Empty strings, null values, boundary conditions
- **State changes:** Verify correct state transitions
- **Async operations:** Test timeouts and error propagation

### When to Update Tests

- **ALWAYS** when adding new functions → Add new tests
- **ALWAYS** when modifying functions → Update existing tests
- **ALWAYS** when deleting functions → Delete corresponding tests
- **ALWAYS** when refactoring → Verify tests still cover the same behavior

---

## VERIFICATION

- After ANY Rust code change, run `cargo test` in the modified file's crate
- ALL tests MUST pass before the task is considered complete
- If you add code but cannot write a test, document WHY in the code comments

**Example:**
If you modify `src/zones.rs`:
1. Update/add tests in `src/zones_test.rs` (separate file)
2. Ensure `src/zones.rs` has: `#[cfg(test)] mod zones_test;`
3. Run `cargo test` to verify
4. Ensure ALL tests pass before moving on

---

## Integration Tests

Place in `integration-test/` directory:
- Test full request/response flows against a real BIND9 instance
- Test failure scenarios, not just happy path
- Test end-to-end workflows (create zone → reload → delete zone)
- Test authentication and authorization flows

---

## Test Execution

> **How:** Run the `cargo-quality` skill. For a specific module: `cargo test <module>`. For verbose output: `cargo test -- --nocapture`.

**ALL tests MUST pass before code is considered complete.**
