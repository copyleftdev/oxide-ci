//! Shared harness for the end-to-end container tests.
//!
//! Every ecosystem slice (`e2e_python_tests`, `e2e_rust_tests`,
//! `e2e_typescript_tests`) is the same experiment with a different toolchain:
//! generate a small project, run it through a real pipeline in a real
//! container, and assert on the structured [`PipelineResult`] rather than on
//! whatever the engine printed.
//!
//! ```text
//! make e2e            # hermetic tiers, safe to gate on
//! make e2e-canary     # live registries, never a gate
//! ```

use crate::executor::{ExecutorConfig, PipelineResult, StageResult, execute_pipeline};
use oxide_core::pipeline::PipelineDefinition;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Images the hermetic tiers run in. Pinned to a major so a surprise upstream
/// release cannot change what the gate means overnight.
pub const PY_IMAGE: &str = "python:3.12-slim";
pub const RUST_IMAGE: &str = "rust:1-slim";
pub const NODE_IMAGE: &str = "node:22-alpine";

pub fn require_docker() {
    let ok = Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    assert!(
        ok,
        "these tests need a running Docker daemon; they are behind the `e2e` feature so this is a \
         real failure, not a reason to skip"
    );
}

/// Pull an image before the pipeline runs.
///
/// The engine does not do this itself — see issue #51. `oxide-runner` creates
/// and starts containers but never calls Docker's image-create endpoint, so a
/// missing image surfaces as a container-creation error rather than a pull.
/// Until #51 is fixed the harness pre-pulls, so these tests measure pipeline
/// behavior instead of image-cache state. Delete this when #51 lands.
pub fn ensure_image(image: &str) {
    let present = Command::new("docker")
        .args(["image", "inspect", image])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if present {
        return;
    }
    let pulled = Command::new("docker")
        .args(["pull", "--quiet", image])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(pulled, "failed to pull {image}");
}

/// A step that runs `command` inside `image`.
///
/// The command goes in a block scalar so quoting inside it never has to be
/// escaped for YAML.
pub fn container_step(name: &str, image: &str, command: &str) -> String {
    format!(
        r#"      - name: {name}
        run: |
          {command}
        environment:
          type: container
          container:
            image: {image}
"#
    )
}

pub fn run_pipeline(yaml: &str, workspace: &Path) -> PipelineResult {
    let definition: PipelineDefinition =
        serde_yaml::from_str(yaml).expect("pipeline fixture should parse");
    let config = ExecutorConfig {
        workspace: workspace.to_path_buf(),
        variables: HashMap::new(),
        secrets: HashMap::new(),
        verbose: false,
    };
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(execute_pipeline(&definition, &config, None))
        .expect("executor should return a result, even for a failing pipeline")
}

pub fn stage_names(result: &PipelineResult) -> Vec<&str> {
    result
        .stages
        .iter()
        .map(|(name, _)| name.as_str())
        .collect()
}

pub fn stage<'a>(result: &'a PipelineResult, name: &str) -> &'a StageResult {
    result
        .stages
        .iter()
        .find(|(stage_name, _)| stage_name == name)
        .map(|(_, stage)| stage)
        .unwrap_or_else(|| {
            panic!(
                "stage `{name}` did not run; stages were {:?}",
                stage_names(result)
            )
        })
}

/// Assert the shape every hermetic success case shares: the pipeline passed,
/// the stages ran in the declared order, and every step reported success.
pub fn assert_pipeline_passed(result: &PipelineResult, expected_stages: &[&str]) {
    assert!(
        result.success,
        "pipeline should pass; stages were {:?}",
        stage_names(result)
    );
    assert_eq!(
        stage_names(result),
        expected_stages,
        "stages ran in the wrong order"
    );
    for (name, stage) in &result.stages {
        assert!(stage.success, "stage `{name}` should have passed");
        for (step_name, step) in &stage.steps {
            assert!(step.success, "step `{step_name}` should have passed");
            assert_eq!(step.exit_code, 0, "step `{step_name}` exit code");
        }
    }
}

/// Assert the shape every hermetic failure case shares: the pipeline failed,
/// the named stage is recorded as failed with a non-zero step, and the stage
/// that depended on it never ran.
///
/// A green pipeline over red tests is the worst failure a CI engine can have,
/// so this matters more than the happy path.
pub fn assert_pipeline_failed_at(result: &PipelineResult, failing: &str, skipped: &str) {
    assert!(
        !result.success,
        "pipeline must fail when the test suite fails"
    );

    let failed_stage = stage(result, failing);
    assert!(
        !failed_stage.success,
        "stage `{failing}` must be marked failed"
    );
    let (_, failing_step) = failed_stage
        .steps
        .iter()
        .find(|(_, step)| !step.success)
        .unwrap_or_else(|| panic!("stage `{failing}` must record which step failed"));
    assert_ne!(
        failing_step.exit_code, 0,
        "failing step needs a non-zero exit code"
    );

    assert!(
        !stage_names(result).contains(&skipped),
        "stage `{skipped}` depends on a failed stage and must not run; stages were {:?}",
        stage_names(result)
    );
}
