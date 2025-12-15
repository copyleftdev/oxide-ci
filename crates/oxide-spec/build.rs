//! Build script for oxide-spec.
//!
//! This script:
//! 1. Parses the AsyncAPI spec at build time
//! 2. Generates a list of all schema names
//! 3. Optionally generates Rust types from schemas (future)

#![allow(clippy::collapsible_if)]

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Path to spec directory (relative to oxide-spec crate)
    let spec_dir = Path::new(&manifest_dir).join("../../spec");

    if !spec_dir.exists() {
        println!("cargo:warning=Spec directory not found at {:?}", spec_dir);
        return;
    }

    // Collect all schema names
    let mut schema_names = HashSet::new();

    let schemas_dir = spec_dir.join("schemas");
    if schemas_dir.exists() {
        if let Ok(entries) = fs::read_dir(&schemas_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_yaml = path.extension().map(|e| e == "yaml").unwrap_or(false);
                let filename = path.file_name().and_then(|f| f.to_str());

                if is_yaml {
                    if let Some(fname) = filename {
                        // Skip index files
                        if fname.starts_with('_') {
                            continue;
                        }

                        // Read and parse the YAML to extract schema names
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                                if let Some(mapping) = yaml.as_mapping() {
                                    for key in mapping.keys() {
                                        if let Some(name) = key.as_str() {
                                            schema_names.insert(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Generate schema_names.rs
    let schema_names_code = format!(
        r#"/// All schema names from the AsyncAPI spec.
pub const SCHEMA_NAMES: &[&str] = &[
{}
];

/// Number of schemas in the spec.
pub const SCHEMA_COUNT: usize = {};
"#,
        schema_names
            .iter()
            .map(|s| format!("    \"{}\",", s))
            .collect::<Vec<_>>()
            .join("\n"),
        schema_names.len(),
    );

    let schema_names_path = Path::new(&out_dir).join("schema_names.rs");
    fs::write(&schema_names_path, schema_names_code).unwrap();

    // Tell Cargo to rerun if spec files change
    println!("cargo:rerun-if-changed=../../spec/schemas");
    if let Ok(entries) = fs::read_dir(&schemas_dir) {
        for entry in entries.flatten() {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }

    println!("cargo:warning=Found {} schemas in spec", schema_names.len());
}
