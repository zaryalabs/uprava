SHELL := /bin/sh

SUDO ?= sudo
COMPOSE ?= $(SUDO) docker compose
COMPOSE_FILE ?= compose.yaml
COMPOSE_CMD = $(COMPOSE) --env-file .env --env-file .env.release -f $(COMPOSE_FILE)
SOURCE_GIT ?= $(SUDO) -Hu uprava git
SOURCE_DIR ?= /srv/uprava-workspaces/uprava
SYSTEMD_UNIT ?= uprava-node.service
RELEASES_DIR ?= builds/releases
RELEASE ?=
CORE_CONFIG_LINK ?= .env
NODE_CONFIG_LINK ?= /etc/uprava/node.env
RELEASE_FAMILY ?= 0.2.0
CORE_STATE_DIR ?= state/$(RELEASE_FAMILY)/core
CORE_CONFIG ?= configuration/releases/$(RELEASE_FAMILY)/core.env
NODE_CONFIG ?= /etc/uprava/releases/$(RELEASE_FAMILY)/node.env
NODE_STATE_PATH ?= /var/lib/uprava-node/$(RELEASE_FAMILY)/node.sqlite

.DEFAULT_GOAL := help

help: ## Show available targets
	@awk 'BEGIN {FS = ":.*## "}; /^[a-zA-Z0-9_.-]+:.*## / {printf "  %-14s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

ps: ## Show Core/Web containers
	$(COMPOSE_CMD) ps

status: ## Show Core/Web containers and Node service status
	$(COMPOSE_CMD) ps
	sudo systemctl status $(SYSTEMD_UNIT) --no-pager || true

logs: ## Show recent Core/Web and Node logs
	$(COMPOSE_CMD) logs --tail=150
	sudo journalctl -u $(SYSTEMD_UNIT) -n 150 --no-pager || true

releases: ## List locally available release manifests
	@find $(RELEASES_DIR) -maxdepth 1 -type f -name '*.env.release' -print 2>/dev/null | sort || true

require-release:
	@if [ -z "$(RELEASE)" ]; then echo "Set RELEASE=<release-id>"; exit 1; fi

build-local: require-release ## Build a bootstrap release from SOURCE_DIR
	@set -eu; \
	sha="$$($(SOURCE_GIT) -C "$(SOURCE_DIR)" rev-parse HEAD)"; \
	release_dir="builds/releases/$(RELEASE)"; \
	mkdir -p "$$release_dir"; \
	sudo docker build -t "uprava-core:$(RELEASE)" -f "$(SOURCE_DIR)/Dockerfile.core" "$(SOURCE_DIR)"; \
	sudo docker build \
		--build-arg VITE_UPRAVA_API_BASE=/api/v1 \
		-t "uprava-web:$(RELEASE)" \
		-f "$(SOURCE_DIR)/apps/web/Dockerfile" \
		"$(SOURCE_DIR)/apps/web"; \
	sudo docker build -t "uprava-node:$(RELEASE)" -f "$(SOURCE_DIR)/Dockerfile.node" "$(SOURCE_DIR)"; \
	container_id="$$(sudo docker create "uprava-node:$(RELEASE)")"; \
	trap 'sudo docker rm -f "$$container_id" >/dev/null 2>&1 || true' EXIT; \
	sudo docker cp "$$container_id:/usr/local/bin/uprava-node" "$$release_dir/uprava-node"; \
	sudo chmod 755 "$$release_dir/uprava-node"; \
	node_sha="$$(sha256sum "$$release_dir/uprava-node" | awk '{print $$1}')"; \
	{ \
		echo "UPRAVA_RELEASE_ID=$(RELEASE)"; \
		echo "UPRAVA_RELEASE_SHA=$$sha"; \
		echo "UPRAVA_RELEASE_AT=$$(date -u +%Y-%m-%dT%H:%M:%SZ)"; \
		echo "UPRAVA_RELEASE_FAMILY=$(RELEASE_FAMILY)"; \
		echo "UPRAVA_CORE_STATE_DIR=$(CORE_STATE_DIR)"; \
		echo "UPRAVA_CORE_CONFIG=$(CORE_CONFIG)"; \
		echo "UPRAVA_NODE_CONFIG=$(NODE_CONFIG)"; \
		echo "UPRAVA_NODE_STATE_PATH=$(NODE_STATE_PATH)"; \
		echo "UPRAVA_CORE_IMAGE=uprava-core:$(RELEASE)"; \
		echo "UPRAVA_WEB_IMAGE=uprava-web:$(RELEASE)"; \
		echo "UPRAVA_NODE_ARTIFACT=local-docker-extracted:uprava-node:$(RELEASE)"; \
		echo "UPRAVA_NODE_SHA256=$$node_sha"; \
	} > "builds/releases/$(RELEASE).env.release"

