# Oxide CI - AsyncAPI Development Commands

.PHONY: lint validate bundle docs clean

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
