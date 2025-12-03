# Contributing to bind9-rndc-api

Thank you for your interest in contributing!

## Development Setup

1. Install Rust (1.83 or later):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository:
   ```bash
   git clone https://github.com/firestoned/bind9-rndc-api.git
   cd bind9-rndc-api
   ```

3. Build and test:
   ```bash
   cargo build
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

## Making Changes

1. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes
3. Add tests for new functionality
4. Ensure all tests pass:
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt
   ```

5. Commit with a descriptive message
6. Push and create a Pull Request

## Code Style

- Follow Rust standard style (use `cargo fmt`)
- Fix all clippy warnings
- Add rustdoc comments for public APIs
- Write tests for new features

## Pull Request Process

1. Update README.md with details of changes if applicable
2. Update CHANGELOG.md following the existing format
3. The PR will be merged once reviewed and approved

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
