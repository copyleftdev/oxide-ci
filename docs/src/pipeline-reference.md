# Pipeline Reference

## Structure

```yaml
name: string          # Pipeline name
version: "1.0"        # Schema version

triggers:             # When to run
  - push:
      branches: [main, develop]
  - pull_request:
      branches: [main]
  - schedule:
      cron: "0 0 * * *"

variables:            # Pipeline variables
  NODE_ENV: production

stages:
  - name: string
    condition: string   # Optional: when to run stage
    environment:        # Optional: execution environment
      type: container|nix|firecracker|host
    steps:
      - name: string
        run: string     # Shell command
        uses: string    # Or plugin reference
        with: {}        # Plugin inputs
        env: {}         # Step environment
        timeout: 300    # Seconds
        continue_on_error: false
```

## Triggers

### Push
```yaml
triggers:
  - push:
      branches: [main, "release/*"]
      paths: ["src/**", "Cargo.toml"]
```

### Pull Request
```yaml
triggers:
  - pull_request:
      branches: [main]
      types: [opened, synchronize]
```

### Schedule
```yaml
triggers:
  - schedule:
      cron: "0 2 * * 1-5"  # Weekdays at 2 AM
```

## Conditions

```yaml
stages:
  - name: deploy
    condition: branch == 'main' && status == 'success'
```

## Matrix Builds

```yaml
stages:
  - name: test
    matrix:
      os: [ubuntu, macos]
      rust: ["1.75", "stable"]
    steps:
      - run: cargo test
```

## Artifacts

```yaml
steps:
  - name: build
    run: cargo build --release
    artifacts:
      - path: target/release/myapp
        retention_days: 7
```
