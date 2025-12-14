# Add AsyncAPI Schema

Workflow for adding a new schema to the AsyncAPI specification.

## Steps

1. **Define the schema** in `spec/schemas/{name}.yaml`
   - Use proper YAML structure with type, properties, required
   - Add descriptions to all fields
   - Reference common types: `$ref: './common.yaml#/Uuid'`

2. **Update schema index** in `spec/schemas/_index.yaml`
   - Add references to all new types in the schema file

3. **Create channel definitions** in `spec/channels/{name}.yaml`
   - Define channels for each event
   - Reference the messages

4. **Update channel index** in `spec/channels/_index.yaml`
   - Add references to all new channels

5. **Create message definitions** in `spec/messages/{name}.yaml`
   - Define message wrappers for each event
   - Reference the payload schemas

6. **Update message index** in `spec/messages/_index.yaml`
   - Add references to all new messages

7. **Validate the spec**
   ```bash
   make lint
   ```
   - Fix any validation errors

8. **Create corresponding Rust types** in `oxide-core`
   - Add types matching the schema
   - Add `spec_link!` macro
   - Add to `oxide-core/src/events.rs` if events

9. **Update documentation**
   - Update README.md if structure changed
   - Add examples if appropriate

10. **Commit with spec label**
    ```bash
    git add spec/
    git commit -m "feat(spec): add {name} schema and events"
    ```
