#!/bin/bash
# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT

# Fast Docker build script for local development
# This script provides multiple build strategies optimized for speed

set -e

# Source Rust environment
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
STRATEGY="${1:-local}"
TAG="${2:-latest}"
IMAGE_NAME=firestoned/bindcar
REGISTRY="${REGISTRY:-ghcr.io}"
FULL_IMAGE="${REGISTRY}/${IMAGE_NAME}:${TAG}"

print_usage() {
    echo "Usage: $0 [strategy] [tag]"
    echo ""
    echo "Strategies:"
    echo "  local     - Build locally then copy binary (fastest: ~10s)"
    echo "  chef      - Use cargo-chef for optimal caching (first: ~5min, subsequent: ~30s)"
    echo "  chainguard - Use production Chainguard Dockerfile with pre-built binaries (~30s)"
    echo "  distroless - Use production Distroless Dockerfile with pre-built binaries (~30s)"
    echo ""
    echo "Examples:"
    echo "  $0 local              # Fastest, builds locally first"
    echo "  $0 chef               # Best for repeated builds"
    echo "  $0 chainguard         # Production Chainguard build (requires binaries/)"
    echo "  $0 distroless         # Production Distroless build (requires binaries/)"
    echo ""
    echo "Environment variables:"
    echo "  REGISTRY - Docker registry (default: ghcr.io)"
}

if [[ "$STRATEGY" == "--help" ]] || [[ "$STRATEGY" == "-h" ]]; then
    print_usage
    exit 0
fi

echo -e "${GREEN}Building Docker image with strategy: ${STRATEGY}${NC}"
echo -e "${GREEN}Image: ${FULL_IMAGE}${NC}"
echo ""

case "$STRATEGY" in
    local)
        echo -e "${YELLOW}Strategy: Local build (fastest)${NC}"
        echo "Step 1/2: Building binary locally with cargo..."
        K8S_OPENAPI_ENABLED_VERSION=1.31 cargo build --release
        echo ""
        echo "Step 2/2: Building Docker image..."
        docker build -f docker/Dockerfile.local -t "$FULL_IMAGE" .
        ;;

    chef)
        echo -e "${YELLOW}Strategy: Cargo-chef (best caching)${NC}"
        echo "Note: First build will be slow (~5min), subsequent builds are fast (~30s)"
        docker build -f docker/Dockerfile.chef -t "$FULL_IMAGE" .
        ;;

    chainguard)
        echo -e "${YELLOW}Strategy: Production Chainguard (uses pre-built binaries)${NC}"
        echo "Note: Requires binaries in binaries/amd64/ and binaries/arm64/"
        if [ ! -f "binaries/amd64/bindcar" ] || [ ! -f "binaries/arm64/bindcar" ]; then
            echo -e "${RED}ERROR: Pre-built binaries not found!${NC}"
            echo "This strategy requires:"
            echo "  - binaries/amd64/bindcar"
            echo "  - binaries/arm64/bindcar"
            echo ""
            echo "Build binaries first with:"
            echo "  cargo build --release --target x86_64-unknown-linux-gnu"
            echo "  cross build --release --target aarch64-unknown-linux-gnu"
            echo "  mkdir -p binaries/amd64 binaries/arm64"
            echo "  cp target/x86_64-unknown-linux-gnu/release/bindcar binaries/amd64/"
            echo "  cp target/aarch64-unknown-linux-gnu/release/bindcar binaries/arm64/"
            exit 1
        fi
        docker build -f docker/Dockerfile.chainguard -t "$FULL_IMAGE" .
        ;;

    distroless)
        echo -e "${YELLOW}Strategy: Production Distroless (uses pre-built binaries)${NC}"
        echo "Note: Requires binaries in binaries/amd64/ and binaries/arm64/"
        if [ ! -f "binaries/amd64/bindcar" ] || [ ! -f "binaries/arm64/bindcar" ]; then
            echo -e "${RED}ERROR: Pre-built binaries not found!${NC}"
            echo "This strategy requires:"
            echo "  - binaries/amd64/bindcar"
            echo "  - binaries/arm64/bindcar"
            echo ""
            echo "Build binaries first with:"
            echo "  cargo build --release --target x86_64-unknown-linux-gnu"
            echo "  cross build --release --target aarch64-unknown-linux-gnu"
            echo "  mkdir -p binaries/amd64 binaries/arm64"
            echo "  cp target/x86_64-unknown-linux-gnu/release/bindcar binaries/amd64/"
            echo "  cp target/aarch64-unknown-linux-gnu/release/bindcar binaries/arm64/"
            exit 1
        fi
        docker build -f docker/Dockerfile -t "$FULL_IMAGE" .
        ;;

    *)
        echo -e "${RED}Error: Unknown strategy '$STRATEGY'${NC}"
        echo ""
        print_usage
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}✓ Build complete!${NC}"
echo "Image: $FULL_IMAGE"
echo ""
echo "Next steps:"
echo "  docker run --rm $FULL_IMAGE --version"
echo "  docker push $FULL_IMAGE"
echo ""
