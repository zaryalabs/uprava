SHELL := /bin/sh

SUDO ?= sudo
DOCKER_CONFIG ?= /root/.docker
DOCKER ?= $(SUDO) docker --config $(DOCKER_CONFIG)
COMPOSE ?= $(DOCKER) compose
COMPOSE_FILE ?= compose.yaml
CORE_CONFIG ?= /etc/uprava/core.env
NODE_CONFIG ?= /etc/uprava/node.env
COMPOSE_CMD = $(COMPOSE) --env-file $(CORE_CONFIG) --env-file .env.release -f $(COMPOSE_FILE)
SYSTEMD_UNIT ?= uprava-node.service
RELEASES_DIR ?= builds/releases
RELEASE ?=
OPS_SCRIPT_DIR ?= $(if $(wildcard $(CURDIR)/scripts/backup-sqlite.sh),$(CURDIR)/scripts,$(CURDIR)/../scripts)

.DEFAULT_GOAL := help

help: ## Show available targets
	@awk 'BEGIN {FS = ":.*## "}; /^[a-zA-Z0-9_.-]+:.*## / {printf "  %-14s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

status: ## Show Core/Web containers and Node service status
	$(COMPOSE_CMD) ps
	$(SUDO) systemctl status $(SYSTEMD_UNIT) --no-pager || true

logs: ## Show recent Core/Web and Node logs
	$(COMPOSE_CMD) logs --tail=150
	$(SUDO) journalctl -u $(SYSTEMD_UNIT) -n 150 --no-pager || true

releases: ## List locally available release manifests
	@find $(RELEASES_DIR) -maxdepth 1 -type f -name '*.env.release' -print 2>/dev/null | sort || true

require-release:
	@case "$(RELEASE)" in \
		''|[!A-Za-z0-9]*|*[!A-Za-z0-9._-]*) echo "Set a safe RELEASE=<release-id>" >&2; exit 1 ;; \
	esac

validate: require-release ## Validate the selected immutable release and host inputs
	@set -eu; \
	manifest="$(RELEASES_DIR)/$(RELEASE).env.release"; \
	test -f "$$manifest" || { echo "Missing $$manifest" >&2; exit 1; }; \
	. "$$manifest"; \
	test "$${UPRAVA_RELEASE_ID:-}" = "$(RELEASE)" || { echo "Manifest identity mismatch" >&2; exit 1; }; \
	test "$${UPRAVA_CORE_STATE_DIR:-}" = state/core || { echo "Core state path must be state/core" >&2; exit 1; }; \
	test "$${UPRAVA_CORE_CONFIG:-}" = "$(CORE_CONFIG)" || { echo "Core config must be $(CORE_CONFIG)" >&2; exit 1; }; \
	test "$${UPRAVA_NODE_CONFIG:-}" = "$(NODE_CONFIG)" || { echo "Node config must be $(NODE_CONFIG)" >&2; exit 1; }; \
	test "$${UPRAVA_NODE_STATE_PATH:-}" = /var/lib/uprava-node/node.sqlite || { echo "Unexpected Node state path" >&2; exit 1; }; \
	test -n "$${UPRAVA_RELEASE_FAMILY:-}" || { echo "Release family is required" >&2; exit 1; }; \
	test -n "$${UPRAVA_RELEASE_SHA:-}"; \
	test -n "$${UPRAVA_NODE_VERSION:-}"; \
	test -n "$${UPRAVA_AUTO_APPROVE_NODE_NAME:-}"; \
	case "$${UPRAVA_CORE_IMAGE:-}:$${UPRAVA_WEB_IMAGE:-}:$${UPRAVA_GENERATED_UI_BUILDER_IMAGE:-}:$${UPRAVA_NODE_ARTIFACT:-}" in \
		*@sha256:*@sha256:*@sha256:*@sha256:*) ;; \
		*) echo "Every runtime artifact must be digest-pinned" >&2; exit 1 ;; \
	esac; \
	test -f "$(CORE_CONFIG)" || { echo "Missing $(CORE_CONFIG)" >&2; exit 1; }; \
	test -f "$(NODE_CONFIG)" || { echo "Missing $(NODE_CONFIG)" >&2; exit 1; }

activate: validate ## Activate the selected manifest and release directory
	@set -eu; \
	manifest="$(RELEASES_DIR)/$(RELEASE).env.release"; \
	$(SUDO) mkdir -p "$(RELEASES_DIR)/$(RELEASE)"; \
	$(SUDO) ln -sfn "$$manifest" .env.release; \
	$(SUDO) ln -sfn "$(RELEASES_DIR)/$(RELEASE)" current

