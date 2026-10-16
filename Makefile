CARGO ?= cargo
SQLX ?= sqlx
NPM ?= npm
PODMAN ?= podman

APP_ENVIRONMENT ?= dev
SQLITE_DSN ?= lux.db
KV_PATH ?= lux.redb

.PHONY: all build run test lint fmt verify migrate-up podman-build \
	web-install web-dev web-build web-lint web-typecheck

all: verify lint test build

build:
	$(CARGO) build --release --locked

run:
	APP_ENVIRONMENT="$(APP_ENVIRONMENT)" SQLITE_DSN="$(SQLITE_DSN)" KV_PATH="$(KV_PATH)" $(CARGO) run -p lux-server

test:
	$(CARGO) test --workspace

lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

fmt:
	$(CARGO) fmt --all

verify:
	$(CARGO) deny check

# Needs sqlx-cli: cargo install sqlx-cli --no-default-features --features sqlite,rustls
# mode=rwc creates the database file if it does not exist yet (goose did the same).
migrate-up:
	$(SQLX) migrate run --source server/migrations --database-url "sqlite:$(SQLITE_DSN)?mode=rwc"

podman-build:
	$(PODMAN) build -t lux-server .

web-install:
	$(NPM) --prefix web ci

web-dev:
	$(NPM) --prefix web run dev

web-build:
	$(NPM) --prefix web run build

web-lint:
	$(NPM) --prefix web run lint

web-typecheck:
	$(NPM) --prefix web run typecheck
