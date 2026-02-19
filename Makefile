.PHONY: help up down logs migrate-up migrate-down migrate-status migrate-new install build clean check web-dev web-build web-preview web-check api-dev api-build api-test seed index fmt lint db-up db-down db-reset

# ──────────────────────────────────────────────
# illuminate.sh — monorepo makefile
# ──────────────────────────────────────────────

# Load .env if present
ifneq (,$(wildcard ./.env))
    include .env
    export
endif

BUN := bun
GO  := go
API_PID := /tmp/illuminate-api.pid
WEB_PID := /tmp/illuminate-web.pid
DB_CONTAINER := illuminate-db
DB_IMAGE := postgres:17-alpine
DB_PORT := 5432

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[33m%-20s\033[0m %s\n", $$1, $$2}'

# ──────────────────────────────────────────────
# Quick commands
# ──────────────────────────────────────────────

up: db-up ## Start DB + API + frontend (backgrounded, use `make logs` to watch)
	@cd api && $(DBMATE) up 2>/dev/null || true
	@mkdir -p /tmp/illuminate-logs
	@echo "Starting API server..."
	@cd api && $(GO) run cmd/server/main.go > /tmp/illuminate-logs/api.log 2>&1 & echo $$! > $(API_PID)
	@echo "Starting frontend dev server..."
	@cd web && $(BUN) dev > /tmp/illuminate-logs/web.log 2>&1 & echo $$! > $(WEB_PID)
	@sleep 1
	@printf '\n'
	@printf '  \033[33m✓\033[0m DB      → localhost:$(DB_PORT)\n'
	@printf '  \033[33m✓\033[0m API     → http://localhost:8080\n'
	@printf '  \033[33m✓\033[0m Web     → http://localhost:5173\n'
	@printf '\n'
	@printf '  Run \033[33mmake logs\033[0m to tail output\n'
	@printf '  Run \033[33mmake down\033[0m to stop\n'

down: ## Stop all running servers
	@if [ -f $(API_PID) ]; then \
		pid=$$(cat $(API_PID)); \
		kill $$pid 2>/dev/null && echo "API stopped (pid $$pid)." || echo "API not running."; \
		rm -f $(API_PID); \
	fi
	@if [ -f $(WEB_PID) ]; then \
		pid=$$(cat $(WEB_PID)); \
		kill $$pid 2>/dev/null && echo "Web stopped (pid $$pid)." || echo "Web not running."; \
		rm -f $(WEB_PID); \
	fi
	@printf 'All servers stopped.\n'

logs: ## Tail logs from API + frontend
	@tail -f /tmp/illuminate-logs/api.log /tmp/illuminate-logs/web.log

logs-api: ## Tail API logs only
	@tail -f /tmp/illuminate-logs/api.log

logs-web: ## Tail frontend logs only
	@tail -f /tmp/illuminate-logs/web.log

# ──────────────────────────────────────────────
# Database
# ──────────────────────────────────────────────

db-up: ## Start PostgreSQL container (creates if needed)
	@if ! docker info >/dev/null 2>&1; then \
		echo "Starting Docker daemon..."; \
		sudo systemctl start docker; \
		sleep 2; \
	fi
	@if docker inspect $(DB_CONTAINER) >/dev/null 2>&1; then \
		if [ "$$(docker inspect -f '{{.State.Running}}' $(DB_CONTAINER) 2>/dev/null)" != "true" ]; then \
			echo "Starting existing DB container..."; \
			docker start $(DB_CONTAINER) >/dev/null; \
		else \
			echo "DB already running."; \
		fi; \
	else \
		echo "Creating DB container..."; \
		docker run -d \
			--name $(DB_CONTAINER) \
			-e POSTGRES_USER=illuminate \
			-e POSTGRES_PASSWORD=illuminate \
			-e POSTGRES_DB=illuminate \
			-p $(DB_PORT):5432 \
			-v illuminate-pgdata:/var/lib/postgresql/data \
			$(DB_IMAGE) >/dev/null; \
	fi
	@printf '  Waiting for PostgreSQL...'
	@for i in 1 2 3 4 5 6 7 8 9 10; do \
		if docker exec $(DB_CONTAINER) pg_isready -U illuminate -q 2>/dev/null; then \
			printf ' \033[32mready\033[0m\n'; \
			break; \
		fi; \
		printf '.'; \
		sleep 1; \
	done

db-down: ## Stop PostgreSQL container
	@docker stop $(DB_CONTAINER) 2>/dev/null && echo "DB stopped." || echo "DB not running."

db-reset: ## Destroy and recreate the database (WARNING: deletes all data)
	@echo "Destroying DB container and volume..."
	@docker rm -f $(DB_CONTAINER) 2>/dev/null || true
	@docker volume rm illuminate-pgdata 2>/dev/null || true
	@$(MAKE) db-up
	@echo "Running migrations..."
	@$(MAKE) migrate-up

# ──────────────────────────────────────────────
# Database migrations (dbmate)
# ──────────────────────────────────────────────

DBMATE := dbmate --migrations-dir ./migrations --no-dump-schema

migrate-up: ## Run all pending migrations
	cd api && $(DBMATE) up

migrate-down: ## Roll back last migration
	cd api && $(DBMATE) rollback

migrate-status: ## Show migration status
	cd api && $(DBMATE) status

migrate-new: ## Create new migration (usage: make migrate-new name=add_column)
	cd api && $(DBMATE) new $(name)

# ──────────────────────────────────────────────
# Install / Build
# ──────────────────────────────────────────────

install: ## Install all dependencies
	cd web && $(BUN) install
	cd api && $(GO) mod tidy

build: web-build api-build ## Production build (frontend + backend)

clean: ## Remove build artifacts and logs
	rm -rf web/build web/.svelte-kit web/node_modules/.vite api/bin /tmp/illuminate-logs
	@echo "cleaned."

check: web-check api-test ## Type check + test everything

# ──────────────────────────────────────────────
# Frontend (web/)
# ──────────────────────────────────────────────

web-dev: ## Start SvelteKit dev server (foreground)
	cd web && $(BUN) dev

web-build: ## Build frontend for production
	cd web && $(BUN) run build

web-preview: ## Preview production build locally
	cd web && $(BUN) run preview

web-check: ## Type check frontend
	cd web && $(BUN) run check

# ──────────────────────────────────────────────
# Backend (api/)
# ──────────────────────────────────────────────

api-dev: ## Start Go API server (foreground)
	cd api && $(GO) run cmd/server/main.go

api-build: ## Build Go binary
	cd api && $(GO) build -o bin/illuminate cmd/server/main.go

api-test: ## Run Go tests
	cd api && $(GO) test ./...

seed: ## Seed repositories from GitHub
	cd api && $(GO) run cmd/seed/main.go

index: ## Index issues from seeded repositories
	cd api && $(GO) run cmd/index/main.go

# ──────────────────────────────────────────────
# Utilities
# ──────────────────────────────────────────────

fmt: ## Format all code
	cd api && $(GO) fmt ./... 2>/dev/null || true

lint: ## Lint all code
	cd api && golangci-lint run ./... 2>/dev/null || true
	cd web && $(BUN) run check
