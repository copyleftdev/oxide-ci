
use crate::executor::{execute_pipeline, ExecutorConfig};
use oxide_core::pipeline::PipelineDefinition;


#[tokio::test]
async fn test_artifact_collection() {
    let yaml = r#"
name: artifact-test
version: "1"
artifacts:
  paths: ["build-output"]
  name: "my-build"
  compression: "zstd"
stages:
  - name: build
    steps:
      - name: create-output
        run: |
          mkdir -p build-output
          echo "Build Result" > build-output/result.txt
"#;

    let def: PipelineDefinition = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
    let temp_ws = tempfile::tempdir().unwrap();

    let config = ExecutorConfig {
        workspace: temp_ws.path().to_path_buf(),
        variables: std::collections::HashMap::new(),
        secrets: std::collections::HashMap::new(),
        verbose: true,
    };

    let res = execute_pipeline(&def, &config, None).await.expect("Execution failed");
    assert!(res.success, "Pipeline should succeed");

    // Check artifacts dir
    let artifacts_dir = temp_ws.path().join("artifacts");
    assert!(artifacts_dir.exists(), "Artifacts dir should exist");

    // Find the artifact file (name includes timestamp)
    let mut found = false;
    for entry in std::fs::read_dir(artifacts_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("my-build-") && name.ends_with(".tar.zst") {
            found = true;
            println!("Found artifact: {}", name);
            break;
        }
    }
    assert!(found, "Artifact file not found");
}
