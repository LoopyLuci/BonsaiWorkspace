.PHONY: help build test clean run release docker lint format install uninstall docs

# Variables
PROJECT_DIR := Omnisystem
CARGO := cargo
DOCKER := docker
DOCKER_IMAGE := omnisystem:latest
DOCKER_REGISTRY := docker.io
VERSION := 3.0.0

# Colors for output
BLUE := \033[0;34m
GREEN := \033[0;32m
RED := \033[0;31m
NC := \033[0m # No Color

help: ## Display this help message
	@echo "$(BLUE)Omnisystem Build System$(NC)"
	@echo "$(GREEN)Available targets:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'

build: ## Build the project
	@echo "$(GREEN)Building Omnisystem...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) build

build-release: ## Build release binary
	@echo "$(GREEN)Building Omnisystem (Release)...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) build --release

test: ## Run all tests
	@echo "$(GREEN)Running tests...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) test --verbose

test-release: ## Run tests in release mode
	@echo "$(GREEN)Running tests (Release)...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) test --release --verbose

bench: ## Run benchmarks
	@echo "$(GREEN)Running benchmarks...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) bench

clean: ## Clean build artifacts
	@echo "$(GREEN)Cleaning build artifacts...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) clean
	rm -rf build/ dist/ target/

lint: ## Run clippy linter
	@echo "$(GREEN)Running clippy...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) clippy -- -D warnings

format: ## Format code with rustfmt
	@echo "$(GREEN)Formatting code...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) fmt

check-format: ## Check code formatting without changes
	@echo "$(GREEN)Checking code formatting...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) fmt -- --check

audit: ## Run security audit
	@echo "$(GREEN)Running security audit...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) audit

run: build ## Build and run the project
	@echo "$(GREEN)Running Omnisystem...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) run

run-release: build-release ## Build and run release binary
	@echo "$(GREEN)Running Omnisystem (Release)...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) run --release

install: build-release ## Install binary to /usr/local/bin
	@echo "$(GREEN)Installing Omnisystem...$(NC)"
	mkdir -p $(HOME)/.local/bin
	cp $(PROJECT_DIR)/target/release/omnisystem $(HOME)/.local/bin/ 2>/dev/null || echo "Binary not found in expected location"
	@echo "$(GREEN)Installation complete. Add $(HOME)/.local/bin to PATH if needed.$(NC)"

uninstall: ## Remove installed binary
	@echo "$(GREEN)Uninstalling Omnisystem...$(NC)"
	rm -f $(HOME)/.local/bin/omnisystem
	@echo "$(GREEN)Uninstall complete.$(NC)"

docs: ## Build documentation
	@echo "$(GREEN)Building documentation...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) doc --no-deps --open

docs-test: ## Test documentation examples
	@echo "$(GREEN)Testing documentation examples...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) test --doc

docker-build: ## Build Docker image
	@echo "$(GREEN)Building Docker image ($(DOCKER_IMAGE))...$(NC)"
	$(DOCKER) build -t $(DOCKER_IMAGE) .
	@echo "$(GREEN)Image built successfully.$(NC)"

docker-run: ## Run Docker container
	@echo "$(GREEN)Running Docker container...$(NC)"
	$(DOCKER) run -it --rm $(DOCKER_IMAGE)

docker-push: docker-build ## Push Docker image to registry
	@echo "$(GREEN)Pushing to registry...$(NC)"
	$(DOCKER) tag $(DOCKER_IMAGE) $(DOCKER_REGISTRY)/$(DOCKER_IMAGE)
	$(DOCKER) push $(DOCKER_REGISTRY)/$(DOCKER_IMAGE)

docker-clean: ## Remove Docker image
	@echo "$(GREEN)Removing Docker image...$(NC)"
	$(DOCKER) rmi $(DOCKER_IMAGE)

setup-dev: ## Set up development environment
	@echo "$(GREEN)Setting up development environment...$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) build
	$(MAKE) lint
	@echo "$(GREEN)Development environment ready!$(NC)"

ci: lint test audit ## Run CI checks locally
	@echo "$(GREEN)All CI checks passed!$(NC)"

all: clean build test lint docs ## Run build, test, lint, and generate docs
	@echo "$(GREEN)All tasks completed successfully!$(NC)"

version: ## Display version information
	@echo "$(BLUE)Omnisystem v$(VERSION)$(NC)"
	cd $(PROJECT_DIR) && $(CARGO) --version
	$(DOCKER) --version

info: version ## Display project information
	@echo ""
	@echo "$(GREEN)Project Structure:$(NC)"
	@echo "  Root: $(PWD)"
	@echo "  Project: $(PROJECT_DIR)"
	@echo "  Cargo.toml: $(PROJECT_DIR)/Cargo.toml"
	@echo ""
	@echo "$(GREEN)Key Commands:$(NC)"
	@echo "  make build          - Build the project"
	@echo "  make test           - Run tests"
	@echo "  make run            - Build and run"
	@echo "  make docker-build   - Build Docker image"
	@echo "  make install        - Install binary"
	@echo "  make clean          - Clean build artifacts"

.DEFAULT_GOAL := help