remember-current: ## Preserve the active release as the automatic rollback target
	@set -eu; \
	if test ! -e .env.release && test ! -L .env.release && test ! -e current && test ! -L current; then \
		$(SUDO) rm -f .env.previous previous; \
		exit 0; \
	fi; \
	test -L .env.release && test -L current || \
		{ echo "Active release links are incomplete or not symbolic links" >&2; exit 1; }; \
	manifest="$$(readlink .env.release)"; \
	release_dir="$$(readlink current)"; \
	test -f "$$manifest" || { echo "Active release manifest is missing" >&2; exit 1; }; \
	. "$$manifest"; \
	test -n "$${UPRAVA_RELEASE_ID:-}" || { echo "Active release identity is missing" >&2; exit 1; }; \
	case "$${UPRAVA_RELEASE_ID}" in \
		''|[!A-Za-z0-9]*|*[!A-Za-z0-9._-]*) echo "Active release identity is unsafe" >&2; exit 1 ;; \
	esac; \
	test "$$(realpath "$$manifest")" = "$$(realpath "$(RELEASES_DIR)/$${UPRAVA_RELEASE_ID}.env.release")" || \
		{ echo "Active release manifest link is inconsistent" >&2; exit 1; }; \
	test "$$(realpath "$$release_dir")" = "$$(realpath "$(RELEASES_DIR)/$${UPRAVA_RELEASE_ID}")" || \
		{ echo "Active release directory link is inconsistent" >&2; exit 1; }; \
	test -x "$$release_dir/uprava-node" || { echo "Active Node binary is missing" >&2; exit 1; }; \
	$(SUDO) ln -sfn "$$manifest" .env.previous; \
	$(SUDO) ln -sfn "$$release_dir" previous

rollback-activate: ## Restore the preserved release links after failed validation
	@set -eu; \
	test -L .env.previous && test -L previous || { echo "No previous release is available" >&2; exit 1; }; \
	candidate_family="$$(sed -n 's/^UPRAVA_RELEASE_FAMILY=//p' .env.release | head -n 1)"; \
	candidate_core_state="$$(sed -n 's/^UPRAVA_CORE_STATE_DIR=//p' .env.release | head -n 1)"; \
	candidate_node_state="$$(sed -n 's/^UPRAVA_NODE_STATE_PATH=//p' .env.release | head -n 1)"; \
	manifest="$$(readlink .env.previous)"; \
	release_dir="$$(readlink previous)"; \
	test -f "$$manifest" || { echo "Previous release manifest is missing" >&2; exit 1; }; \
	. "$$manifest"; \
	test -n "$${UPRAVA_RELEASE_ID:-}" || { echo "Previous release identity is missing" >&2; exit 1; }; \
	case "$${UPRAVA_RELEASE_ID}" in \
		''|[!A-Za-z0-9]*|*[!A-Za-z0-9._-]*) echo "Previous release identity is unsafe" >&2; exit 1 ;; \
	esac; \
	test -n "$$candidate_family" && test "$${UPRAVA_RELEASE_FAMILY:-}" = "$$candidate_family" || \
		{ echo "Previous release family is incompatible with the candidate state" >&2; exit 1; }; \
	test "$${UPRAVA_CORE_STATE_DIR:-}" = "$$candidate_core_state" && \
		test "$${UPRAVA_NODE_STATE_PATH:-}" = "$$candidate_node_state" || \
		{ echo "Previous release does not use the candidate state slots" >&2; exit 1; }; \
	test "$$(realpath "$$manifest")" = "$$(realpath "$(RELEASES_DIR)/$${UPRAVA_RELEASE_ID}.env.release")" || \
		{ echo "Previous release manifest link is inconsistent" >&2; exit 1; }; \
	test "$$(realpath "$$release_dir")" = "$$(realpath "$(RELEASES_DIR)/$${UPRAVA_RELEASE_ID}")" || \
		{ echo "Previous release directory link is inconsistent" >&2; exit 1; }; \
	test -x "$$release_dir/uprava-node" || { echo "Previous Node binary is missing" >&2; exit 1; }; \
	$(SUDO) ln -sfn "$$manifest" .env.release; \
	$(SUDO) ln -sfn "$$release_dir" current

pull: ## Pull Core/Web images and install the verified Node artifact
	@test -e .env.release || { echo "Missing active .env.release" >&2; exit 1; }
	$(COMPOSE_CMD) pull
	$(MAKE) --no-print-directory -f "$(firstword $(MAKEFILE_LIST))" node-artifact \
		SUDO="$(SUDO)" RELEASES_DIR="$(RELEASES_DIR)" DOCKER_CONFIG="$(DOCKER_CONFIG)"

