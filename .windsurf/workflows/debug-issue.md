# Debug Issue

Workflow for debugging a problem in the codebase.

## Steps

1. **Reproduce the issue**
   - Get exact steps to reproduce
   - Identify relevant error messages
   - Note which crate/module is affected

2. **Locate relevant code**
   - Use grep to find related code:
   ```bash
   grep -r "error_message_or_function" crates/
   ```
   - Check the crate identified in error traces

3. **Add diagnostic logging** (if needed)
   ```rust
   tracing::debug!("Variable state: {:?}", variable);
   tracing::error!("Unexpected condition: {}", condition);
   ```

4. **Run with debug logging**
   ```bash
   RUST_LOG=debug cargo run -p oxide-{crate}
   ```

5. **Write a failing test** that reproduces the bug
   ```rust
   #[test]
   fn test_reproduces_bug_123() {
       // Setup that triggers the bug
       // Assert expected vs actual behavior
   }
   ```

6. **Identify root cause**
   - Trace through the code path
   - Check assumptions
   - Verify data transformations

7. **Implement the fix**
   - Make minimal changes to fix the issue
   - Prefer upstream fixes over downstream workarounds
   - Don't over-engineer

8. **Verify the fix**
   ```bash
   cargo test -p oxide-{crate} test_reproduces_bug_123
   ```

9. **Run full test suite**
   ```bash
   cargo test --workspace
   ```

10. **Remove diagnostic logging** (keep if useful long-term)

11. **Commit the fix**
    ```bash
    git add -A
    git commit -m "fix(oxide-{crate}): description of fix

    Closes #123"
    ```
