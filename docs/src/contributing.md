# Contributing

## Development Setup

```bash
# Clone
git clone https://github.com/copyleftdev/oxide-ci
cd oxide-ci

# Install tools
make setup

# Build
cargo build --workspace

# Test
cargo test --workspace

# Lint
cargo clippy --workspace
```

## Code Style

- Run `cargo fmt` before committing
- All public APIs need documentation
- Follow existing patterns in codebase

## Commit Convention

```
type(scope): description

feat(runner): add Nix environment support
fix(api): correct WebSocket reconnection
docs(readme): update installation instructions
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

## Pull Request Process

1. Create feature branch from `main`
2. Make changes with tests
3. Ensure CI passes
4. Request review

## Architecture Rules

- Domain types belong in `oxide-core`
- External integrations get their own crate
- Use trait-based ports for dependencies
- Events must match AsyncAPI spec schemas

## Running Locally

```bash
# Start dependencies
docker-compose up -d

# Run API server
cargo run -p oxide-api

# Run agent
cargo run -p oxide-agent
```
