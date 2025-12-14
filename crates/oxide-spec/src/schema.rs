//! Schema parsing and representation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed AsyncAPI schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncApiSchema {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub format: Option<String>,
    pub description: Option<String>,
    pub properties: Option<HashMap<String, AsyncApiSchema>>,
    pub required: Option<Vec<String>>,
    pub items: Option<Box<AsyncApiSchema>>,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
    #[serde(rename = "oneOf")]
    pub one_of: Option<Vec<AsyncApiSchema>>,
    #[serde(rename = "allOf")]
    pub all_of: Option<Vec<AsyncApiSchema>>,
    #[serde(rename = "anyOf")]
    pub any_of: Option<Vec<AsyncApiSchema>>,
    pub default: Option<serde_json::Value>,
    pub example: Option<serde_json::Value>,
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<Box<AsyncApiSchema>>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    #[serde(rename = "minLength")]
    pub min_length: Option<u64>,
    #[serde(rename = "maxLength")]
    pub max_length: Option<u64>,
    pub pattern: Option<String>,
}

impl AsyncApiSchema {
    /// Load a schema from a YAML file.
    pub fn from_yaml_file(path: &str) -> Result<HashMap<String, Self>, SchemaError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| SchemaError::IoError(e.to_string()))?;

        serde_yaml::from_str(&content).map_err(|e| SchemaError::ParseError(e.to_string()))
    }

    /// Load a schema from a YAML string.
    pub fn from_yaml_str(content: &str) -> Result<HashMap<String, Self>, SchemaError> {
        serde_yaml::from_str(content).map_err(|e| SchemaError::ParseError(e.to_string()))
    }

    /// Get the effective type of this schema.
    pub fn effective_type(&self) -> Option<&str> {
        self.schema_type.as_deref()
    }

    /// Check if a field is required.
    pub fn is_required(&self, field: &str) -> bool {
        self.required
            .as_ref()
            .map(|r| r.iter().any(|f| f == field))
            .unwrap_or(false)
    }
}

/// Schema loading/parsing errors.
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Schema not found: {0}")]
    NotFound(String),

    #[error("Invalid reference: {0}")]
    InvalidReference(String),
}

/// Registry of all schemas from the spec.
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    schemas: HashMap<String, AsyncApiSchema>,
}

impl SchemaRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load all schemas from the spec directory.
    pub fn load_from_spec_dir(spec_dir: &str) -> Result<Self, SchemaError> {
        let mut registry = Self::new();

        let schemas_dir = format!("{}/schemas", spec_dir);

        // Load all YAML files in the schemas directory
        if let Ok(entries) = std::fs::read_dir(&schemas_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "yaml").unwrap_or(false) {
                    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                        // Skip index files
                        if filename.starts_with('_') {
                            continue;
                        }

                        let schemas =
                            AsyncApiSchema::from_yaml_file(path.to_str().unwrap_or_default())?;

                        for (name, schema) in schemas {
                            registry.schemas.insert(name, schema);
                        }
                    }
                }
            }
        }

        Ok(registry)
    }

    /// Get a schema by name.
    pub fn get(&self, name: &str) -> Option<&AsyncApiSchema> {
        self.schemas.get(name)
    }

    /// List all schema names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.schemas.keys().map(|s| s.as_str())
    }

    /// Get the number of loaded schemas.
    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }
}
