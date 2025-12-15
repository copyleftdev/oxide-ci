# Architecture

## Overview

Oxide CI uses a **hexagonal (ports & adapters) architecture** with event-driven communication.

```
┌─────────────────────────────────────────────────────┐
│                    oxide-api                         │
│              (HTTP/WebSocket Server)                 │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│                   oxide-nats                         │
│                  (Event Bus)                         │
└───┬─────────┬─────────┬─────────┬─────────┬────────┘
    │         │         │         │         │
┌───▼───┐ ┌───▼───┐ ┌───▼───┐ ┌───▼───┐ ┌───▼───┐
│ sched │ │ agent │ │runner │ │notify │ │billing│
└───────┘ └───────┘ └───────┘ └───────┘ └───────┘
```

## Core Components

| Crate | Purpose |
|-------|---------|
| `oxide-core` | Domain types, traits, zero external deps |
| `oxide-api` | HTTP/WebSocket API server |
| `oxide-scheduler` | Pipeline scheduling and orchestration |
| `oxide-agent` | Build agent lifecycle |
| `oxide-runner` | Step execution (container, nix, firecracker) |
| `oxide-nats` | NATS JetStream event bus |
| `oxide-db` | PostgreSQL persistence |

## Event Flow

1. **Trigger** → API receives webhook/schedule
2. **Schedule** → Scheduler queues run, publishes `run.queued`
3. **Dispatch** → Agent claims run, publishes `run.started`
4. **Execute** → Runner executes steps, streams logs
5. **Complete** → Agent publishes `run.completed`
6. **Notify** → Notification service sends alerts

## AsyncAPI Spec

All events follow schemas in `spec/`:
- `spec/asyncapi.yaml` - Main spec
- `spec/schemas/*.yaml` - Type definitions
- `spec/channels/*.yaml` - Event channels