node-artifact: ## Extract and checksum the active Node artifact
	@set -eu; \
	. ./.env.release; \
	release_dir="$(RELEASES_DIR)/$${UPRAVA_RELEASE_ID}"; \
	output="$$release_dir/uprava-node"; \
	tmp="$$output.tmp.$$$$"; \
	container_id=""; \
	cleanup() { test -z "$$container_id" || $(DOCKER) rm -f "$$container_id" >/dev/null 2>&1 || true; $(SUDO) rm -f "$$tmp"; }; \
	trap cleanup EXIT INT TERM; \
	$(DOCKER) pull "$${UPRAVA_NODE_ARTIFACT}" >/dev/null; \
	container_id="$$( $(DOCKER) create "$${UPRAVA_NODE_ARTIFACT}" )"; \
	$(DOCKER) cp "$$container_id:/usr/local/bin/uprava-node" "$$tmp"; \
	actual="$$(sha256sum "$$tmp" | awk '{print $$1}')"; \
	test "$$actual" = "$${UPRAVA_NODE_SHA256}" || { echo "Node checksum mismatch" >&2; exit 1; }; \
	$(SUDO) chmod 755 "$$tmp"; \
	$(SUDO) mv "$$tmp" "$$output"; \
	cleanup; trap - EXIT INT TERM

deploy: validate ## Preserve the current release, then pull and apply the selected release
	$(MAKE) --no-print-directory -f "$(firstword $(MAKEFILE_LIST))" remember-current \
		SUDO="$(SUDO)" RELEASES_DIR="$(RELEASES_DIR)"
	$(MAKE) --no-print-directory -f "$(firstword $(MAKEFILE_LIST))" activate \
		RELEASE="$(RELEASE)" SUDO="$(SUDO)" RELEASES_DIR="$(RELEASES_DIR)" \
		CORE_CONFIG="$(CORE_CONFIG)" NODE_CONFIG="$(NODE_CONFIG)"
	$(MAKE) --no-print-directory -f "$(firstword $(MAKEFILE_LIST))" pull \
		SUDO="$(SUDO)" RELEASES_DIR="$(RELEASES_DIR)" CORE_CONFIG="$(CORE_CONFIG)" \
		NODE_CONFIG="$(NODE_CONFIG)" COMPOSE_FILE="$(COMPOSE_FILE)" DOCKER_CONFIG="$(DOCKER_CONFIG)"
	$(COMPOSE_CMD) up -d --remove-orphans
	$(SUDO) systemctl daemon-reload
	$(SUDO) systemctl enable $(SYSTEMD_UNIT) >/dev/null
	$(SUDO) systemctl restart $(SYSTEMD_UNIT)

rollback: rollback-activate ## Reactivate the release preserved before deploy
	$(COMPOSE_CMD) up -d --remove-orphans
	$(SUDO) systemctl daemon-reload
	$(SUDO) systemctl restart $(SYSTEMD_UNIT)

deactivate: ## Stop a failed first release and remove its active links
	@set -u; status=0; \
	$(COMPOSE_CMD) down || status=$$?; \
	$(SUDO) systemctl stop $(SYSTEMD_UNIT) || status=$$?; \
	$(SUDO) rm -f .env.release current; \
	exit $$status

down: ## Stop Uprava runtime units without deleting state
	$(COMPOSE_CMD) down
	$(SUDO) systemctl stop $(SYSTEMD_UNIT)

backup: ## Archive Core state through SQLite's online backup API
	@set -eu; \
	backup_id="$$(date -u +%Y%m%d-%H%M%S)"; \
	mkdir -p "backups/$$backup_id"; \
	"$(OPS_SCRIPT_DIR)/backup-sqlite.sh" state/core/core.sqlite "backups/$$backup_id/core.sqlite"; \
	tar -czf "backups/uprava-core-data-$$backup_id.tgz" -C "backups/$$backup_id" core.sqlite; \
	rm -rf "backups/$$backup_id"

restore-verify: ## Verify a SQLite backup archive without activating it
	@test -n "$(BACKUP)" || { echo "Set BACKUP=<file>" >&2; exit 1; }
	@test -f "$(BACKUP)" || { echo "Missing backup $(BACKUP)" >&2; exit 1; }
	@set -eu; \
	tmp_dir="$$(mktemp -d)"; \
	trap 'rm -rf "$$tmp_dir"' EXIT INT TERM; \
	tar -xzf "$(BACKUP)" -C "$$tmp_dir"; \
	"$(OPS_SCRIPT_DIR)/verify-sqlite-backup.sh" "$$(find "$$tmp_dir" -name core.sqlite -type f -print -quit)"

.PHONY: help status logs releases require-release validate activate remember-current rollback-activate pull node-artifact deploy rollback deactivate down backup restore-verify
