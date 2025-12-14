---
trigger: glob
globs: ["spec/**/*.yaml", "spec/**/*.yml"]
---

# AsyncAPI Specification Rules

Rules for working with the AsyncAPI specification.

## Spec Structure

<structure>
- Main spec: `spec/asyncapi.yaml`
- Schemas: `spec/schemas/*.yaml`
- Channels: `spec/channels/*.yaml`
- Messages: `spec/messages/*.yaml`
- Operations: `spec/operations/*.yaml`
- Index files: `_index.yaml` in each directory
</structure>

## Adding New Schemas

<new_schema>
1. Create schema file in `spec/schemas/` (e.g., `feature.yaml`)
2. Define all types with proper YAML structure
3. Add references to `spec/schemas/_index.yaml`
4. Create corresponding channel in `spec/channels/`
5. Create corresponding message in `spec/messages/`
6. Update `spec/channels/_index.yaml` and `spec/messages/_index.yaml`
7. Run `make lint` to validate
</new_schema>

## Schema Conventions

<conventions>
- Use `type: object` with explicit `properties`
- Mark required fields in `required` array
- Use `$ref` for shared types (e.g., `$ref: './common.yaml#/Uuid'`)
- Add `description` to all properties
- Use `format` for special types (e.g., `format: uuid`, `format: date-time`)
- Use `enum` for fixed value sets
</conventions>

## Event Naming

<events>
- Channel pattern: `{domain}/{event}` (e.g., `run/started`)
- Message names: `PascalCase` (e.g., `RunStarted`)
- Payload names: `{Message}Payload` (e.g., `RunStartedPayload`)
- Use past tense for completed events: `started`, `completed`, `failed`
</events>

## Validation

<validation>
- Always run `make lint` after spec changes
- Ensure Rust types match spec using `oxide-spec` crate
- Check examples parse correctly
- Verify all `$ref` paths resolve
</validation>
