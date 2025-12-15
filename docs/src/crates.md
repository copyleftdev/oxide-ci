# Crate Overview

## Core

| Crate | Description |
|-------|-------------|
| `oxide-core` | Domain types, IDs, events, traits |
| `oxide-spec` | AsyncAPI spec validation and correlation |

## Infrastructure

| Crate | Description |
|-------|-------------|
| `oxide-api` | Axum HTTP/WebSocket server |
| `oxide-nats` | NATS JetStream event bus |
| `oxide-db` | SQLx PostgreSQL layer |
| `oxide-cache` | Distributed caching (S3, Redis) |

## Execution

| Crate | Description |
|-------|-------------|
| `oxide-scheduler` | Pipeline scheduling |
| `oxide-agent` | Build agent |
| `oxide-runner` | Step execution engine |
| `oxide-plugins` | WASM plugin host (Extism) |

## Integrations

| Crate | Description |
|-------|-------------|
| `oxide-auth` | OIDC token exchange (AWS, GCP, Azure) |
| `oxide-secrets` | Secret providers (Vault, AWS SM) |
| `oxide-notify` | Notifications (Slack, Discord, PagerDuty) |
| `oxide-billing` | Stripe metered billing |
| `oxide-licensing` | Keygen license validation |

## Observability

| Crate | Description |
|-------|-------------|
| `oxide-trace` | OpenTelemetry distributed tracing |

## Generate Docs

```bash
cargo doc --workspace --no-deps --open
```
