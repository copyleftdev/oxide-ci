# Getting Started

## Installation

```bash
cargo install oxide-ci
```

## Quick Start

1. Create a `.oxide-ci/pipeline.yaml` in your repository:

```yaml
name: my-pipeline
version: "1.0"

stages:
  - name: build
    steps:
      - name: checkout
        uses: git-checkout@v1
      - name: compile
        run: cargo build --release

  - name: test
    steps:
      - name: unit-tests
        run: cargo test
```

2. Run locally:

```bash
oxide-ci run
```

## Configuration

Set environment variables or use a config file:

| Variable | Description | Default |
|----------|-------------|---------|
| `OXIDE_API_URL` | API server URL | `http://localhost:8080` |
| `OXIDE_TOKEN` | Authentication token | - |
| `OXIDE_LOG_LEVEL` | Log level | `info` |

## Next Steps

- [Pipeline Reference](./pipeline-reference.md) - Full YAML syntax
- [Environments](./environments.md) - Container, Nix, Firecracker
- [Secrets](./secrets.md) - Managing sensitive data
