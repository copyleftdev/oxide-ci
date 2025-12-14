---
trigger: glob
globs: ["*.rs", "Cargo.toml", "Cargo.lock"]
---

# Rust Coding Standards

Rules for writing Rust code in the Oxide CI project.

## General Style

<style>
- Use Rust 2024 edition features
- Follow `rustfmt` formatting (run `cargo fmt`)
- Run `cargo clippy` before committing
- Prefer `thiserror` for error types
- Use `async-trait` for async trait methods
- Prefer `tokio` for async runtime
</style>

## Type Design

<types>
- Use newtype pattern for IDs: `pub struct PipelineId(Uuid)`
- Derive `Debug, Clone, Serialize, Deserialize` on all types
- Use `#[serde(rename_all = "snake_case")]` for enums
- Add `#[non_exhaustive]` to public enums that may grow
- Implement `Display` for user-facing types
</types>

## Error Handling

<errors>
- Define errors in `oxide-core/src/error.rs`
- Use `thiserror::Error` derive macro
- Return `Result<T, Error>` not `Result<T, Box<dyn Error>>`
- Include context in error messages
- Log errors at the boundary, not deep in the stack
</errors>

## Async Code

<async>
- Use `async fn` over `-> impl Future`
- Prefer `tokio::spawn` for background tasks
- Use `tokio::select!` for concurrent operations
- Always handle cancellation gracefully
- Use `tokio::sync::mpsc` for channels
</async>

## Testing

<testing>
- Unit tests in same file as code
- Integration tests in `tests/` directory
- Use `#[tokio::test]` for async tests
- Use `testcontainers` for database/NATS tests
- Mock external services, don't stub internal traits
</testing>

## Documentation

<docs>
- Add `//!` module-level docs to every file
- Document public functions with `///`
- Include examples in doc comments where helpful
- Link to AsyncAPI spec schemas in relevant types
</docs>

## Dependencies

<deps>
- Use workspace dependencies in `Cargo.toml`
- Prefer well-maintained crates from the ecosystem
- Pin versions for reproducibility
- Minimize feature flags to reduce compile time
</deps>
