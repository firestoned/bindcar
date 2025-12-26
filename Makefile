# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT

# Makefile for bindcar

# Configuration
K8S_OPENAPI_ENABLED_VERSION ?= 1.31
IMAGE_NAME ?= bindcar
IMAGE_TAG ?= latest
REGISTRY ?= ghcr.io/firestoned
PLATFORMS ?= linux/amd64,linux/arm64
NAMESPACE ?= dns-system
KIND_CLUSTER ?= bindy-test

.PHONY: help
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

.PHONY: build
build: ## Build the binary in release mode
	K8S_OPENAPI_ENABLED_VERSION=$(K8S_OPENAPI_ENABLED_VERSION) cargo build --release

.PHONY: test
test: ## Run tests
	K8S_OPENAPI_ENABLED_VERSION=$(K8S_OPENAPI_ENABLED_VERSION) cargo test

.PHONY: fmt
fmt: ## Format code
	cargo fmt

.PHONY: clippy
clippy: ## Run clippy
	K8S_OPENAPI_ENABLED_VERSION=$(K8S_OPENAPI_ENABLED_VERSION) cargo clippy -- -D warnings

.PHONY: check
check: fmt clippy test ## Run all checks

#
# Docker targets
#

.PHONY: docker-build
docker-build: ## Build Docker image (uses cargo-chef strategy)
	./scripts/build-docker-fast.sh chef

.PHONY: docker-build-local
docker-build-local: ## Build Docker image locally (fastest: ~10s)
	./scripts/build-docker-fast.sh local

.PHONY: docker-build-chainguard
docker-build-chainguard: ## Build Chainguard production image (requires binaries/)
	./scripts/build-docker-fast.sh chainguard

.PHONY: docker-build-distroless
docker-build-distroless: ## Build Distroless production image (requires binaries/)
	./scripts/build-docker-fast.sh distroless

.PHONY: docker-push
docker-push: ## Push Docker image to registry
	docker push $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG)

.PHONY: docker-push-kind
docker-push-kind: docker-build-local ## Push Docker image to local kind
	kind load docker-image $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG) --name $(KIND_CLUSTER)

.PHONY: docker-buildx
docker-buildx: ## Build multi-arch Docker image
	docker buildx build --platform $(PLATFORMS) -t $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG) --push .

.PHONY: run
run: ## Run the API server locally
	@mkdir -p .tmp/zones
	K8S_OPENAPI_ENABLED_VERSION=$(K8S_OPENAPI_ENABLED_VERSION) RUST_LOG=debug BIND_ZONE_DIR=.tmp/zones cargo run

.PHONY: clean
clean: ## Clean build artifacts
	cargo clean
	rm -rf target/

# Documentation targets
.PHONY: docs
docs: export PATH := $(HOME)/.cargo/bin:$(PATH)
docs: ## Build all documentation (mdBook + rustdoc + OpenAPI)
	@echo "Building all documentation..."
	@command -v mdbook >/dev/null 2>&1 || { echo "Error: mdbook not found. Install with: cargo install mdbook"; exit 1; }
	@echo "Building rustdoc API documentation..."
	@K8S_OPENAPI_ENABLED_VERSION=$(K8S_OPENAPI_ENABLED_VERSION) cargo doc --no-deps
	@echo "Build mdBook documentation..."
	@cd docs && mdbook build
	@echo "Copying rustdoc into documentation..."
	@mkdir -p docs/target/rustdoc
	@cp -r target/doc/* docs/target/rustdoc/
	@echo "Creating rustdoc index redirect..."
	@echo '<!DOCTYPE html>' > docs/target/rustdoc/index.html
	@echo '<html>' >> docs/target/rustdoc/index.html
	@echo '<head>' >> docs/target/rustdoc/index.html
	@echo '    <meta charset="utf-8">' >> docs/target/rustdoc/index.html
	@echo '    <title>bindcar API Documentation</title>' >> docs/target/rustdoc/index.html
	@echo '    <meta http-equiv="refresh" content="0; url=bindcar/index.html">' >> docs/target/rustdoc/index.html
	@echo '</head>' >> docs/target/rustdoc/index.html
	@echo '<body>' >> docs/target/rustdoc/index.html
	@echo '    <p>Redirecting to <a href="bindcar/index.html">bindcar API Documentation</a>...</p>' >> docs/target/rustdoc/index.html
	@echo '</body>' >> docs/target/rustdoc/index.html
	@echo '</html>' >> docs/target/rustdoc/index.html
	@echo "Generating OpenAPI specification..."
	@$(MAKE) --no-print-directory docs-openapi
	@echo "Documentation built successfully in docs/target/"
	@echo "  - User guide: docs/target/index.html"
	@echo "  - API reference: docs/target/rustdoc/bindcar/index.html"
	@echo "  - OpenAPI spec: docs/target/openapi.json"

.PHONY: docs-openapi
docs-openapi: ## Generate OpenAPI/Swagger specification
	@echo "Starting temporary API server to extract OpenAPI spec..."
	@mkdir -p .tmp/zones docs/target
	@BIND_ZONE_DIR=.tmp/zones cargo run & \
		SERVER_PID=$$!; \
		echo "Waiting for server to start..."; \
		sleep 3; \
		echo "Fetching OpenAPI specification..."; \
		curl -s http://localhost:8080/api/v1/openapi.json > docs/target/openapi.json && \
		echo "OpenAPI specification saved to docs/target/openapi.json" || \
		echo "Failed to fetch OpenAPI specification"; \
		echo "Stopping server..."; \
		kill $$SERVER_PID 2>/dev/null || true; \
		sleep 1

.PHONY: docs-serve
docs-serve: docs ## Build and serve documentation locally
	@echo "Serving documentation at http://localhost:3000"
	@cd docs/target && python3 -m http.server 3000

.PHONY: docs-mdbook
docs-mdbook: ## Build mdBook documentation only
	@command -v mdbook >/dev/null 2>&1 || { echo "Installing mdbook..."; cargo install mdbook; }
	@cd docs && mdbook build
	@echo "mdBook documentation built in docs/target/"

.PHONY: docs-rustdoc
docs-rustdoc: ## Build rustdoc API documentation only
	cargo doc --no-deps --open

.PHONY: docs-clean
docs-clean: ## Clean documentation build artifacts
	rm -rf docs/target/
	rm -rf target/doc/

.PHONY: docs-watch
docs-watch: ## Watch and rebuild mdBook documentation on changes
	@command -v mdbook >/dev/null 2>&1 || { echo "Installing mdbook..."; cargo install mdbook; }
	cd docs && mdbook serve

#
# Code Coverage
#

.PHONY: coverage
coverage: ## Generate code coverage report
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { echo "Installing cargo-tarpaulin..."; cargo install cargo-tarpaulin; }
	@echo "Generating coverage report..."
	cargo tarpaulin --lib --out Html --out Lcov --output-dir coverage
	@echo "Coverage report generated in coverage/"
	@echo "  - HTML: coverage/index.html"
	@echo "  - LCOV: coverage/lcov.info"

.PHONY: coverage-open
coverage-open: coverage ## Generate and open coverage report in browser
	@command -v open >/dev/null 2>&1 && open coverage/index.html || \
	command -v xdg-open >/dev/null 2>&1 && xdg-open coverage/index.html || \
	echo "Coverage report available at coverage/index.html"

.PHONY: coverage-clean
coverage-clean: ## Clean coverage artifacts
	rm -rf coverage/ cobertura.xml
