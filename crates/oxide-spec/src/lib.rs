//! Spec-Code Correlation for Oxide CI
//!
//! This crate provides tooling to ensure that Rust types match the AsyncAPI specification.
//! It enables:
//!
//! 1. **Schema Validation**: Validate that Rust types match AsyncAPI schemas
//! 2. **Traceability**: Link code to spec sections
//! 3. **Contract Testing**: Test that serialized data matches spec examples
//!
//! ## Usage
//!
//! ```rust,ignore
//! use oxide_spec::{validate_against_spec, SpecRef};
//!
//! #[derive(SpecRef)]
//! #[spec(schema = "RunQueuedPayload", file = "schemas/run.yaml")]
//! pub struct RunQueuedPayload { /* ... */ }
//!
//! // At test time, validate the type matches the spec
//! validate_against_spec::<RunQueuedPayload>()?;
//! ```

pub mod schema;
pub mod traceability;
pub mod validation;

pub use schema::*;
pub use traceability::*;
pub use validation::*;

/// Marker trait for types that have a corresponding AsyncAPI schema.
pub trait SpecLinked {
    /// The schema name in the AsyncAPI spec.
    const SCHEMA_NAME: &'static str;
    
    /// The file path relative to spec/ directory.
    const SPEC_FILE: &'static str;
    
    /// Optional: line number in the spec file.
    const SPEC_LINE: Option<u32> = None;
}

/// Macro to link a Rust type to its AsyncAPI schema.
///
/// This creates compile-time documentation and enables runtime validation.
#[macro_export]
macro_rules! spec_link {
    ($type:ty, schema = $schema:literal, file = $file:literal) => {
        impl $crate::SpecLinked for $type {
            const SCHEMA_NAME: &'static str = $schema;
            const SPEC_FILE: &'static str = $file;
        }
    };
    ($type:ty, schema = $schema:literal, file = $file:literal, line = $line:literal) => {
        impl $crate::SpecLinked for $type {
            const SCHEMA_NAME: &'static str = $schema;
            const SPEC_FILE: &'static str = $file;
            const SPEC_LINE: Option<u32> = Some($line);
        }
    };
}

/// Result of spec validation.
#[derive(Debug)]
pub struct SpecValidationResult {
    pub type_name: String,
    pub schema_name: String,
    pub spec_file: String,
    pub is_valid: bool,
    pub errors: Vec<SpecValidationError>,
    pub warnings: Vec<String>,
}

/// A spec validation error.
#[derive(Debug)]
pub struct SpecValidationError {
    pub path: String,
    pub message: String,
    pub spec_expected: Option<String>,
    pub rust_actual: Option<String>,
}

impl std::fmt::Display for SpecValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at {}: {}", self.path, self.message)?;
        if let Some(expected) = &self.spec_expected {
            write!(f, "\n  spec expected: {}", expected)?;
        }
        if let Some(actual) = &self.rust_actual {
            write!(f, "\n  rust actual: {}", actual)?;
        }
        Ok(())
    }
}
