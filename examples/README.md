# Oxide CI Examples

Real-world pipeline examples demonstrating Oxide CI across languages, frameworks, and deployment targets.

## Quick Start

```bash
# Run any example locally
cd examples/rust-cli
oxide-ci run
```

## Examples by Language

| Example | Stack | Features Demonstrated |
|---------|-------|----------------------|
| [rust-cli](./rust-cli/) | Rust CLI tool | Build, test, release binaries, caching |
| [node-api](./node-api/) | Node.js REST API | npm, Docker build, multi-stage |
| [python-ml](./python-ml/) | Python ML pipeline | Nix environment, GPU, artifacts |
| [go-microservice](./go-microservice/) | Go gRPC service | Matrix builds, container registry |

## Examples by Use Case

| Example | Scenario | Features Demonstrated |
|---------|----------|----------------------|
| [monorepo](./monorepo/) | Multi-project repo | Path triggers, parallel stages |
| [deploy-aws](./deploy-aws/) | AWS deployment | OIDC auth, ECS/Lambda deploy |
| [deploy-k8s](./deploy-k8s/) | Kubernetes | Helm, ArgoCD, multi-cluster |
| [nix-devshell](./nix-devshell/) | Reproducible builds | Flakes, pure mode, caching |

## Pipeline Patterns

### Basic Build & Test
```yaml
stages:
  - name: build
    steps:
      - run: make build
      - run: make test
```

### Docker Build & Push
```yaml
stages:
  - name: docker
    steps:
      - uses: docker-build@v1
        with:
          tags: ghcr.io/org/app:${{ sha }}
      - uses: docker-push@v1
```

### Deploy with Approval
```yaml
stages:
  - name: deploy-staging
    steps:
      - uses: deploy@v1
        with:
          environment: staging
  
  - name: deploy-prod
    needs: [deploy-staging]
    approval:
      required: true
      approvers: [platform-team]
    steps:
      - uses: deploy@v1
        with:
          environment: production
```

## Contributing Examples

1. Create a new directory under `examples/`
2. Add a `.oxide-ci/pipeline.yaml`
3. Include a `README.md` explaining the use case
4. Submit a PR!
