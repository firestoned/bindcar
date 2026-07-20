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

# nsupdate builder stage — bindcar shells out to `nsupdate` (RFC 2136 dynamic
# DNS updates) for record management. Install it from Debian 13 (the same distro
# as the cc-debian13 runtime below) and stage the binary plus its shared-library
# dependencies (glibc core excluded — the runtime base provides it). `/lib` deps
# are remapped to `/usr/lib` because the runtime base's `/lib` is a usrmerge
# symlink (COPYing a real `/lib` dir onto it fails). The digest
# is pinned for reproducibility (A11); resolve with
#   docker buildx imagetools inspect debian:13-slim --raw | sha256sum
FROM debian:13-slim@sha256:020c0d20b9880058cbe785a9db107156c3c75c2ac944a6aa7ab59f2add76a7bd AS nsupdate
RUN apt-get update -qq \
 && apt-get install -y -qq --no-install-recommends bind9-dnsutils \
 && mkdir -p /staging/usr/bin \
 && cp -L /usr/bin/nsupdate /staging/usr/bin/ \
 && LD_TRACE_LOADED_OBJECTS=1 /usr/bin/nsupdate 2>/dev/null \
      | awk '/=>/ { print $3 }' | grep -E '^/' \
      | grep -Ev '/(libc|libm|libpthread|libdl|librt|libresolv|libnsl)\.so' \
      | sort -u \
      | while read -r lib; do \
          dst="/staging$lib"; \
          case "$lib" in /lib/*|/lib64/*) dst="/staging/usr$lib" ;; esac; \
          mkdir -p "$(dirname "$dst")"; \
          cp -L "$lib" "$dst"; \
        done

# Runtime stage - Distroless with glibc
# Pinned to the multi-arch manifest-list (OCI image index) digest, NOT a
# platform-specific digest — BuildKit selects the native arch and reproducible
# builds are guaranteed (the floating :nonroot tag could otherwise change bytes
# between builds, breaking SLSA provenance). Resolved via:
#   curl -sI -H 'Accept: application/vnd.oci.image.index.v1+json' \
#     https://gcr.io/v2/distroless/cc-debian13/manifests/nonroot
# Refresh with the get-multiarch-digest skill; Dependabot's docker ecosystem
# keeps it current once pinned.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:aded2458d026e046cb68199db0e5793e1028ffa143f7258f3c4278253e20add7

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

# nsupdate + its shared libraries (for dynamic DNS record management)
COPY --from=nsupdate /staging/ /

# Distroless's :nonroot tag defaults to UID 65532, but set it explicitly as a
# second line of defense (defense-in-depth) so the container never runs as root
# even if the base default ever changes.
USER 65532:65532

# Expose API port
EXPOSE 8080

# Set default environment variables
ENV BIND_ZONE_DIR=/var/cache/bind
ENV API_PORT=8080
ENV RUST_LOG=info
# DISABLE_AUTH is intentionally not baked in: the binary defaults it to false,
# and an ENV whose name matches "AUTH" trips BuildKit's SecretsUsedInArgOrEnv
# lint. Set it explicitly at run time if ever required.

# Start the API server
ENTRYPOINT ["/usr/local/bin/bindcar"]
