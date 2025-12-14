# Add New Event

Workflow for adding a new event to the Oxide CI system.

## Steps

1. **Define the event schema** in `spec/schemas/{domain}.yaml`
   ```yaml
   NewEventPayload:
     type: object
     description: Payload for the new event
     properties:
       id:
         $ref: './common.yaml#/Uuid'
       timestamp:
         $ref: './common.yaml#/Timestamp'
       # Add event-specific properties
     required:
       - id
       - timestamp
   ```

2. **Add to schema index** `spec/schemas/_index.yaml`
   ```yaml
   NewEventPayload:
     $ref: './{domain}.yaml#/NewEventPayload'
   ```

3. **Create channel** in `spec/channels/{domain}.yaml`
   ```yaml
   new-event:
     address: '{domain}/new-event/{id}'
     messages:
       NewEvent:
         $ref: '../messages/{domain}.yaml#/NewEvent'
   ```

4. **Add to channel index** `spec/channels/_index.yaml`

5. **Create message** in `spec/messages/{domain}.yaml`
   ```yaml
   NewEvent:
     name: NewEvent
     payload:
       $ref: '../schemas/{domain}.yaml#/NewEventPayload'
   ```

6. **Add to message index** `spec/messages/_index.yaml`

7. **Validate spec**
   ```bash
   make lint
   ```

8. **Add Rust type** in `oxide-core/src/events.rs`
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct NewEventPayload {
       pub id: Uuid,
       pub timestamp: DateTime<Utc>,
       // ... fields matching spec
   }
   ```

9. **Add to Event enum** in `oxide-core/src/events.rs`
   ```rust
   pub enum Event {
       // ... existing variants
       NewEvent(NewEventPayload),
   }
   ```

10. **Add subject pattern** in `Event::subject()` method

11. **Add spec link** for correlation
    ```rust
    spec_link!(NewEventPayload, schema = "NewEventPayload", file = "schemas/{domain}.yaml");
    ```

12. **Write tests**
    - Serialization roundtrip test
    - Spec validation test

13. **Commit**
    ```bash
    git add spec/ crates/oxide-core/
    git commit -m "feat(spec): add NewEvent to {domain} domain"
    ```
