# Development Setup

Set up your development environment for contributing to bindcar.

## Prerequisites

- Rust 1.89.0 or later
- Git
- Docker (optional, for testing)
- mdBook (for documentation)

## Clone Repository

```bash
git clone https://github.com/firestoned/bindcar.git
cd bindcar
```

## Build

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

See [Building from Source](./building.md) for detailed build instructions.

## Run

```bash
# Create test zone directory
mkdir -p .tmp/zones

# Run with debug logging
RUST_LOG=debug BIND_ZONE_DIR=.tmp/zones cargo run
```

## Test

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

See [Running Tests](./testing.md) for detailed testing information.

## Code Quality

### Format

```bash
cargo fmt
```

### Lint

```bash
cargo clippy -- -D warnings
```

### Check

```bash
# Run all checks
make check
```

## Documentation

### Build Documentation

```bash
# Build all documentation
make docs

# Build and serve locally
make docs-serve
```

### API Documentation

```bash
# Build rustdoc
cargo doc --open
```

## Development Workflow

1. Create feature branch
2. Make changes
3. Run tests: `cargo test`
4. Format code: `cargo fmt`
5. Run clippy: `cargo clippy`
6. Commit changes
7. Push and create PR

## Next Steps

- [Building from Source](./building.md) - Detailed build instructions
- [Running Tests](./testing.md) - Testing guide
- [Contributing](./contributing.md) - Contribution guidelines
