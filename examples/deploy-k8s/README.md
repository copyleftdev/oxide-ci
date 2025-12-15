# Kubernetes Deployment Example

Helm-based deployment with GitOps and ArgoCD for production.

## Features Demonstrated

- **Helm charts** — Templated Kubernetes manifests
- **Multi-cluster** — Dev → Staging → Production
- **OIDC to GKE** — Keyless cluster authentication
- **GitOps** — ArgoCD sync for production
- **Rollout verification** — Wait for healthy pods

## Pipeline Flow

```
build (Docker)
       │
       ▼
helm-lint (validate)
       │
       ▼
deploy-dev (helm upgrade)
       │
       ▼
deploy-staging (3 replicas)
       │
       ▼
   [approval]
       │
       ▼
deploy-production (GitOps → ArgoCD)
```

## Cluster Setup

| Cluster | Purpose | Replicas |
|---------|---------|----------|
| dev-cluster | Development | 1 |
| staging-cluster | Pre-production | 3 |
| prod-cluster | Production (ArgoCD) | 5 |

## GitOps Flow

Production uses GitOps pattern:

1. Pipeline updates `gitops-config` repo
2. ArgoCD detects change
3. ArgoCD syncs to production cluster
4. Pipeline waits for sync completion

## Helm Values

```yaml
# helm/myapp/values.yaml
image:
  repository: ghcr.io/myorg/myapp
  tag: latest
  
replicas: 1

resources:
  requests:
    memory: 256Mi
    cpu: 100m
  limits:
    memory: 512Mi
    cpu: 500m
```

## Run Locally

```bash
oxide-ci run --stage helm-lint  # Validate only
```
