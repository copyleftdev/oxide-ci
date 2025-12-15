# AWS Deployment Example

Multi-service AWS deployment with OIDC authentication and staged rollout.

## Features Demonstrated

- **OIDC keyless auth** — No AWS credentials stored
- **ECR push** — Container registry integration
- **ECS deploy** — Fargate service updates
- **Lambda deploy** — Serverless function updates
- **S3 + CloudFront** — Static site deployment
- **Approval gates** — Human review for production

## Pipeline Flow

```
authenticate (OIDC → STS)
       │
       ├─────────┬─────────────┐
       ▼         ▼             ▼
    build   deploy-lambda  deploy-static
       │
       ▼
deploy-staging
       │
       ▼
   [approval]
       │
       ▼
deploy-production
```

## OIDC Setup

```bash
# Create OIDC provider in AWS
aws iam create-open-id-connect-provider \
  --url https://oxideci.dev \
  --client-id-list oxide-ci \
  --thumbprint-list <THUMBPRINT>

# Create role with trust policy
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": {"Federated": "arn:aws:iam::123456789:oidc-provider/oxideci.dev"},
    "Action": "sts:AssumeRoleWithWebIdentity",
    "Condition": {
      "StringEquals": {
        "oxideci.dev:sub": "org:myorg:pipeline:deploy-aws"
      }
    }
  }]
}
```

## Secrets Required

| Secret | Purpose |
|--------|---------|
| `SLACK_WEBHOOK` | Deployment notifications |

**Note**: No AWS credentials needed — OIDC handles authentication!

## Run Locally

```bash
oxide-ci run --stage build  # Build only (no deploy)
```
