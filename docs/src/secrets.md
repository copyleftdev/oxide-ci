# Secrets Management

## Defining Secrets

In pipeline:
```yaml
secrets:
  - name: API_KEY
    required: true
  - name: DEPLOY_TOKEN
    required: false
```

## Using Secrets

```yaml
steps:
  - name: deploy
    run: ./deploy.sh
    env:
      API_KEY: ${{ secrets.API_KEY }}
```

## Secret Providers

### Environment Variables
```yaml
secrets:
  provider: env
```

### HashiCorp Vault
```yaml
secrets:
  provider: vault
  vault:
    address: https://vault.example.com
    path: secret/data/ci
```

### AWS Secrets Manager
```yaml
secrets:
  provider: aws
  aws:
    region: us-east-1
    secret_id: my-app/prod
```

## OIDC Token Exchange

Keyless authentication to cloud providers:

```yaml
steps:
  - name: deploy-aws
    uses: aws-auth@v1
    with:
      role_arn: arn:aws:iam::123456789:role/deploy
```

The CI generates a signed JWT that AWS STS exchanges for temporary credentials.
