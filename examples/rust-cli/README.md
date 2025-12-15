# Rust CLI Example

A typical Rust CLI tool pipeline with caching, matrix builds, and GitHub releases.

## Features Demonstrated

- **Cargo caching** — Reuses `~/.cargo` and `target/` across runs
- **Matrix builds** — Cross-compile for Linux, macOS, Windows
- **Artifact upload** — Store binaries between stages
- **Conditional release** — Only on version tags

## Pipeline Flow

```
check (fmt, clippy)
       │
       ▼
     test
       │
       ▼
  build (matrix)
  ┌────┼────┐
  │    │    │
linux macos windows
  │    │    │
  └────┼────┘
       ▼
   release (on tag)
```

## Run Locally

```bash
oxide-ci run
```

## Typical Build Times

| Stage | Duration |
|-------|----------|
| check | ~30s |
| test | ~45s |
| build (cached) | ~60s |
| **Total** | **~2.5 min** |
