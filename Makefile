# Oxide CI - AsyncAPI Development Commands

.PHONY: help lint validate bundle docs clean setup check fmt test

help: ## Display this help message
	@(.venv/bin/python scripts/generate_ascii_logo.py 2>/dev/null) || (python3 scripts/generate_ascii_logo.py 2>/dev/null) || echo "Oxide CI"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

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
	cargo test --workspace --lib

test-integration:
	cargo test -p oxide-tests --features integration

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

# Run all pre-commit checks
precommit:
	pre-commit run --all-files
