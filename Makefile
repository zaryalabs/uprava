SHELL := /bin/sh

RUST_MANIFEST := Cargo.toml
WEB_DIR := apps/web
WEB_PACKAGE := $(WEB_DIR)/package.json

ifneq (,$(wildcard pnpm-lock.yaml))
WEB_PM := pnpm
WEB_RUN := pnpm --dir $(WEB_DIR)
else ifneq (,$(wildcard bun.lockb))
WEB_PM := bun
WEB_RUN := cd $(WEB_DIR) && bun
else ifneq (,$(wildcard package-lock.json))
WEB_PM := npm
WEB_RUN := npm --prefix $(WEB_DIR)
else
WEB_PM := npm
WEB_RUN := npm --prefix $(WEB_DIR)
endif

.DEFAULT_GOAL := help

help: ## Show available make targets
	@awk 'BEGIN {FS = ":.*## "}; /^[a-zA-Z0-9_.-]+:.*## / {printf "  %-14s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

init: ## Install local hooks and project dependencies when manifests exist
	@if command -v pre-commit >/dev/null 2>&1; then \
		pre-commit install; \
	else \
		echo "pre-commit is not installed; install it to enable commit hooks"; \
	fi
	@if [ -f "$(RUST_MANIFEST)" ]; then \
		cargo fetch; \
	else \
		echo "No Cargo.toml found; skipping Rust dependency fetch"; \
	fi
	@if [ -f "$(WEB_PACKAGE)" ]; then \
		$(WEB_RUN) install; \
	else \
		echo "No $(WEB_PACKAGE) found; skipping web dependency install"; \
	fi

fmt: docs-fmt rust-fmt web-fmt ## Format all supported project files

l: docs-l rust-l web-l ## Run light checks

dl: l rust-dl web-dl ## Run deep checks

t: rust-t web-t ## Run tests

c: fmt dl t ## Run full local quality gate

pc: ## Run pre-commit hooks on all files
	@if command -v pre-commit >/dev/null 2>&1; then \
		pre-commit run --all-files; \
	else \
		echo "pre-commit is not installed; cannot run hooks"; \
		exit 1; \
	fi

docs-fmt: ## Format/check docs when a formatter is available
	@echo "No docs formatter configured yet; skipping docs format"

docs-l: ## Run lightweight docs checks
	@find README.md AGENTS.md CONTRIBUTING.md docs -type f \( -name '*.md' -o -name '*.toml' -o -name '*.yaml' -o -name '*.yml' -o -name '*.json' \) -print >/dev/null
	@echo "Docs files are present"

rust-fmt: ## Format Rust code when Cargo workspace exists
	@if [ -f "$(RUST_MANIFEST)" ]; then \
		cargo fmt --all; \
	else \
		echo "No Cargo.toml found; skipping Rust format"; \
	fi

rust-l: ## Run Rust format check and clippy when Cargo workspace exists
	@if [ -f "$(RUST_MANIFEST)" ]; then \
		cargo fmt --all -- --check; \
		cargo clippy --workspace --all-targets -- -D warnings; \
	else \
		echo "No Cargo.toml found; skipping Rust lint"; \
	fi

rust-dl: ## Run deeper Rust dependency/config checks when tools are available
	@if [ -f "$(RUST_MANIFEST)" ]; then \
		if command -v cargo-audit >/dev/null 2>&1; then cargo audit; else echo "cargo-audit not installed; skipping audit"; fi; \
		if command -v cargo-deny >/dev/null 2>&1; then cargo deny check; else echo "cargo-deny not installed; skipping deny"; fi; \
		if command -v taplo >/dev/null 2>&1; then taplo fmt --check && taplo lint; else echo "taplo not installed; skipping TOML checks"; fi; \
	else \
		echo "No Cargo.toml found; skipping deep Rust checks"; \
	fi

rust-t: ## Run Rust tests when Cargo workspace exists
	@if [ -f "$(RUST_MANIFEST)" ]; then \
		if cargo nextest --version >/dev/null 2>&1; then \
			cargo nextest run --workspace; \
		else \
			cargo test --workspace; \
		fi; \
	else \
		echo "No Cargo.toml found; skipping Rust tests"; \
	fi

web-r: ## Run web development server when web app exists
	@if [ -f "$(WEB_PACKAGE)" ]; then \
		$(WEB_RUN) run dev; \
	else \
		echo "No $(WEB_PACKAGE) found; skipping web dev server"; \
	fi

web-fmt: ## Format web app when package script exists
	@if [ -f "$(WEB_PACKAGE)" ]; then \
		$(WEB_RUN) run format; \
	else \
		echo "No $(WEB_PACKAGE) found; skipping web format"; \
	fi

web-l: ## Run web lint/typecheck when package scripts exist
	@if [ -f "$(WEB_PACKAGE)" ]; then \
		$(WEB_RUN) run lint; \
		$(WEB_RUN) run typecheck; \
	else \
		echo "No $(WEB_PACKAGE) found; skipping web lint"; \
	fi

web-dl: ## Run web production build when package scripts exist
	@if [ -f "$(WEB_PACKAGE)" ]; then \
		$(WEB_RUN) run build; \
	else \
		echo "No $(WEB_PACKAGE) found; skipping web build"; \
	fi

web-t: ## Run web tests when package scripts exist
	@if [ -f "$(WEB_PACKAGE)" ]; then \
		$(WEB_RUN) run test; \
	else \
		echo "No $(WEB_PACKAGE) found; skipping web tests"; \
	fi

clean: ## Remove common local build and cache artifacts
	rm -rf target htmlcov coverage .pytest_cache .ruff_cache .mypy_cache .ty
	rm -rf $(WEB_DIR)/dist $(WEB_DIR)/coverage

.PHONY: help init fmt l dl t c pc docs-fmt docs-l rust-fmt rust-l rust-dl rust-t web-r web-fmt web-l web-dl web-t clean
