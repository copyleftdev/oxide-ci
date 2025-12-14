# Run Tests

Workflow for running the test suite.

## Steps

1. **Run formatting check**
   ```bash
   cargo fmt --check
   ```
   - If fails, run `cargo fmt` to fix

2. **Run clippy lints**
   ```bash
   cargo clippy --workspace -- -D warnings
   ```
   - Fix any warnings before proceeding

3. **Run unit tests**
   ```bash
   cargo test --workspace
   ```

4. **Run specific crate tests** if working on one crate
   ```bash
   cargo test -p oxide-core
   cargo test -p oxide-nats
   # etc.
   ```

5. **Run integration tests** (requires Docker)
   ```bash
   # Start dependencies
   docker compose -f docker-compose.dev.yaml up -d
   
   # Run integration tests
   cargo test --workspace --features integration
   ```

6. **Validate AsyncAPI spec**
   ```bash
   make lint
   ```

7. **Run spec correlation tests**
   ```bash
   cargo test -p oxide-spec
   ```

8. **Check test coverage** (optional)
   ```bash
   cargo tarpaulin --workspace --out html
   open tarpaulin-report.html
   ```

9. **Summary**
   - Report any failures
   - Suggest fixes for common issues
