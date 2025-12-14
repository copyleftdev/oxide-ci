---
trigger: model_decision
description: Apply when user is writing tests, running tests, debugging test failures, or discussing testing strategy
---

# Testing Rules

Rules for testing in the Oxide CI project.

## Test Categories

<categories>
- **Unit tests**: Test individual functions/methods in isolation
- **Integration tests**: Test crate interactions with real dependencies
- **Spec tests**: Validate Rust types match AsyncAPI schemas
- **E2E tests**: Full pipeline execution tests
</categories>

## Running Tests

<commands>
- `cargo test --workspace` - Run all unit tests
- `cargo test -p oxide-core` - Test specific crate
- `cargo test --workspace --features integration` - Include integration tests
- `make lint` - Validate AsyncAPI spec
</commands>

## Test Infrastructure

<infrastructure>
- Use `testcontainers` for PostgreSQL and NATS
- Start containers once per test module, not per test
- Use `#[ignore]` for tests requiring external services
- Create test fixtures in `tests/fixtures/`
</infrastructure>

## Test Patterns

<patterns>
- Arrange-Act-Assert structure
- One assertion per test when possible
- Use descriptive test names: `test_run_queued_event_serializes_correctly`
- Test error cases, not just happy path
- Use `pretty_assertions` for better diff output
</patterns>

## Spec Validation Tests

<spec_tests>
```rust
// Link type to spec
spec_link!(RunQueuedPayload, schema = "RunQueuedPayload", file = "schemas/run.yaml");

// Validate in tests
#[test]
fn test_run_queued_matches_spec() {
    let validator = SpecValidator::new("../../spec").unwrap();
    let result = validator.validate::<RunQueuedPayload>();
    assert!(result.is_valid, "{:?}", result.errors);
}
```
</spec_tests>

## Mocking

<mocking>
- Mock external APIs (Stripe, Keygen, Vault) in tests
- Use `wiremock` for HTTP mocking
- Implement test doubles for port traits
- Never mock `oxide-core` types
</mocking>
