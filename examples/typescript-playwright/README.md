# TypeScript + Playwright Example

Real E2E testing pipeline with TypeScript and Playwright.

## Stack

- **TypeScript** — Type-safe JavaScript
- **Playwright** — Cross-browser E2E testing
- **Node.js** — Runtime

## Pipeline Stages

1. **install** — Install npm dependencies and Playwright browsers
2. **quality** — TypeScript type checking
3. **test** — Run Playwright tests against example.com

## Run Locally

```bash
# From this directory
npm install
npx playwright install chromium
npx playwright test

# Or via Oxide CI
oxide run
```

## Files

```
typescript-playwright/
├── .oxide-ci/
│   └── pipeline.yaml     # CI pipeline
├── src/
│   └── utils.ts          # Source code
├── tests/
│   └── example.spec.ts   # Playwright tests
├── package.json
├── tsconfig.json
└── playwright.config.ts
```

## Test Output

```
Running 3 tests using 1 worker

  ✓ Example Tests › has title (1.2s)
  ✓ Example Tests › contains example heading (0.8s)
  ✓ Example Tests › has more information link (0.6s)

  3 passed (3.5s)
```
