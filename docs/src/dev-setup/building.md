# Building from Source

Detailed instructions for building bindcar from source.

## Prerequisites

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Verify installation:
```bash
rustc --version
# Should show: rustc 1.87.0 or later
```

### Install Dependencies

Ubuntu/Debian:
```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev
```

macOS:
```bash
brew install openssl
```

## Build Process

### Debug Build

```bash
cargo build
```

Output: `target/debug/bindcar`

### Release Build

```bash
cargo build --release
```

Output: `target/release/bindcar`

Optimizations:
- LTO enabled
- Optimized for size and speed
- Debug symbols stripped

## Build Options

### Feature Flags

Currently no optional features. All features are enabled by default.

### Custom Optimization

Edit `Cargo.toml` profile:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

## Cross-Compilation

### Linux ARM64

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

### Static Binary

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

## Docker Build

### Build Image

```bash
docker build -t bindcar:latest .
```

### Multi-arch Build

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t bindcar:latest \
  --push .
```

## Troubleshooting

### OpenSSL Not Found

```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev

# macOS
brew install openssl
export OPENSSL_DIR=/usr/local/opt/openssl
```

### Linker Errors

```bash
# Install build tools
sudo apt-get install build-essential
```

### Out of Memory

```bash
# Reduce parallel jobs
cargo build --release -j 2
```

## Verification

```bash
# Run binary
./target/release/bindcar --version

# Test basic functionality
BIND_ZONE_DIR=.tmp/zones ./target/release/bindcar
```

## Next Steps

- [Running Tests](./testing.md) - Test your build
- [Development Setup](./index.md) - Development environment
