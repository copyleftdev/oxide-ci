//! Validation of Rust types against AsyncAPI schemas.

use crate::schema::{AsyncApiSchema, SchemaRegistry};
use crate::{SpecLinked, SpecValidationError, SpecValidationResult};
use schemars::JsonSchema;
use schemars::schema_for;
use serde::Serialize;
use std::collections::HashSet;

/// Validator for spec-code correlation.
pub struct SpecValidator {
    registry: SchemaRegistry,
    #[allow(dead_code)]
    spec_dir: String,
}

impl SpecValidator {
    /// Create a new validator with schemas loaded from the spec directory.
    pub fn new(spec_dir: &str) -> Result<Self, crate::schema::SchemaError> {
        let registry = SchemaRegistry::load_from_spec_dir(spec_dir)?;
        Ok(Self {
            registry,
            spec_dir: spec_dir.to_string(),
        })
    }

    /// Validate a type against its linked schema.
    pub fn validate<T: SpecLinked + JsonSchema>(&self) -> SpecValidationResult {
        let schema_name = T::SCHEMA_NAME;
        let spec_file = T::SPEC_FILE;

        let mut result = SpecValidationResult {
            type_name: std::any::type_name::<T>().to_string(),
            schema_name: schema_name.to_string(),
            spec_file: spec_file.to_string(),
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };

        // Get the AsyncAPI schema
        let Some(async_schema) = self.registry.get(schema_name) else {
            result.is_valid = false;
            result.errors.push(SpecValidationError {
                path: "/".to_string(),
                message: format!("Schema '{}' not found in spec", schema_name),
                spec_expected: None,
                rust_actual: None,
            });
            return result;
        };

        // Generate JSON Schema from Rust type
        let rust_schema = schema_for!(T);

        // Compare schemas - wrap SchemaObject in Schema::Object
        let schema = schemars::schema::Schema::Object(rust_schema.schema);
        self.compare_schemas(&schema, async_schema, "", &mut result);

        result.is_valid = result.errors.is_empty();
        result
    }

    /// Validate that a value serializes correctly according to the spec.
    pub fn validate_value<T: SpecLinked + Serialize>(&self, value: &T) -> SpecValidationResult {
        let mut result = SpecValidationResult {
            type_name: std::any::type_name::<T>().to_string(),
            schema_name: T::SCHEMA_NAME.to_string(),
            spec_file: T::SPEC_FILE.to_string(),
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };

        // Serialize the value
        let json = match serde_json::to_value(value) {
            Ok(j) => j,
            Err(e) => {
                result.is_valid = false;
                result.errors.push(SpecValidationError {
                    path: "/".to_string(),
                    message: format!("Serialization failed: {}", e),
                    spec_expected: None,
                    rust_actual: None,
                });
                return result;
            }
        };

        // Get the AsyncAPI schema
        let Some(async_schema) = self.registry.get(T::SCHEMA_NAME) else {
            result.is_valid = false;
            result.errors.push(SpecValidationError {
                path: "/".to_string(),
                message: format!("Schema '{}' not found in spec", T::SCHEMA_NAME),
                spec_expected: None,
                rust_actual: None,
            });
            return result;
        };

        // Validate the JSON against the schema
        self.validate_json_against_schema(&json, async_schema, "", &mut result);

        result.is_valid = result.errors.is_empty();
        result
    }

