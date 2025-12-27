# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT

# PRODUCTION DOCKERFILE - Uses pre-built binaries from GitHub Actions
# This Dockerfile is optimized for multi-architecture builds in CI/CD pipelines.
# It uses pre-built glibc binaries (much faster to build than musl) and supports
# both linux/amd64 and linux/arm64 platforms via Docker BuildKit.
#
# Build time: ~30 seconds (just copies pre-built binaries)
#
# Base image: Google Distroless (glibc-based, ~20MB, minimal attack surface)
#
# Usage in CI/CD:
#   1. Build binaries for both architectures using cargo (GNU targets)
#   2. Copy binaries to ./binaries/amd64/ and ./binaries/arm64/
#   3. Build multi-arch image:
#      docker buildx build --platform linux/amd64,linux/arm64 \
#        -t registry/image:tag .
#
# The TARGETARCH build argument is automatically set by Docker BuildKit
# to match the target platform (amd64 or arm64).

# Runtime stage - Distroless with glibc
FROM gcr.io/distroless/cc-debian12:nonroot

ARG TARGETARCH
ARG VERSION=0.1.0

LABEL org.opencontainers.image.source="https://github.com/firestoned/bindcar"
LABEL org.opencontainers.image.description="BIND9 RNDC API Server (Distroless)"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.version="${VERSION}"

# Copy the pre-built binary for the target architecture
# TARGETARCH is automatically set by BuildKit (amd64 or arm64)
# Map docker architectures to our binary paths:
# - amd64 → binaries/amd64/bindcar (x86_64-unknown-linux-gnu)
# - arm64 → binaries/arm64/bindcar (aarch64-unknown-linux-gnu)
COPY --chmod=755 binaries/${TARGETARCH}/bindcar /usr/local/bin/bindcar

# Distroless runs as nonroot user (UID 65532) by default
# No need to specify USER

# Expose API port
EXPOSE 8080

# Set default environment variables
ENV BIND_ZONE_DIR=/var/cache/bind
ENV API_PORT=8080
ENV RUST_LOG=info
ENV DISABLE_AUTH=false

# Start the API server
ENTRYPOINT ["/usr/local/bin/bindcar"]