activate: require-release ## Activate a locally available release
	@set -eu; \
	manifest="$(RELEASES_DIR)/$(RELEASE).env.release"; \
	test -f "$$manifest" || { echo "Missing $$manifest"; exit 1; }; \
	. "$$manifest"; \
	test "$${UPRAVA_RELEASE_ID:-}" = "$(RELEASE)" || { echo "Manifest release identity mismatch" >&2; exit 1; }; \
	test -n "$${UPRAVA_RELEASE_FAMILY:-}" || { echo "Manifest is missing UPRAVA_RELEASE_FAMILY" >&2; exit 1; }; \
	test -n "$${UPRAVA_CORE_STATE_DIR:-}" || { echo "Manifest is missing UPRAVA_CORE_STATE_DIR" >&2; exit 1; }; \
	test -f "$${UPRAVA_CORE_CONFIG:-}" || { echo "Missing Core config $${UPRAVA_CORE_CONFIG:-}" >&2; exit 1; }; \
	test -f "$${UPRAVA_NODE_CONFIG:-}" || { echo "Missing Node config $${UPRAVA_NODE_CONFIG:-}" >&2; exit 1; }; \
	grep -qx 'UPRAVA_DATABASE_URL=sqlite:///data/core.sqlite' "$$UPRAVA_CORE_CONFIG" || { echo "Core config must use the manifest-mounted /data/core.sqlite" >&2; exit 1; }; \
	grep -qx "UPRAVA_NODE_STATE_PATH=$${UPRAVA_NODE_STATE_PATH:-}" "$$UPRAVA_NODE_CONFIG" || { echo "Node config state path does not match manifest" >&2; exit 1; }; \
	case "$${UPRAVA_NODE_STATE_PATH:-}" in /var/lib/uprava-node/"$$UPRAVA_RELEASE_FAMILY"/*) ;; *) echo "Node state path does not match release family" >&2; exit 1;; esac; \
	case "$${UPRAVA_CORE_STATE_DIR}" in state/"$$UPRAVA_RELEASE_FAMILY"/core) ;; *) echo "Core state path does not match release family" >&2; exit 1;; esac; \
	$(SUDO) mkdir -p "$(RELEASES_DIR)/$(RELEASE)" "$${UPRAVA_CORE_STATE_DIR}"; \
	$(SUDO) ln -sfn "$$manifest" .env.release; \
	$(SUDO) ln -sfn "$${UPRAVA_CORE_CONFIG}" "$(CORE_CONFIG_LINK)"; \
	$(SUDO) ln -sfn "$${UPRAVA_NODE_CONFIG}" "$(NODE_CONFIG_LINK)"; \
	$(SUDO) ln -sfn "$(RELEASES_DIR)/$(RELEASE)" current

rollback: require-release ## Activate a selected prior release; deploy and smoke remain explicit
	@set -eu; \
	release="$(RELEASE)"; \
	case "$$release" in \
		''|[!A-Za-z0-9]*|*[!A-Za-z0-9._-]*) \
			echo "Unsafe RELEASE=$$release; use an alphanumeric release id with only . _ - after the first character" >&2; \
			exit 1; \
		;; \
	esac; \
	manifest="$(RELEASES_DIR)/$$release.env.release"; \
	test -f "$$manifest" || { echo "Missing $$manifest" >&2; exit 1; }; \
	target_release="$$(sed -n 's/^UPRAVA_RELEASE_ID=//p' "$$manifest")"; \
	if [ "$$target_release" != "$$release" ]; then \
		echo "Refusing rollback: $$manifest declares UPRAVA_RELEASE_ID=$$target_release, expected $$release" >&2; \
		exit 1; \
	fi; \
	if [ ! -e .env.release ]; then \
		echo "Missing active .env.release; activate a release before rollback" >&2; \
		exit 1; \
	fi; \
	active_release="$$(sed -n 's/^UPRAVA_RELEASE_ID=//p' .env.release | head -n 1)"; \
	if [ -z "$$active_release" ]; then \
		echo "Active .env.release is missing UPRAVA_RELEASE_ID" >&2; \
		exit 1; \
	fi; \
	if [ "$$active_release" = "$$release" ]; then \
		echo "Refusing rollback: RELEASE=$$release is already active" >&2; \
		exit 1; \
	fi; \
	echo "Rollback preflight passed: $$active_release -> $$release"; \
	echo "Ensure a verified backup exists before continuing; activation changes only release symlinks."; \
	$(MAKE) --no-print-directory -f "$(firstword $(MAKEFILE_LIST))" activate RELEASE="$$release" RELEASES_DIR="$(RELEASES_DIR)" SUDO="$(SUDO)" CORE_CONFIG_LINK="$(CORE_CONFIG_LINK)" NODE_CONFIG_LINK="$(NODE_CONFIG_LINK)"; \
	echo "Rollback release activated: $$release"; \
	echo "Required follow-up (explicit): make deploy, then make smoke; inspect make status.";

pull: ## Pull Core/Web images when they come from a registry
	$(COMPOSE_CMD) pull --ignore-pull-failures
	$(MAKE) --no-print-directory node-artifact

node-artifact: ## Fetch, extract and verify the active Node artifact
	@test -e .env.release || { echo "Missing .env.release; run make activate RELEASE=<release-id> first"; exit 1; }
	@set -eu; \
	. ./.env.release; \
	checksum_file() { \
		if command -v sha256sum >/dev/null 2>&1; then sha256sum "$$1" | awk '{print $$1}'; \
		elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$$1" | awk '{print $$1}'; \
		else echo "sha256sum or shasum is required" >&2; exit 1; fi; \
	}; \
	release_id="$${UPRAVA_RELEASE_ID:-}"; \
	artifact="$${UPRAVA_NODE_ARTIFACT:-}"; \
	expected_sha="$${UPRAVA_NODE_SHA256:-}"; \
	if [ -z "$$release_id" ]; then echo "UPRAVA_RELEASE_ID is missing from .env.release" >&2; exit 1; fi; \
	if [ -z "$$artifact" ]; then echo "UPRAVA_NODE_ARTIFACT is missing from .env.release" >&2; exit 1; fi; \
	if [ -z "$$expected_sha" ]; then echo "UPRAVA_NODE_SHA256 is missing from .env.release" >&2; exit 1; fi; \
	release_dir="$(RELEASES_DIR)/$$release_id"; \
	output="$$release_dir/uprava-node"; \
	case "$$artifact" in \
		local-docker-extracted:*) \
			test -x "$$output" || { echo "Missing executable $$output for local bootstrap artifact"; exit 1; }; \
			actual_sha="$$(checksum_file "$$output")"; \
			test "$$actual_sha" = "$$expected_sha" || { echo "Checksum mismatch for $$output"; exit 1; }; \
			echo "Node artifact already installed at $$output"; \
			exit 0; \
			;; \
	esac; \
	$(SUDO) mkdir -p "$$release_dir"; \
	if ! $(SUDO) docker image inspect "$$artifact" >/dev/null 2>&1; then \
		$(SUDO) docker pull "$$artifact"; \
	fi; \
	container_id="$$( $(SUDO) docker create "$$artifact" )"; \
	tmp_file="$$output.tmp.$$"; \
	cleanup() { $(SUDO) docker rm -f "$$container_id" >/dev/null 2>&1 || true; $(SUDO) rm -f "$$tmp_file"; }; \
	trap cleanup EXIT; \
	$(SUDO) docker cp "$$container_id:/usr/local/bin/uprava-node" "$$tmp_file"; \
	$(SUDO) chmod 755 "$$tmp_file"; \
	actual_sha="$$(checksum_file "$$tmp_file")"; \
	test "$$actual_sha" = "$$expected_sha" || { echo "Checksum mismatch for $$artifact"; echo "expected $$expected_sha"; echo "actual   $$actual_sha"; exit 1; }; \
	$(SUDO) mv "$$tmp_file" "$$output"; \
	trap - EXIT; \
	$(SUDO) docker rm -f "$$container_id" >/dev/null 2>&1 || true; \
	echo "Installed Node artifact at $$output"

up: ## Start or update Core/Web and Node
	$(COMPOSE_CMD) up -d --remove-orphans
	sudo systemctl restart $(SYSTEMD_UNIT)

down: ## Stop Core/Web and Node
	$(COMPOSE_CMD) down
	sudo systemctl stop $(SYSTEMD_UNIT)

restart: ## Restart Core/Web and Node
	$(COMPOSE_CMD) restart
	sudo systemctl restart $(SYSTEMD_UNIT)

deploy: ## Pull, start/update, show status and run smoke
	$(MAKE) --no-print-directory pull
	$(MAKE) --no-print-directory up
	$(MAKE) --no-print-directory status
	$(MAKE) --no-print-directory smoke

smoke: ## Run local server smoke checks
	$(COMPOSE_CMD) exec -T core uprava-server healthcheck 127.0.0.1:8080
	$(COMPOSE_CMD) exec -T web wget -qO- http://127.0.0.1:8080/health
	sudo systemctl is-active --quiet $(SYSTEMD_UNIT)

backup: ## Archive Core state directory
	@set -eu; \
	backup_id="$$(date -u +%Y%m%d-%H%M%S)"; \
	mkdir -p "backups/$$backup_id"; \
	"$(CURDIR)/../scripts/backup-sqlite.sh" \
		"$${UPRAVA_CORE_STATE_DIR:-state/0.2.0/core}/core.sqlite" \
		"backups/$$backup_id/core.sqlite"; \
	tar -czf "backups/uprava-core-data-$$backup_id.tgz" -C "backups/$$backup_id" core.sqlite; \
	rm -rf "backups/$$backup_id"

restore: ## Restore Core state from BACKUP=<file>
	@if [ -z "$(BACKUP)" ]; then echo "Set BACKUP=<file>"; exit 1; fi
	@test -f "$(BACKUP)" || { echo "Missing backup $(BACKUP)"; exit 1; }
	tar -xzf "$(BACKUP)"

restore-verify: ## Verify a SQLite backup archive without activating it
	@if [ -z "$(BACKUP)" ]; then echo "Set BACKUP=<file>"; exit 1; fi
	@test -f "$(BACKUP)" || { echo "Missing backup $(BACKUP)"; exit 1; }
	@set -eu; \
	tmp_dir="$$(mktemp -d)"; \
	trap 'rm -rf "$$tmp_dir"' EXIT INT TERM; \
	tar -xzf "$(BACKUP)" -C "$$tmp_dir"; \
	"$(CURDIR)/../scripts/verify-sqlite-backup.sh" "$$(find "$$tmp_dir" -name core.sqlite -type f -print -quit)"
