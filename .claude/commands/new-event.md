---
description: Add a new AsyncAPI event end to end — schema, channel, message, Rust type, spec_link, tests
argument-hint: "[domain/event, e.g. run/cancelled]"
disable-model-invocation: true
---

Add the event `$ARGUMENTS` across spec and code. Both sides land in the same change; a spec without a type (or a type without a schema) is not acceptable.

Existing schemas: !`ls spec/schemas/`

## Steps

1. **Schema** — `spec/schemas/<domain>.yaml`: `{Message}Payload` with explicit `properties`, a `required` array, `description` on every property, `format: uuid` / `format: date-time` where they apply, `$ref: './common.yaml#/...'` for shared types. Register it in `spec/schemas/_index.yaml`.
2. **Channel** — `spec/channels/`, address `{domain}/{event}`, past-tense verb. Register in `spec/channels/_index.yaml`.
3. **Message** — `spec/messages/`, `PascalCase` name referencing the payload schema. Register in `spec/messages/_index.yaml`.
4. **Operation** — add to `spec/operations/` if something sends or receives it.
5. **Validate** — `make lint`. Fix every `$ref` the validator can't resolve.
6. **Rust type** — in `crates/oxide-core/src/events.rs`: derive `Debug, Clone, Serialize, Deserialize`, `#[serde(rename_all = "snake_case")]`, newtype IDs, `Uuid`/`DateTime<Utc>` matching the schema formats, `///` docs.
7. **Correlate** — `oxide_spec::spec_link!(...)` plus a test in `crates/oxide-spec/tests/correlation_tests.rs` asserting the type validates against the schema.
8. **Emit/handle** — wire the event where it belongs (`oxide-scheduler`, `oxide-runner`, `oxide-nats` subject mapping) if the ticket calls for it.
9. **Gate** — `./scripts/verify.sh --spec`, then have `spec-correlator` confirm no drift.
