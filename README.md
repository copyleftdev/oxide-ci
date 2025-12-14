# Oxide CI

> API-first CI/CD engine written in Rust.

⚠️ **PRIVATE PRODUCT** - Do not commit to public repositories.

## Overview

Oxide CI is an event-driven CI/CD system with:
- **Rust core** — Performance, safety, zero-cost abstractions
- **WASM plugins** — Sandboxed, polyglot plugin system
- **Keygen licensing** — License management and entitlements
- **Stripe billing** — Subscriptions and metered usage

## Structure

```
oxide-ci/
├── spec/                    # AsyncAPI specification
│   ├── asyncapi.yaml        # Main entry point
│   ├── channels/            # Event channels
│   ├── messages/            # Message definitions
│   ├── schemas/             # Data schemas
│   │   ├── common.yaml      # Shared types (UUID, Timestamp, Status)
│   │   ├── pipeline.yaml    # Pipeline definition schema (user-authored)
│   │   ├── environment.yaml # Container, Firecracker, Nix environments
│   │   ├── run.yaml         # Run lifecycle payloads
│   │   ├── stage.yaml       # Stage lifecycle payloads
│   │   ├── step.yaml        # Step lifecycle payloads
│   │   ├── agent.yaml       # Agent pool management
│   │   ├── cache.yaml       # Build cache events
│   │   ├── secrets.yaml     # Secret management (Vault, AWS, GCP, Azure)
│   │   ├── auth.yaml        # OIDC token exchange
│   │   ├── matrix.yaml      # Matrix build expansion
│   │   ├── approval.yaml    # Approval gates & environment protection
│   │   ├── notification.yaml# Slack, Teams, PagerDuty, webhooks
│   │   ├── trace.yaml       # OpenTelemetry distributed tracing
│   │   ├── artifact.yaml    # Build artifacts
│   │   ├── webhook.yaml     # VCS webhook payloads
│   │   ├── licensing.yaml   # Keygen license events
│   │   └── billing.yaml     # Stripe billing events
│   └── operations/          # Operation definitions
├── examples/                # Example pipeline configurations
├── Makefile                 # Development commands
└── package.json             # Node dependencies (for AsyncAPI CLI)
```

## Development

```bash
# Install dependencies
make install

# Validate spec
make lint

# Bundle for distribution
make bundle

# Generate docs
make docs
```

## Monetization

### Keygen (Licensing)
- License validation on API access
- Machine fingerprinting for seat limits
- Entitlement-based feature flags
- Grace periods and suspension

### Stripe (Billing)
- Subscription plans (Starter, Pro, Enterprise)
- Metered billing for build minutes
- Payment failure → license suspension flow

## License

Proprietary
