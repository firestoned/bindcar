# Rust Style Guide

## Core Principles

- Use `thiserror` for error types, not string errors
- Prefer `anyhow::Result` in binaries, typed errors in libraries
- Use `tracing` for logging, not `println!` or `log`
- Async functions should use `tokio`
- **No magic numbers**: Any numeric literal other than `0` or `1` MUST be declared as a named constant
- **Use early returns/guard clauses**: Minimize nesting by handling edge cases early and returning

---

## Early Return / Guard Clause Pattern

**CRITICAL: Prefer early returns over nested if-else statements.**

The "early return" or "guard clause" coding style emphasizes minimizing nested if-else statements and promoting clearer, more linear code flow. This is achieved by handling error conditions or special cases at the beginning of a function and exiting early if those conditions are met.

### Key Principles

1. **Handle preconditions first**: Validate input parameters and other preconditions at the start of a function. If a condition is not met, return immediately (e.g., `return Err(...)`, `return None`, or `return Ok(())`).

   ```rust
   // ✅ GOOD - Early return for validation
   pub async fn handle_zone_request(
       req: ZoneRequest,
       config: &Config,
   ) -> Result<ZoneResponse> {
       // Guard clause: Check if zone name is valid
       if req.zone_name.is_empty() {
           return Err(AppError::InvalidZoneName("empty zone name".into()));
       }

       // Main logic continues here (happy path)
       let zone = create_zone(&req, config).await?;
       Ok(ZoneResponse::from(zone))
   }

   // ❌ BAD - Nested if-else
   pub async fn handle_zone_request(
       req: ZoneRequest,
       config: &Config,
   ) -> Result<ZoneResponse> {
       if !req.zone_name.is_empty() {
           let zone = create_zone(&req, config).await?;
           Ok(ZoneResponse::from(zone))
       } else {
           Err(AppError::InvalidZoneName("empty zone name".into()))
       }
   }
   ```

2. **Minimize else statements**: Instead of using if-else for mutually exclusive conditions, use early returns within if blocks.

   ```rust
   // ✅ GOOD - No else needed
   fn calculate_discount(price: f64, is_premium_member: bool) -> f64 {
       if is_premium_member {
           return price * 0.90;  // Apply 10% discount and return
       }
       // No 'else' needed; non-premium members are handled here
       price * 0.95  // Apply 5% discount
   }
   ```

3. **Use `?` for error propagation**: Rust's `?` operator is a form of early return for errors. Use it liberally to keep the happy path unindented.

   ```rust
   // ✅ GOOD - Early error returns with ?
   pub async fn add_zone(config: &Config, zone_name: &str) -> Result<()> {
       let zone_dir = config.zone_dir.as_ref().ok_or_else(|| anyhow!("No zone dir"))?;
       let zone_file = write_zone_file(zone_dir, zone_name).await?;
       run_rndc_addzone(zone_name, &zone_file).await?;
       Ok(())
   }
   ```

### Benefits

- **Reduced nesting**: Improves readability and reduces cognitive load
- **Clearer code flow**: The main logic is less cluttered by error handling
- **Easier to test**: Each condition can be tested in isolation
- **Fail-fast approach**: Catches invalid states or inputs early in the execution
- **More maintainable**: Changes to edge cases don't affect the main logic

---

## Magic Numbers Rule

**CRITICAL: All numeric literals (except 0 and 1) MUST be named constants.**

A "magic number" is any numeric literal (other than `0` or `1`) that appears directly in code without explanation.

### Why

- **Readability**: Named constants make code self-documenting
- **Maintainability**: Change the value in one place, not scattered throughout
- **Semantic Meaning**: The constant name explains *why* the value matters
- **Type Safety**: Constants prevent accidental typos in numeric values

### Rules

- **`0` and `1` are allowed** - These are ubiquitous and self-explanatory (empty, none, first item, etc.)
- **All other numbers MUST be named constants** - No exceptions
- Use descriptive names that explain the *purpose*, not just the value

### Examples

```rust
// ✅ GOOD - Named constants
const DEFAULT_ZONE_TTL: u32 = 3600;
const MAX_RETRY_ATTEMPTS: u8 = 3;
const RECONCILE_INTERVAL_SECS: u64 = 300;
const DNS_PORT: u16 = 53;
const API_PORT: u16 = 8080;

fn build_zone(ttl: Option<u32>) -> Zone {
    Zone {
        ttl: ttl.unwrap_or(DEFAULT_ZONE_TTL),
        ..Default::default()
    }
}

// ❌ BAD - Magic numbers
fn build_zone(ttl: Option<u32>) -> Zone {
    Zone {
        ttl: ttl.unwrap_or(3600),  // What does 3600 mean? Why this value?
        ..Default::default()
    }
}
```

### Where to Define Constants

- **Module-level**: For constants used only within one file
- **Crate-level** (`src/constants.rs` or `src/types.rs`): For constants used across modules
- Group related constants together with documentation

### Test Files Exception

Test files (`*_test.rs`) may use literal values for test data when it improves readability and the values are only used once. However, if the same test value appears multiple times or represents a meaningful configuration value, it should still use the global constants.

---

## Code Quality: Use Global Constants for Repeated Strings

When a string literal appears in multiple places across the codebase, it MUST be defined as a global constant and referenced consistently.

### When to Create a Global Constant

- String appears 2+ times in the same file
- String appears in multiple files
- String represents a configuration value (paths, filenames, keys, etc.)
- String is part of an API contract or protocol

---

## Dependency Management

Before adding a new dependency:
1. Check if existing deps solve the problem
2. Verify the crate is actively maintained (commits in last 6 months)
3. Prefer crates from well-known authors or the Rust ecosystem
4. Document why the dependency was added in `.claude/CHANGELOG.md`

---

## Code Comments

All public functions and types **must** have rustdoc comments:

```rust
/// Creates a new DNS zone by writing a zone file and registering it with BIND9 via RNDC.
///
/// # Arguments
/// * `config` - Server configuration including zone directory and RNDC settings
/// * `zone_name` - Fully qualified domain name of the zone (e.g., "example.com")
///
/// # Errors
/// Returns `AppError::ZoneExists` if zone already exists.
/// Returns `AppError::RndcFailed` if RNDC command fails.
pub async fn create_zone(config: &Config, zone_name: &str) -> Result<Zone, AppError> {
```

---

## Things to Never Do

- **Never** use `unwrap()` in production code - use `?` or explicit error handling
- **Never** hardcode ports or paths - make them configurable
- **Never** use `sleep()` for synchronization
- **Never** ignore errors from RNDC commands
- **Never** store mutable state without synchronization (use `Arc<Mutex<T>>` or equivalent)
