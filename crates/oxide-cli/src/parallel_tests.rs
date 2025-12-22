#[cfg(test)]
mod tests {
    use crate::executor::{ExecutorConfig, execute_pipeline};
    use oxide_core::pipeline::{PipelineDefinition, StageDefinition, StepDefinition};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_sleep_step(name: &str, seconds: u32) -> StepDefinition {
        StepDefinition {
            name: name.to_string(),
            display_name: None,
            plugin: None,
            run: Some(format!("sleep {}", seconds)),
            shell: "bash".to_string(),
            working_directory: None,
            environment: None,
            variables: HashMap::new(),
            secrets: vec![],
            condition: None,
            timeout_minutes: 1,
            retry: None,
            continue_on_error: false,
            outputs: vec![],
        }
    }

    fn make_stage(name: &str, depends_on: Vec<&str>, sleep_seconds: u32) -> StageDefinition {
        StageDefinition {
            name: name.to_string(),
            display_name: None,
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
            condition: None,
            environment: None,
            variables: HashMap::new(),
            steps: vec![make_sleep_step("sleep", sleep_seconds)],
            parallel: false,
            timeout_minutes: None,
            retry: None,
            agent: None,
            matrix: None,
        }
    }

    #[tokio::test]
    async fn test_parallel_stages() {
        // Stage A and B are independent, each takes 1s.
        // Total time should be around 1s, not 2s.
        let pipeline = PipelineDefinition {
            version: "1".to_string(),
            name: "parallel-test".to_string(),
            description: None,
            triggers: vec![],
            variables: HashMap::new(),
            stages: vec![
                make_stage("stage-a", vec![], 2),
                make_stage("stage-b", vec![], 2),
            ],
            cache: None,
            artifacts: None,
            timeout_minutes: 5,
            concurrency: None,
        };

        let config = ExecutorConfig {
            workspace: PathBuf::from("."),
            variables: HashMap::new(),
            verbose: true,
        };

        let start = std::time::Instant::now();
        let result = execute_pipeline(&pipeline, &config, None).await.unwrap();
        let duration = start.elapsed();

        assert!(result.success);
        assert_eq!(result.stages.len(), 2);

        // Allow some overhead, but it should be significantly less than 4s
        println!("Duration: {:?}", duration);
        assert!(
            duration.as_secs() < 3,
            "Pipeline took too long: {:?}, expected parallel execution ~2s",
            duration
        );
    }
}
