# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT

# Makefile for bindcar

# Configuration
IMAGE_NAME ?= bindcar
IMAGE_TAG ?= latest
REGISTRY ?= ghcr.io/firestoned
PLATFORMS ?= linux/amd64,linux/arm64

.PHONY: help
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

.PHONY: build
build: ## Build the binary in release mode
	cargo build --release

.PHONY: test
test: ## Run tests
	cargo test

.PHONY: fmt
fmt: ## Format code
	cargo fmt

.PHONY: clippy
clippy: ## Run clippy
	cargo clippy -- -D warnings

.PHONY: check
check: fmt clippy test ## Run all checks

.PHONY: docker-build
docker-build: ## Build Docker image
	docker build -t $(IMAGE_NAME):$(IMAGE_TAG) .

.PHONY: docker-push
docker-push: ## Push Docker image to registry
	docker tag $(IMAGE_NAME):$(IMAGE_TAG) $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG)
	docker push $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG)

.PHONY: docker-buildx
docker-buildx: ## Build multi-arch Docker image
	docker buildx build --platform $(PLATFORMS) -t $(REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG) --push .

.PHONY: run
run: ## Run the API server locally
	@mkdir -p .tmp/zones
	RUST_LOG=debug BIND_ZONE_DIR=.tmp/zones cargo run

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
	@cargo doc --no-deps --all-features
	@echo "Install mermaid assets and build mdBook documentation..."
	@cd docs && mdbook-mermaid install && mdbook build
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
	cargo doc --no-deps --all-features --open

.PHONY: docs-clean
docs-clean: ## Clean documentation build artifacts
	rm -rf docs/target/
	rm -rf target/doc/

.PHONY: docs-watch
docs-watch: ## Watch and rebuild mdBook documentation on changes
	@command -v mdbook >/dev/null 2>&1 || { echo "Installing mdbook..."; cargo install mdbook; }
	cd docs && mdbook serve
