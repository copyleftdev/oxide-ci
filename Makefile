# Oxide CI - AsyncAPI Development Commands

.PHONY: help lint validate bundle docs clean setup check fmt test verify e2e e2e-canary \
	dev-up dev-down dev-logs dev-status dev-smoke stack-up stack-down stack-logs

help: ## Display this help message
	@(which chafa >/dev/null && chafa docs/media/logo.png --size=40x20) || (.venv/bin/python scripts/generate_ascii_logo.py 2>/dev/null) || (python3 scripts/generate_ascii_logo.py 2>/dev/null) || echo "Oxide CI"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

# Local development stack: infra in containers, services native, so a code
# change is a `cargo run` away rather than an image rebuild.
dev-up: ## Start the local dev stack (Postgres, NATS, api, scheduler, agent)
	@./scripts/dev.sh up

dev-down: ## Stop the local dev stack
	@./scripts/dev.sh down

dev-logs: ## Follow the dev stack service logs
	@./scripts/dev.sh logs

dev-status: ## Show what the dev stack is running
	@./scripts/dev.sh status

dev-smoke: ## Create a pipeline, run it, and prove it reached an agent
	@./scripts/dev.sh smoke

# The full containerised stack, closer to production and slower to iterate on.
stack-up: ## Start everything in Docker (adds MinIO and Jaeger)
	docker compose up -d --build
	@echo "API on http://localhost:8080 — 'make stack-logs' to watch"

stack-down: ## Stop the containerised stack
	docker compose down

stack-logs: ## Follow the containerised stack logs
	docker compose logs -f oxide-api oxide-scheduler oxide-agent

# Development Setup
dev-setup: ## Setup Python venv and install dependencies
	python3 -m venv .venv
	.venv/bin/pip install -r requirements.txt
	@echo "✅ Development environment setup complete."


# Validate the AsyncAPI spec
lint: validate

validate:
	@echo "🔍 Validating AsyncAPI spec..."
	npx asyncapi validate spec/asyncapi.yaml
	@echo "✅ Spec is valid!"

# Bundle into single file (for distribution)
bundle:
	@echo "📦 Bundling spec..."
	npx asyncapi bundle spec/asyncapi.yaml -o dist/asyncapi.bundled.yaml
	@echo "✅ Bundled to dist/asyncapi.bundled.yaml"

# Generate HTML documentation
docs:
	@echo "📄 Generating documentation..."
	npx asyncapi generate fromTemplate spec/asyncapi.yaml @asyncapi/html-template -o dist/docs
	@echo "✅ Docs generated in dist/docs/"

# Clean generated files
clean:
	rm -rf dist/

# Install dependencies
install:
	npm install

# Watch for changes and validate
watch:
	@echo "👀 Watching for changes..."
	fswatch -o spec/ | xargs -n1 -I{} make validate

# Setup pre-commit hooks
setup:
	@./scripts/setup-hooks.sh

# Rust checks
check:
	cargo check --workspace

fmt:
	cargo fmt --all

test:
	cargo test --workspace --lib --bins

e2e: ## Run end-to-end container tests (needs Docker, minutes)
	cargo test -p oxide-cli --features e2e -- --test-threads=1

e2e-canary: ## Run e2e canary tests against live package registries (non-blocking)
	cargo test -p oxide-cli --features e2e -- --test-threads=1 --ignored

test-integration:
	cargo test -p oxide-tests --features integration

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

# Full verification gate: fmt + clippy + tests, stamps the ledger the
# Claude Code Stop hook checks. Pass ARGS="--spec" to also validate AsyncAPI.
verify: ## Run the full verification gate (fmt, clippy, tests)
	@./scripts/verify.sh $(ARGS)

# Run all pre-commit checks
precommit:
	pre-commit run --all-files