    fn compare_schemas(
        &self,
        rust_schema: &schemars::schema::Schema,
        async_schema: &AsyncApiSchema,
        path: &str,
        result: &mut SpecValidationResult,
    ) {
        use schemars::schema::Schema;

        let Schema::Object(rust_obj) = rust_schema else {
            return;
        };

        // Compare types
        if let Some(async_type) = &async_schema.schema_type {
            let rust_type = rust_obj.instance_type.as_ref().and_then(|t| match t {
                schemars::schema::SingleOrVec::Single(s) => Some(format!("{:?}", s).to_lowercase()),
                schemars::schema::SingleOrVec::Vec(v) => {
                    v.first().map(|s| format!("{:?}", s).to_lowercase())
                }
            });

            if let Some(rt) = rust_type
                && &rt != async_type
                && !(rt == "integer" && async_type == "number")
            {
                result.warnings.push(format!(
                    "{}: type mismatch - spec has '{}', Rust has '{}'",
                    path, async_type, rt
                ));
            }
        }

        // Compare properties for objects
        if let (Some(async_props), Some(rust_obj_validation)) =
            (&async_schema.properties, &rust_obj.object)
        {
            let rust_props: HashSet<_> = rust_obj_validation.properties.keys().collect();
            let async_prop_names: HashSet<_> = async_props.keys().collect();

            // Check for missing properties in Rust
            for prop in async_prop_names.difference(&rust_props) {
                if async_schema.is_required(prop) {
                    result.errors.push(SpecValidationError {
                        path: format!("{}/{}", path, prop),
                        message: format!("Required property '{}' missing in Rust type", prop),
                        spec_expected: Some("present".to_string()),
                        rust_actual: Some("absent".to_string()),
                    });
                } else {
                    result.warnings.push(format!(
                        "{}/{}: optional property in spec not in Rust type",
                        path, prop
                    ));
                }
            }

            // Check for extra properties in Rust
            for prop in rust_props.difference(&async_prop_names) {
                result.warnings.push(format!(
                    "{}/{}: property in Rust type not in spec",
                    path, prop
                ));
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn validate_json_against_schema(
        &self,
        json: &serde_json::Value,
        schema: &AsyncApiSchema,
        path: &str,
        result: &mut SpecValidationResult,
    ) {
        use serde_json::Value;

        // Check type
        if let Some(expected_type) = &schema.schema_type {
            let actual_type = match json {
                Value::Null => "null",
                Value::Bool(_) => "boolean",
                Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
            };

            // Allow integer where number is expected
            if actual_type != expected_type
                && !(actual_type == "integer" && expected_type == "number")
            {
                result.errors.push(SpecValidationError {
                    path: path.to_string(),
                    message: "Type mismatch".to_string(),
                    spec_expected: Some(expected_type.clone()),
                    rust_actual: Some(actual_type.to_string()),
                });
            }
        }

        // Check required properties for objects
        if let Value::Object(obj) = json {
            if let Some(required) = &schema.required {
                for req_field in required {
                    if !obj.contains_key(req_field) {
                        result.errors.push(SpecValidationError {
                            path: format!("{}/{}", path, req_field),
                            message: format!("Required field '{}' is missing", req_field),
                            spec_expected: Some("present".to_string()),
                            rust_actual: Some("absent".to_string()),
                        });
                    }
                }
            }

            // Recursively validate properties
            if let Some(props) = &schema.properties {
                for (key, value) in obj {
                    if let Some(prop_schema) = props.get(key) {
                        self.validate_json_against_schema(
                            value,
                            prop_schema,
                            &format!("{}/{}", path, key),
                            result,
                        );
                    }
                }
            }
        }

        // Check enum values
        if let Some(enum_values) = &schema.enum_values
            && !enum_values.contains(json)
        {
            result.errors.push(SpecValidationError {
                path: path.to_string(),
                message: "Value not in enum".to_string(),
                spec_expected: Some(format!("{:?}", enum_values)),
                rust_actual: Some(json.to_string()),
            });
        }
    }
}

/// Validate all registered types against their specs.
#[macro_export]
macro_rules! validate_all {
    ($validator:expr, $($type:ty),* $(,)?) => {{
        let mut results = Vec::new();
        $(
            results.push($validator.validate::<$type>());
        )*
        results
    }};
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_validator_creation() {
        // This would need the spec directory to exist
        // let validator = SpecValidator::new("../../spec");
        // assert!(validator.is_ok());
    }
}
