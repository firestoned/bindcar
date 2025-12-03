# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT

# Multi-stage build for bindcar container
# This container runs alongside BIND9 and provides an HTTP API for zone management

# Build stage
FROM rust:1.87.0 AS builder

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage
FROM alpine:3.20

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    bind-tools

# Create non-root user
RUN adduser -D -u 1000 -s /sbin/nologin bindcar

# Create zone directory
RUN mkdir -p /var/cache/bind && \
    chown bindcar:bindcar /var/cache/bind

# Copy binary from builder
COPY --from=builder /build/target/release/bindcar /usr/local/bin/bindcar

# Set user
USER bindcar

# Expose API port
EXPOSE 8080

# Set default environment variables
ENV BIND_ZONE_DIR=/var/cache/bind
ENV API_PORT=8080
ENV RUST_LOG=info
ENV DISABLE_AUTH=false

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/bindcar", "--version"] || exit 1

# Run the API server
ENTRYPOINT ["/usr/local/bin/bindcar"]
