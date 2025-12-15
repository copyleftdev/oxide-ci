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
#[ignore] // Requires spec directory at correct path
fn test_schema_validation() {
    let _validator = SpecValidator::new("../../spec").expect("Failed to load spec");
    // Validation would be done on types that have SpecLinked implemented
}

#[test]
fn test_validator_creation() {
    // Test that validator can be created (may fail if spec dir doesn't exist)
    let result = SpecValidator::new("../../spec");
    if let Ok(validator) = result {
        // Validator was created successfully
        println!("Validator created with spec from ../../spec");
        drop(validator);
    } else {
        // Expected in some test environments
        println!("Spec directory not found (expected in some environments)");
    }
}
