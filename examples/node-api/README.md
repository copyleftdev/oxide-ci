# Node.js REST API Example

Express/Fastify API with TypeScript, Docker builds, and ECS deployment.

## Features Demonstrated

- **npm caching** — Speeds up `npm ci` with cached `node_modules`
- **Docker multi-stage** — Optimized production images
- **Container registry** — Push to GitHub Container Registry
- **ECS deployment** — Deploy to AWS ECS on main branch

## Pipeline Flow

```
install (npm ci)
       │
       ▼
quality (lint, typecheck, test)
       │
       ▼
 build (docker)
       │
       ▼
  push (ghcr.io)  ─── only on main
       │
       ▼
deploy-staging
```

## Secrets Required

| Secret | Purpose |
|--------|---------|
| `GHCR_USER` | GitHub username |
| `GHCR_TOKEN` | GitHub PAT with `packages:write` |

## Run Locally

```bash
oxide-ci run --stage quality  # Just run tests
oxide-ci run                   # Full pipeline (requires Docker)
```
