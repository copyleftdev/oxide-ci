# Go Microservice Example

gRPC microservice with protobuf, multi-arch builds, and Cloud Run deployment.

## Features Demonstrated

- **Go module caching** — Fast dependency resolution
- **Protobuf generation** — `go generate` for gRPC stubs
- **Multi-arch builds** — amd64 + arm64 Docker images
- **OIDC to GCP** — Keyless authentication via Workload Identity
- **Cloud Run deploy** — Serverless container deployment

## Pipeline Flow

```
setup (deps, protoc)
       │
       ▼
quality (lint, test)
       │
       ▼
  build (matrix)
  ┌─────┴─────┐
amd64       arm64
  └─────┬─────┘
        ▼
push (manifest)
        │
        ▼
deploy (Cloud Run)
```

## OIDC Setup

```bash
# Create Workload Identity Pool
gcloud iam workload-identity-pools create ci \
  --location=global

# Create Provider for Oxide CI
gcloud iam workload-identity-pools providers create-oidc oxide \
  --location=global \
  --workload-identity-pool=ci \
  --issuer-uri=https://oxideci.dev \
  --attribute-mapping="google.subject=assertion.sub"
```

## Run Locally

```bash
oxide-ci run --stage quality  # Lint and test only
```
