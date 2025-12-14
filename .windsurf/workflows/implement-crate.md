# Implement Crate

Workflow for implementing a new feature in an Oxide CI crate.

## Steps

1. **Identify the GitHub issue** for this work
   - Run `gh issue list --label "crate"` to see available crate issues
   - If no issue exists, create one first

2. **Review the issue** to understand scope and requirements
   - Read the issue description, acceptance criteria, and dependencies
   - Check if there are blockers

3. **Review relevant spec schemas** if this involves events
   - Check `spec/schemas/` for related schemas
   - Check `spec/channels/` for channel definitions
   - Check `spec/messages/` for message definitions

4. **Review the architecture** in `ARCHITECTURE.md`
   - Find the section for this crate
   - Understand interfaces and dependencies

5. **Create domain types** in `oxide-core` if needed
   - Add types to appropriate module
   - Add `spec_link!` macro for spec correlation
   - Run `cargo test -p oxide-core`

6. **Implement the crate functionality**
   - Follow the port/adapter pattern
   - Implement traits defined in `oxide-core/src/ports.rs`
   - Add comprehensive error handling

7. **Write tests**
   - Unit tests for business logic
   - Integration tests with testcontainers if applicable
   - Spec validation tests if event payloads

8. **Validate**
   - Run `cargo fmt && cargo clippy`
   - Run `cargo test -p {crate-name}`
   - Run `make lint` if spec changes were made

9. **Commit and push**
   - Use conventional commit format
   - Reference the issue number
   - Create PR if ready for review
