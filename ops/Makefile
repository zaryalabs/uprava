SHELL := /bin/sh

COMPOSE ?= sudo docker compose
COMPOSE_FILE ?= compose.yaml
COMPOSE_CMD = $(COMPOSE) --env-file .env --env-file .env.release -f $(COMPOSE_FILE)
SOURCE_GIT ?= sudo -Hu uprava git
SOURCE_DIR ?= /srv/uprava-workspaces/uprava
SYSTEMD_UNIT ?= uprava-node.service
RELEASE ?=

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
	@find builds/releases -maxdepth 1 -type f -name '*.env.release' -print 2>/dev/null | sort || true

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
		echo "UPRAVA_CORE_IMAGE=uprava-core:$(RELEASE)"; \
		echo "UPRAVA_WEB_IMAGE=uprava-web:$(RELEASE)"; \
		echo "UPRAVA_NODE_ARTIFACT=local-docker-extracted:uprava-node:$(RELEASE)"; \
		echo "UPRAVA_NODE_SHA256=$$node_sha"; \
	} > "builds/releases/$(RELEASE).env.release"

activate: require-release ## Activate a locally available release
	@test -f "builds/releases/$(RELEASE).env.release" || { echo "Missing builds/releases/$(RELEASE).env.release"; exit 1; }
	@test -x "builds/releases/$(RELEASE)/uprava-node" || { echo "Missing executable builds/releases/$(RELEASE)/uprava-node"; exit 1; }
	ln -sfn "builds/releases/$(RELEASE).env.release" .env.release
	ln -sfn "builds/releases/$(RELEASE)" current

pull: ## Pull Core/Web images when they come from a registry
	$(COMPOSE_CMD) pull --ignore-pull-failures

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
	$(COMPOSE_CMD) exec -T web node -e "const sleep=ms=>new Promise(r=>setTimeout(r,ms));(async()=>{let last;for(let i=0;i<20;i++){try{const r=await fetch('http://127.0.0.1:5173');if(!r.ok)throw new Error(String(r.status));const t=await r.text();if(!t.includes('Uprava'))throw new Error('missing title');return}catch(e){last=e;await sleep(500)}}throw last})()"
	sudo systemctl is-active --quiet $(SYSTEMD_UNIT)

backup: ## Archive Core state directory
	@set -eu; \
	mkdir -p backups; \
	tar -czf "backups/uprava-core-data-$$(date -u +%Y%m%d-%H%M%S).tgz" volumes/core-data

restore: ## Restore Core state from BACKUP=<file>
	@if [ -z "$(BACKUP)" ]; then echo "Set BACKUP=<file>"; exit 1; fi
	@test -f "$(BACKUP)" || { echo "Missing backup $(BACKUP)"; exit 1; }
	tar -xzf "$(BACKUP)"
