# Single entry point for local work. `make test` is the one command that
# runs every test in the workspace (it starts the throwaway Postgres).

TEST_DATABASE_URL ?= postgres://finbot:finbot@127.0.0.1:54329/finbot
DEV_DATABASE_URL ?= postgres://finbot:finbot@127.0.0.1:54320/finbot
COVERAGE_MIN_LINES ?= 90

# Arch's rust package ships without rustup's llvm-tools-preview; use the
# system LLVM for coverage when rustup is absent.
ifeq ($(shell command -v rustup 2>/dev/null),)
export LLVM_COV ?= $(shell command -v llvm-cov 2>/dev/null)
export LLVM_PROFDATA ?= $(shell command -v llvm-profdata 2>/dev/null)
endif

.PHONY: test test-db dev dev-db lint deny coverage sqlx-prepare images shellcheck check

test-db:
	docker compose -f compose.test.yml up -d --wait

test: test-db
	DATABASE_URL=$(TEST_DATABASE_URL) cargo test --workspace --all-features

dev-db:
	docker compose -f compose.dev.yml up -d --wait

# Runs the API from source with Swagger UI at http://127.0.0.1:8080/docs.
dev: dev-db
	DATABASE_URL=$(DEV_DATABASE_URL) SWAGGER_ENABLED=true LOG_FORMAT=pretty HTTP_BIND=127.0.0.1:8080 \
		cargo run -p finbot -- serve

lint:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings

deny:
	cargo deny check

coverage: test-db
	DATABASE_URL=$(TEST_DATABASE_URL) cargo llvm-cov --workspace --all-features \
		--fail-under-lines $(COVERAGE_MIN_LINES) --summary-only

# Refreshes .sqlx/ (offline query data used by Docker and CI builds).
sqlx-prepare: test-db
	DATABASE_URL=$(TEST_DATABASE_URL) sqlx migrate run --source crates/pg/migrations
	DATABASE_URL=$(TEST_DATABASE_URL) cargo sqlx prepare --workspace -- --all-targets --all-features

images:
	docker build -f docker/app.Dockerfile -t finbot:local .
	docker build -t finbot-backup:local docker/backup

shellcheck:
	docker run --rm -v "$(CURDIR):/mnt:ro" koalaman/shellcheck:stable -x -s sh -e SC1091 \
		/mnt/scripts/deploy.sh /mnt/scripts/init-secrets.sh /mnt/scripts/install-nginx-vhost.sh /mnt/docker/backup/backup.sh /mnt/docker/backup/restore.sh

check: lint test deny coverage shellcheck
