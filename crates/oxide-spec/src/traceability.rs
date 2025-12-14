//! Traceability matrix between code and spec.

use crate::SpecLinked;
use std::collections::HashMap;

/// A traceability entry linking code to spec.
#[derive(Debug, Clone)]
pub struct TraceEntry {
    /// Rust type name (fully qualified).
    pub rust_type: String,

    /// AsyncAPI schema name.
    pub schema_name: String,

    /// Spec file path.
    pub spec_file: String,

    /// Optional line number in spec.
    pub spec_line: Option<u32>,

    /// Source file where the Rust type is defined.
    pub source_file: Option<String>,

    /// Source line where the Rust type is defined.
    pub source_line: Option<u32>,
}

/// Traceability matrix for spec-code correlation.
#[derive(Debug, Default)]
pub struct TraceabilityMatrix {
    entries: Vec<TraceEntry>,
    by_schema: HashMap<String, usize>,
    by_type: HashMap<String, usize>,
}

impl TraceabilityMatrix {
    /// Create a new empty matrix.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a type with its spec link.
    pub fn register<T: SpecLinked>(&mut self) {
        let entry = TraceEntry {
            rust_type: std::any::type_name::<T>().to_string(),
            schema_name: T::SCHEMA_NAME.to_string(),
            spec_file: T::SPEC_FILE.to_string(),
            spec_line: T::SPEC_LINE,
            source_file: None,
            source_line: None,
        };

        let idx = self.entries.len();
        self.by_schema.insert(entry.schema_name.clone(), idx);
        self.by_type.insert(entry.rust_type.clone(), idx);
        self.entries.push(entry);
    }

    /// Get all entries.
    pub fn entries(&self) -> &[TraceEntry] {
        &self.entries
    }

    /// Find entry by schema name.
    pub fn by_schema(&self, schema_name: &str) -> Option<&TraceEntry> {
        self.by_schema
            .get(schema_name)
            .map(|&idx| &self.entries[idx])
    }

    /// Find entry by Rust type name.
    pub fn by_type(&self, type_name: &str) -> Option<&TraceEntry> {
        self.by_type.get(type_name).map(|&idx| &self.entries[idx])
    }

    /// Get schemas without Rust implementations.
    pub fn unimplemented_schemas<'a>(&'a self, all_schemas: &'a [String]) -> Vec<&'a str> {
        all_schemas
            .iter()
            .filter(|s| !self.by_schema.contains_key(*s))
            .map(|s| s.as_str())
            .collect()
    }

    /// Generate a markdown report.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Spec-Code Traceability Matrix\n\n");
        md.push_str("| Schema | Rust Type | Spec File | Line |\n");
        md.push_str("|--------|-----------|-----------|------|\n");

        for entry in &self.entries {
            md.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} |\n",
                entry.schema_name,
                entry
                    .rust_type
                    .split("::")
                    .last()
                    .unwrap_or(&entry.rust_type),
                entry.spec_file,
                entry.spec_line.map(|l| l.to_string()).unwrap_or_default(),
            ));
        }

        md
    }
}

/// Macro to build a traceability matrix at compile time.
#[macro_export]
macro_rules! traceability_matrix {
    ($($type:ty),* $(,)?) => {{
        let mut matrix = $crate::TraceabilityMatrix::new();
        $(
            matrix.register::<$type>();
        )*
        matrix
    }};
}
