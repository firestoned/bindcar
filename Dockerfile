# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT

# Multi-stage build for bind9-rndc-api container
# This container runs alongside BIND9 and provides an HTTP API for zone management

# Build stage
FROM rust:1.83-bookworm AS builder

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    bind9-utils \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -u 1000 -s /bin/false bind9-api

# Create zone directory
RUN mkdir -p /var/cache/bind && \
    chown bind9-api:bind9-api /var/cache/bind

# Copy binary from builder
COPY --from=builder /build/target/release/bind9-rndc-api /usr/local/bin/bind9-rndc-api

# Set user
USER bind9-api

# Expose API port
EXPOSE 8080

# Set default environment variables
ENV BIND_ZONE_DIR=/var/cache/bind
ENV API_PORT=8080
ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/bind9-rndc-api", "--version"] || exit 1

# Run the API server
ENTRYPOINT ["/usr/local/bin/bind9-rndc-api"]
