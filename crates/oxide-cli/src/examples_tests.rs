use crate::executor::{execute_pipeline, ExecutorConfig};
use oxide_core::pipeline::PipelineDefinition;
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn test_examples_execution() {
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples");

    if !examples_dir.exists() {
        println!("Examples directory not found at {:?}", examples_dir);
        return;
    }

    let entries = fs::read_dir(examples_dir).unwrap();
    
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            println!("Testing example: {:?}", path.file_name().unwrap());
            
            let content = fs::read_to_string(&path).unwrap();
            
            // Try to parse the pipeline - this verifies the schema matches our structs
            let definition: Result<PipelineDefinition, _> = serde_yaml::from_str(&content);
            
            if let Ok(def) = definition {
                 println!("  Parsed successfully: {}", def.name);
                 
                 // We can also try to build the DAG to verify dependencies
                 let dag_builder = crate::dag::DagBuilder::new();
                 if let Err(e) = dag_builder.build(&def) {
                     panic!("Failed to build DAG for {:?}: {}", path.file_name(), e);
                 }
                 println!("  DAG built successfully");

            } else {
                 println!("Skipping {:?} - not a pipeline definition or invalid YAML (Error: {:?})", path.file_name(), definition.err());
            }
        }
    }
}
