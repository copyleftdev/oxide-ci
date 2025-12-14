---
trigger: always_on
---

# Oxide CI Project Rules

These rules apply to all work on the Oxide CI project.

## Project Context

<project_overview>
- Oxide CI is an event-driven CI/CD engine written in Rust
- Uses AsyncAPI 3.0 specification in `spec/` directory
- Follows hexagonal (ports & adapters) architecture
- Event bus: NATS with JetStream
- Database: PostgreSQL with SQLx
- API: Axum HTTP/WebSocket
- Plugins: WASM via Extism
</project_overview>

## Architecture Rules

<architecture>
- All domain types belong in `oxide-core` crate
- External integrations get their own crate (e.g., `oxide-nats`, `oxide-db`)
- Use trait-based ports for all external dependencies
- Never put business logic in adapter crates
- Events must match the AsyncAPI spec schemas
</architecture>

## Code Organization

<crate_structure>
- `crates/oxide-core/` - Domain types, traits, zero external deps
- `crates/oxide-api/` - HTTP/WebSocket server
- `crates/oxide-scheduler/` - Pipeline scheduling
- `crates/oxide-agent/` - Build agent
- `crates/oxide-runner/` - Step execution
- `crates/oxide-plugins/` - WASM plugin host
- `crates/oxide-nats/` - NATS event bus
- `crates/oxide-db/` - PostgreSQL layer
- `crates/oxide-spec/` - Spec-code correlation
</crate_structure>

## Spec Compliance

<spec_rules>
- All event payloads MUST match schemas in `spec/schemas/*.yaml`
- Use `spec_link!` macro to link Rust types to AsyncAPI schemas
- Run `make lint` to validate AsyncAPI spec after changes
- New features require corresponding AsyncAPI schema updates
- Reference `ARCHITECTURE.md` for design decisions
</spec_rules>

## GitHub Integration

<github_rules>
- All work should reference a GitHub issue
- Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`
- Create issues using `gh issue create` with proper labels
- Reference issues in commits: `Closes #123` or `Refs #123`
- Available labels: `epic`, `crate`, `infrastructure`, `integration`, `api`, `spec`, `priority:high/medium/low`
</github_rules>
