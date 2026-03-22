# ObjStor Makefile

.PHONY: help build run test clean docker-build docker-run docker-push

# Default target
.DEFAULT_GOAL := help

# Variables
IMAGE_NAME ?= objstor
IMAGE_TAG ?= latest
DOCKER_REGISTRY ?=
PORT ?= 8080

help: ## Show this help message
	@echo 'ObjStor - S3-Compatible Object Storage'
	@echo ''
	@echo 'Usage:'
	@echo '  make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the Rust project
	cargo build --release

run: ## Run the server
	cargo run

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean
	rm -rf data/logs/*

docker-build: ## Build Docker image
	docker build -t $(IMAGE_NAME):$(IMAGE_TAG) .
	@echo "Built image: $(IMAGE_NAME):$(IMAGE_TAG)"

docker-build-nc: ## Build Docker image without cache
	docker build --no-cache -t $(IMAGE_NAME):$(IMAGE_TAG) .

docker-run: ## Run Docker container
	docker run -d \
		--name objstor \
		-p $(PORT):8080 \
		-v objstor_data:/app/data \
		-e RUST_LOG=info \
		$(IMAGE_NAME):$(IMAGE_TAG)

docker-stop: ## Stop Docker container
	docker stop objstor || true
	docker rm objstor || true

docker-logs: ## Show Docker container logs
	docker logs -f objstor

docker-shell: ## Shell into running container
	docker exec -it objstor sh

docker-compose-up: ## Start with docker-compose
	docker-compose up -d

docker-compose-down: ## Stop docker-compose
	docker-compose down

docker-compose-logs: ## Show docker-compose logs
	docker-compose logs -f

docker-push: ## Push image to registry
	@if [ -z "$(DOCKER_REGISTRY)" ]; then \
		echo "Error: DOCKER_REGISTRY not set"; \
		exit 1; \
	fi
	docker tag $(IMAGE_NAME):$(IMAGE_TAG) $(DOCKER_REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG)
	docker push $(DOCKER_REGISTRY)/$(IMAGE_NAME):$(IMAGE_TAG)

docker-clean: ## Remove Docker containers and volumes
	docker-compose down -v
	docker rm -f objstor || true
	docker volume rm objstor_data objstor_logs || true

init: ## Initialize configuration
	./scripts/configure.sh

format: ## Format code
	cargo fmt

clippy: ## Run linter
	cargo clippy

check: ## Run checks
	cargo check
	cargo clippy
	cargo test

all: format check docker-build ## Run all checks and build
