# Build and Check

Workflow for building the project and running all checks.

## Steps

1. **Clean previous build** (optional)
   ```bash
   cargo clean
   ```

2. **Check code compiles**
   ```bash
   cargo check --workspace
   ```
   - Fix any compilation errors before proceeding

3. **Format code**
   ```bash
   cargo fmt
   ```

4. **Run clippy lints**
   ```bash
   cargo clippy --workspace -- -D warnings
   ```
   - Address all warnings

5. **Build debug**
   ```bash
   cargo build --workspace
   ```

6. **Build release** (when ready for deployment)
   ```bash
   cargo build --workspace --release
   ```

7. **Run tests**
   ```bash
   cargo test --workspace
   ```

8. **Validate AsyncAPI spec**
   ```bash
   make lint
   ```

9. **Check documentation builds**
   ```bash
   cargo doc --workspace --no-deps
   ```

10. **Summary**
    - Report build status
    - Report any warnings or issues
    - Estimate binary sizes if release build
