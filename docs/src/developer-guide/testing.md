# Running Tests

Test suite and testing practices for bindcar.

## Run All Tests

```bash
cargo test
```

## Test Types

### Unit Tests

Test individual functions and modules:

```bash
cargo test --lib
```

Located in:
- `src/auth.rs` - Authentication tests
- `src/zones.rs` - Zone configuration tests
- `src/rndc.rs` - RNDC executor tests

### Integration Tests

Test API endpoints and workflows:

```bash
cargo test --test '*'
```

Located in `tests/` directory.

## Running Specific Tests

### By Name

```bash
cargo test test_auth_middleware
```

### By Module

```bash
cargo test zones::
```

### With Output

```bash
cargo test -- --nocapture
```

## Test Coverage

Currently no coverage tooling configured. Future improvement.

## Writing Tests

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zone_config_validation() {
        let config = ZoneConfig {
            ttl: 3600,
            // ...
        };
        assert!(config.is_valid());
    }
}
```

### Integration Test Example

```rust
#[tokio::test]
async fn test_create_zone() {
    let app = create_test_app().await;
    
    let response = app
        .post("/api/v1/zones")
        .json(&zone_request)
        .send()
        .await;
        
    assert_eq!(response.status(), StatusCode::CREATED);
}
```

## Continuous Integration

Tests run automatically on:
- Pull requests
- Pushes to main branch

See `.github/workflows/pr.yml`.

## Next Steps

- [Development Setup](./setup.md) - Development environment
- [Contributing](./contributing.md) - Contribution guidelines
