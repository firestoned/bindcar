# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT

# Multi-stage build for bindcar container
# This container runs alongside BIND9 and provides an HTTP API for zone management

# Build stage
FROM rust:1.91.0 AS builder

# Set build target based on architecture
ARG TARGETPLATFORM
ARG BUILDPLATFORM

WORKDIR /build

# Install musl tools for static linking (multi-arch)
RUN case "$TARGETPLATFORM" in \
        "linux/amd64") \
            rustup target add x86_64-unknown-linux-musl && \
            apt-get update && \
            apt-get install -y musl-tools && \
            rm -rf /var/lib/apt/lists/* \
            ;; \
        "linux/arm64") \
            rustup target add aarch64-unknown-linux-musl && \
            apt-get update && \
            apt-get install -y musl-tools && \
            rm -rf /var/lib/apt/lists/* \
            ;; \
        *) \
            echo "Unsupported platform: $TARGETPLATFORM" && exit 1 \
            ;; \
    esac

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Copy source code
COPY src ./src

# Build release binary for musl (static linking) based on target platform
RUN case "$TARGETPLATFORM" in \
        "linux/amd64") \
            cargo build --release --target x86_64-unknown-linux-musl \
            ;; \
        "linux/arm64") \
            cargo build --release --target aarch64-unknown-linux-musl \
            ;; \
    esac

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

# Copy binary from builder (musl target) - architecture-aware
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
        "linux/amd64") \
            echo "Copying amd64 binary" \
            ;; \
        "linux/arm64") \
            echo "Copying arm64 binary" \
            ;; \
    esac

COPY --from=builder /build/target/*/release/bindcar /usr/local/bin/bindcar

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
