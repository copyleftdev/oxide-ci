# Crate Ecosystem

Oxide CI is a modular workspace consisting of specific-purpose crates.

## Core Services
| Crate | Description |
|-------|-------------|
| **`oxide-core`** | Defines shared types (`Pipeline`, `Run`), traits (`EventBus`), and interpolation logic. The foundation of the system. |
| **`oxide-api`** | The API Server (REST/gRPC) handling requests from the CLI and Web UI. |
| **`oxide-scheduler`** | Orchestrates pipeline execution, resolves DAG dependencies, and assigns jobs to agents. |
| **`oxide-agent`** | The worker process that connects to the fleet and executes assigned jobs. |
| **`oxide-runner`** | The step execution engine. Handles Docker containers, plugins, and process isolation. |

## Infrastructure & Support
| Crate | Description |
|-------|-------------|
| **`oxide-cli`** | The command-line interface tool (`oxide`). |
| **`oxide-plugins`** | Implements the plugin system, hosting both Native (`git`, `cache`) and WASM plugins. |
| **`oxide-db`** | Database abstraction layer (PostgreSQL) and migrations. |
| **`oxide-nats`** | NATS-based event bus implementation for messaging. |
| **`oxide-auth`** | Authentication and authorization logic. |
| **`oxide-licensing`** | License verification and management. |
| **`oxide-spec`** | Formal specifications and schemas. |

## Feature Modules
| Crate | Description |
|-------|-------------|
| **`oxide-secrets`** | Secure vault for managing pipeline secrets and credentials. |
| **`oxide-cache`** | Manages build caches and artifact storage (S3/MinIO). |
| **`oxide-billing`** | Usage metering and billing verification (GitHub Actions integration). |
| **`oxide-trace`** | OpenTelemetry integration for tracing and observability. |
| **`oxide-notify`** | Notification delivery system (Email, Slack, Webhooks). |

## Testing
| Crate | Description |
|-------|-------------|
| **`oxide-tests`** | shared integration test suite and fixtures. |
