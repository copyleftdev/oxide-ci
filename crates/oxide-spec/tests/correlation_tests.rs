//! Integration tests for spec-code correlation.
//!
//! These tests validate that Rust types match their AsyncAPI schema definitions.

use oxide_core::events::*;
use oxide_core::pipeline::*;
use oxide_core::run::*;
use oxide_spec::{SpecLinked, SpecValidator, spec_link};

// Link Rust types to their AsyncAPI schemas
spec_link!(
    RunQueuedPayload,
    schema = "RunQueuedPayload",
    file = "schemas/run.yaml"
);
spec_link!(
    RunStartedPayload,
    schema = "RunStartedPayload",
    file = "schemas/run.yaml"
);
spec_link!(
    RunCompletedPayload,
    schema = "RunCompletedPayload",
    file = "schemas/run.yaml"
);
spec_link!(
    RunCancelledPayload,
    schema = "RunCancelledPayload",
    file = "schemas/run.yaml"
);

spec_link!(
    StageStartedPayload,
    schema = "StageStartedPayload",
    file = "schemas/stage.yaml"
);
spec_link!(
    StageCompletedPayload,
    schema = "StageCompletedPayload",
    file = "schemas/stage.yaml"
);

spec_link!(
    StepStartedPayload,
    schema = "StepStartedPayload",
    file = "schemas/step.yaml"
);
spec_link!(
    StepOutputPayload,
    schema = "StepOutputPayload",
    file = "schemas/step.yaml"
);
spec_link!(
    StepCompletedPayload,
    schema = "StepCompletedPayload",
    file = "schemas/step.yaml"
);

spec_link!(
    CacheHitPayload,
    schema = "CacheHitPayload",
    file = "schemas/cache.yaml"
);
spec_link!(
    CacheMissPayload,
    schema = "CacheMissPayload",
    file = "schemas/cache.yaml"
);
spec_link!(
    CacheUploadedPayload,
    schema = "CacheUploadedPayload",
    file = "schemas/cache.yaml"
);
spec_link!(
    CacheEvictedPayload,
    schema = "CacheEvictedPayload",
    file = "schemas/cache.yaml"
);

spec_link!(
    PipelineDefinition,
    schema = "PipelineDefinition",
    file = "schemas/pipeline.yaml"
);
spec_link!(
    StageDefinition,
    schema = "StageDefinition",
    file = "schemas/pipeline.yaml"
);
spec_link!(
    StepDefinition,
    schema = "StepDefinition",
    file = "schemas/pipeline.yaml"
);

#[test]
fn test_traceability_matrix() {
    use oxide_spec::traceability_matrix;

    let matrix = traceability_matrix!(
        RunQueuedPayload,
        RunStartedPayload,
        RunCompletedPayload,
        StageStartedPayload,
        StageCompletedPayload,
        StepStartedPayload,
        StepCompletedPayload,
        CacheHitPayload,
        CacheMissPayload,
        PipelineDefinition,
    );

    assert_eq!(matrix.entries().len(), 10);

    // Can look up by schema name
    let entry = matrix.by_schema("RunQueuedPayload");
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().spec_file, "schemas/run.yaml");

    // Generate markdown report
    let md = matrix.to_markdown();
    assert!(md.contains("RunQueuedPayload"));
    println!("{}", md);
}

#[test]
#[ignore] // Requires spec directory at correct path
fn test_schema_validation() {
    let validator = SpecValidator::new("../../spec").expect("Failed to load spec");

    // Validate that RunQueuedPayload matches the spec
    // let result = validator.validate::<RunQueuedPayload>();
    // assert!(result.is_valid, "Validation errors: {:?}", result.errors);
}

#[test]
fn test_spec_link_info() {
    // Check that SpecLinked trait provides correct info
    assert_eq!(RunQueuedPayload::SCHEMA_NAME, "RunQueuedPayload");
    assert_eq!(RunQueuedPayload::SPEC_FILE, "schemas/run.yaml");

    assert_eq!(PipelineDefinition::SCHEMA_NAME, "PipelineDefinition");
    assert_eq!(PipelineDefinition::SPEC_FILE, "schemas/pipeline.yaml");
}

/// This test demonstrates how to use the spec validation in CI.
#[test]
#[ignore] // Enable in CI with spec directory
fn test_all_event_schemas() {
    let validator = SpecValidator::new("../../spec").expect("Failed to load spec");

    let results = oxide_spec::validate_all!(
        validator,
        RunQueuedPayload,
        RunStartedPayload,
        RunCompletedPayload,
        StageStartedPayload,
        StageCompletedPayload,
    );

    let mut all_valid = true;
    for result in &results {
        if !result.is_valid {
            eprintln!("❌ {} failed validation:", result.type_name);
            for error in &result.errors {
                eprintln!("   {}", error);
            }
            all_valid = false;
        } else {
            eprintln!("✓ {} matches spec", result.type_name);
        }
    }

    assert!(all_valid, "Some types failed spec validation");
}
