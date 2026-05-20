# PlausiDen-Annotator Makefile.

.PHONY: help
help: ## Show this help.
	@printf '\n\033[1mPlausiDen-Annotator — Makefile help\033[0m\n\n'
	@printf 'See AGENTS.md + TOOLS.md for the canonical surface.\n\n'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z0-9_.-]+:.*?## / {printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@printf '\n'

.PHONY: build
build: ## Build the workspace.
	cargo build --workspace

.PHONY: test
test: ## Run workspace tests.
	cargo test --workspace

.PHONY: clippy
clippy: ## Run clippy.
	cargo clippy --workspace --all-targets -- -D warnings

.PHONY: fmt
fmt: ## Format the workspace.
	cargo fmt --all

.PHONY: ci
ci: ## CI gate set.
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace

.PHONY: clean
clean: ## Remove cargo build artifacts.
	cargo clean
