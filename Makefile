# Single entry point for local work. `make test` is the one command that
# runs every test in the workspace.

.PHONY: test lint deny check

test:
	cargo test --workspace

lint:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings

deny:
	cargo deny check

check: lint test deny
