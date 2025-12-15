# Monorepo Example

pnpm workspace monorepo with smart change detection and parallel builds.

## Features Demonstrated

- **Change detection** — Only build affected packages
- **Dependency awareness** — Rebuild dependents when shared changes
- **Parallel stages** — API and Web build concurrently
- **Workspace caching** — pnpm store shared across runs
- **E2E testing** — Full integration tests after builds

## Structure

```
packages/
├── shared/     # Common utilities
├── api/        # Node.js backend
├── web/        # React frontend
└── e2e/        # Playwright tests
```

## Pipeline Flow

```
        detect (git diff)
              │
      ┌───────┴───────┐
      ▼               ▼
   shared ────────────┤
      │               │
      ▼               ▼
     api             web
      │               │
      └───────┬───────┘
              ▼
             e2e
```

## Change Scenarios

| Changed | Runs |
|---------|------|
| `packages/api/` | api, e2e |
| `packages/web/` | web, e2e |
| `packages/shared/` | shared, api, web, e2e |
| `packages/e2e/` | e2e only |

## Run Locally

```bash
oxide-ci run                    # Full pipeline
oxide-ci run --stage api        # Just API
```
