# Project Guidelines

## Rust Development Environment

**CRITICAL:** Before executing any `cargo` or Rust-related commands, always run:
```bash
source ~/.zshrc
```

This ensures the Rust toolchain is properly loaded in the shell environment.

## Service Mesh

When referencing service mesh in documentation or code, always use **Linkerd** as the example implementation.

## Code Style

### Early Return Pattern (Guard Clauses)

Always use the "early return" or "guard clause" coding style to minimize nested if-else statements and promote clearer, more linear code flow. Handle error conditions or special cases at the beginning of a function and exit early if those conditions are met.

**Key principles:**

1. **Handle preconditions first** - Validate input parameters and other preconditions at the start of a function. If a condition is not met, return immediately (e.g., return `None`, `Err`, or an error value). This prevents the main logic from executing with invalid data.

   ```rust
   fn process_data(data: Option<Vec<String>>) -> Option<ProcessedData> {
       // Handle invalid input early
       let data = match data {
           Some(d) => d,
           None => return None,
       };

       if data.is_empty() {
           return None; // Handle empty data
       }

       // ... rest of the processing logic (happy path)
   }
   ```

2. **Minimize else statements** - Instead of using if-else for mutually exclusive conditions, use early returns within if blocks. If a condition is met and an action is performed, return the result. The code after the if block then implicitly handles the "else" case.

   ```rust
   fn calculate_discount(price: f64, is_premium_member: bool) -> f64 {
       if is_premium_member {
           return price * 0.90; // Apply 10% discount and return
       }

       // No 'else' needed; non-premium members are handled here
       price * 0.95 // Apply 5% discount
   }
   ```

3. **Prioritize readability and clarity** - The goal is to make the code easier to understand by reducing indentation levels and keeping related logic together. When a reader encounters an early return, they know that specific branch of execution has concluded.

4. **Use Result types for error handling** - In Rust, prefer `Result<T, E>` types with early returns using the `?` operator for error propagation:

   ```rust
   fn process_request(config: &Config) -> Result<Response, Error> {
       if config.api_key.is_empty() {
           return Err(Error::MissingApiKey);
       }

       let validated_data = validate_input(&config.data)?;
       let processed = process_validated_data(validated_data)?;

       Ok(Response::new(processed))
   }
   ```

**Benefits:**
- Reduced nesting improves readability and reduces cognitive load
- Clearer code flow with main logic less cluttered by error handling
- Easier to test each condition in isolation
- Fail-fast approach catches invalid states or inputs early in execution
