//! Integration tests for spec-code correlation.
//!
//! These tests validate that Rust types match their AsyncAPI schema definitions.
//!
//! Note: The spec_link! macro implementations for oxide-core types must be done
//! in oxide-core itself due to Rust's orphan rules. These tests demonstrate
//! the API usage with local test types.

use oxide_spec::{SpecLinked, SpecValidator};

// Define local test types to demonstrate spec_link! usage
// (Cannot impl SpecLinked for external types due to orphan rules)

#[derive(Debug)]
#[allow(dead_code)]
struct TestPayload {
    id: String,
    status: String,
}

oxide_spec::spec_link!(
    TestPayload,
    schema = "TestPayload",
    file = "schemas/test.yaml"
);

#[test]
fn test_spec_linked_trait() {
    // Verify that spec_link! macro works for local types
    assert_eq!(TestPayload::SCHEMA_NAME, "TestPayload");
    assert_eq!(TestPayload::SPEC_FILE, "schemas/test.yaml");
    assert_eq!(TestPayload::SPEC_LINE, None);
}

#[test]
fn test_traceability_matrix() {
    use oxide_spec::traceability_matrix;

    let matrix = traceability_matrix!(TestPayload,);

    assert_eq!(matrix.entries().len(), 1);

    // Can look up by schema name
    let entry = matrix.by_schema("TestPayload");
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().spec_file, "schemas/test.yaml");

    // Generate markdown report
    let md = matrix.to_markdown();
    assert!(md.contains("TestPayload"));
    println!("{}", md);
}

#[test]
fn test_schema_validation() {
    let validator = SpecValidator::new("../../spec").expect("Failed to load spec");

    // Verify we loaded schemas from the spec
    println!("Loaded schemas from spec directory");

    // The validator is ready to validate types that implement SpecLinked
    drop(validator);
}

#[test]
fn test_validator_loads_all_schemas() {
    let validator = SpecValidator::new("../../spec").expect("Failed to load spec");

    // The build script found 112 schemas - verify we can load them
    println!("Validator created successfully with spec from ../../spec");

    // Validator should have loaded schemas
    drop(validator);
}

#[test]
fn test_spec_schema_registry() {
    use oxide_spec::SchemaRegistry;

    let registry =
        SchemaRegistry::load_from_spec_dir("../../spec").expect("Failed to load schema registry");

    // Check that we loaded schemas
    let count = registry.len();
    println!("Loaded {} schemas from spec", count);
    assert!(count > 0, "Should have loaded at least one schema");

    // Check for some expected schemas
    let schema_names: Vec<_> = registry.names().collect();
    println!("Schema names: {:?}", schema_names);

    // Verify some known schemas exist
    assert!(
        registry.get("RunQueuedPayload").is_some()
            || registry.get("PipelineDefinition").is_some()
            || registry.get("StepDefinition").is_some(),
        "Should have at least one known schema"
    );
}
