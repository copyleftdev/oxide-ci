# Caching

## Basic Usage

```yaml
steps:
  - name: cache-deps
    uses: cache@v1
    with:
      key: cargo-${{ hashFiles('Cargo.lock') }}
      paths:
        - ~/.cargo/registry
        - target
```

## Cache Keys

| Expression | Description |
|------------|-------------|
| `${{ hashFiles('...') }}` | Hash of file contents |
| `${{ runner.os }}` | Operating system |
| `${{ branch }}` | Current branch |

## Restore Keys

Fallback keys if exact match not found:

```yaml
with:
  key: npm-${{ hashFiles('package-lock.json') }}
  restore_keys:
    - npm-${{ branch }}-
    - npm-
```

## Cache Backends

### Local (Default)
```yaml
cache:
  backend: local
  path: /var/cache/oxide-ci
```

### S3
```yaml
cache:
  backend: s3
  s3:
    bucket: my-cache-bucket
    region: us-east-1
```

### Redis
```yaml
cache:
  backend: redis
  redis:
    url: redis://cache.example.com:6379
```
