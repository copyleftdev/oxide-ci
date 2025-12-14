# Review Codebase

Workflow for understanding and reviewing the Oxide CI codebase.

## Steps

1. **Start with the architecture**
   - Read `ARCHITECTURE.md` for overall design
   - Understand crate responsibilities

2. **Review the AsyncAPI spec**
   - Check `spec/asyncapi.yaml` for overview
   - Browse `spec/schemas/` for data types
   - Browse `spec/channels/` for event channels

3. **Understand the domain types** in `oxide-core`
   - `src/ids.rs` - Entity identifiers
   - `src/pipeline.rs` - Pipeline definition
   - `src/run.rs` - Run lifecycle
   - `src/events.rs` - All event payloads
   - `src/ports.rs` - Trait interfaces

4. **Review key infrastructure crates**
   - `oxide-nats/` - Event bus implementation
   - `oxide-db/` - Database repositories

5. **Check example pipelines**
   - `examples/pipeline.yaml` - Basic example
   - `examples/matrix-pipeline.yaml` - Matrix builds
   - `examples/firecracker-pipeline.yaml` - MicroVM
   - `examples/nix-pipeline.yaml` - Nix flakes

6. **Review GitHub issues** for current work
   ```bash
   gh issue list --label "priority:high"
   ```

7. **Check recent commits** for context
   ```bash
   git log --oneline -20
   ```

8. **Summarize findings**
   - Current state of implementation
   - Key patterns used
   - Open issues and blockers
