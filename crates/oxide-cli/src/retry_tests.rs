use crate::executor::{ExecutorConfig, execute_pipeline};
use oxide_core::pipeline::PipelineDefinition;

#[tokio::test]
async fn test_retry_logic() {
    let yaml = r#"
name: retry-test
version: "1"
stages:
  - name: retry-stage
    steps:
      - name: flaky-step
        run: |
          if [ -f flaky_marker ]; then
            echo "Success on retry"
            exit 0
          else
            touch flaky_marker
            echo "Failing first time"
            exit 1
          fi
        retry:
          max_attempts: 2
          delay_seconds: 1
          exponential_backoff: false
"#;

    let def: PipelineDefinition = serde_yaml::from_str(yaml).expect("Failed to parse YAML");

    // Use a temp dir
    let temp_dir = tempfile::tempdir().unwrap();

    let config = ExecutorConfig {
        workspace: temp_dir.path().to_path_buf(),
        variables: std::collections::HashMap::new(),
        secrets: std::collections::HashMap::new(),
        verbose: true,
    };

    let result = execute_pipeline(&def, &config, None)
        .await
        .expect("Execution failed");

    assert!(result.success, "Pipeline should succeed after retry");
    // Verify it took 2 attempts?
    // We can't easily inspect internal logs here, but success proves it retried,
    // because first run GUARANTEED exit 1.
}
