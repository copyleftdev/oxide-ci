# Validate Spec-Code Correlation

Workflow for ensuring Rust types match the AsyncAPI specification.

## Steps

1. **Review which types need validation**
   - Check `oxide-core/src/events.rs` for all event payloads
   - Check `oxide-core/src/pipeline.rs` for pipeline types

2. **Ensure spec_link macros are in place**
   ```rust
   // In tests or a dedicated file
   spec_link!(RunQueuedPayload, schema = "RunQueuedPayload", file = "schemas/run.yaml");
   ```

3. **Build the traceability matrix**
   ```rust
   let matrix = traceability_matrix!(
       RunQueuedPayload,
       RunStartedPayload,
       RunCompletedPayload,
       // ... all types
   );
   println!("{}", matrix.to_markdown());
   ```

4. **Run spec validation tests**
   ```bash
   cargo test -p oxide-spec
   ```

5. **Check for unimplemented schemas**
   - Compare spec schemas vs Rust types
   - Note any schemas without Rust implementations

6. **Validate serialization roundtrips**
   ```rust
   #[test]
   fn test_serialization_roundtrip() {
       let original = RunQueuedPayload { /* ... */ };
       let json = serde_json::to_string(&original).unwrap();
       let parsed: RunQueuedPayload = serde_json::from_str(&json).unwrap();
       assert_eq!(original, parsed);
   }
   ```

7. **Generate correlation report**
   - Save traceability matrix to `docs/TRACEABILITY.md`
   - Note any discrepancies

8. **Fix any mismatches**
   - Update Rust types to match spec, OR
   - Update spec to match intended Rust types
   - Run `make lint` after spec changes
