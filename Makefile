# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT

# Makefile for bind9-rndc-api

# Configuration
IMAGE_NAME ?= bind9-rndc-api
IMAGE_TAG ?= latest
REGISTRY ?= ghcr.io/firestoned
PLATFORMS ?= linux/amd64,linux/arm64

.PHONY: help
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

.PHONY: build
build: ## Build the binary locally
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
	RUST_LOG=debug cargo run

.PHONY: clean
clean: ## Clean build artifacts
	cargo clean
	rm -rf target/
