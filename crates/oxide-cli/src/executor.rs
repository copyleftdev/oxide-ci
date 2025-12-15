//! Local pipeline executor for running pipelines without a server.

use console::style;
use oxide_core::pipeline::{PipelineDefinition, StageDefinition, StepDefinition};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Result of a step execution.
#[derive(Debug)]
#[allow(dead_code)]
pub struct StepResult {
    pub success: bool,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// Result of a stage execution.
#[derive(Debug)]
#[allow(dead_code)]
pub struct StageResult {
    pub success: bool,
    pub steps: Vec<(String, StepResult)>,
}

/// Result of a pipeline execution.
#[derive(Debug)]
#[allow(dead_code)]
pub struct PipelineResult {
    pub success: bool,
    pub stages: Vec<(String, StageResult)>,
    pub duration_ms: u64,
}

/// Local executor configuration.
pub struct ExecutorConfig {
    pub workspace: PathBuf,
    pub variables: HashMap<String, String>,
    pub verbose: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            variables: HashMap::new(),
            verbose: false,
        }
    }
}

/// Execute a pipeline locally.
pub async fn execute_pipeline(
    definition: &PipelineDefinition,
    config: &ExecutorConfig,
    stage_filter: Option<&str>,
) -> Result<PipelineResult, Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    let mut stages_results = Vec::new();
    let mut all_success = true;

    println!(
        "\n{} Running pipeline: {}",
        style("▶").cyan().bold(),
        style(&definition.name).bold()
    );
    println!(
        "  {} stages, timeout: {} min\n",
        definition.stages.len(),
        definition.timeout_minutes
    );

    // Merge pipeline variables with config variables
    let mut variables = config.variables.clone();
    for (k, v) in &definition.variables {
        variables.insert(k.clone(), v.clone());
    }

    for stage in &definition.stages {
        // Filter stages if specified
        if let Some(filter) = stage_filter {
            if stage.name != filter {
                continue;
            }
        }

        let stage_result = execute_stage(stage, &config.workspace, &variables, config.verbose).await?;
        let success = stage_result.success;
        stages_results.push((stage.name.clone(), stage_result));

        if !success {
            all_success = false;
            break; // Stop on first failure
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    // Print summary
    println!();
    if all_success {
        println!(
            "{} Pipeline completed successfully in {:.2}s",
            style("✓").green().bold(),
            duration_ms as f64 / 1000.0
        );
    } else {
        println!(
            "{} Pipeline failed after {:.2}s",
            style("✗").red().bold(),
            duration_ms as f64 / 1000.0
        );
    }

    Ok(PipelineResult {
        success: all_success,
        stages: stages_results,
        duration_ms,
    })
}

/// Execute a single stage.
async fn execute_stage(
    stage: &StageDefinition,
    workspace: &Path,
    variables: &HashMap<String, String>,
    verbose: bool,
) -> Result<StageResult, Box<dyn std::error::Error>> {
    println!(
        "{} Stage: {}",
        style("━━▶").cyan(),
        style(&stage.name).bold()
    );

    let mut step_results = Vec::new();
    let mut all_success = true;

    // Merge stage variables
    let mut stage_vars = variables.clone();
    for (k, v) in &stage.variables {
        stage_vars.insert(k.clone(), v.clone());
    }

    for (idx, step) in stage.steps.iter().enumerate() {
        let step_result = execute_step(step, workspace, &stage_vars, verbose, idx + 1, stage.steps.len()).await?;
        let success = step_result.success;
        step_results.push((step.name.clone(), step_result));

        if !success && !step.continue_on_error {
            all_success = false;
            break;
        }
    }

    if all_success {
        println!(
            "    {} Stage {} passed\n",
            style("✓").green(),
            style(&stage.name).dim()
        );
    } else {
        println!(
            "    {} Stage {} failed\n",
            style("✗").red(),
            style(&stage.name).dim()
        );
    }

    Ok(StageResult {
        success: all_success,
        steps: step_results,
    })
}

/// Execute a single step.
async fn execute_step(
    step: &StepDefinition,
    workspace: &Path,
    variables: &HashMap<String, String>,
    _verbose: bool,
    step_num: usize,
    total_steps: usize,
) -> Result<StepResult, Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();

    // Handle plugin steps (skip for now, just log)
    if let Some(ref plugin) = step.plugin {
        println!(
            "    [{}/{}] {} (plugin: {})",
            step_num,
            total_steps,
            style(&step.name).bold(),
            style(plugin).dim()
        );
        println!("      {} Plugin execution not yet implemented", style("⚠").yellow());
        return Ok(StepResult {
            success: true,
            exit_code: 0,
            duration_ms: 0,
        });
    }

    // Handle run steps
    let Some(ref script) = step.run else {
        println!(
            "    [{}/{}] {} (no action)",
            step_num, total_steps,
            style(&step.name).dim()
        );
        return Ok(StepResult {
            success: true,
            exit_code: 0,
            duration_ms: 0,
        });
    };

    println!(
        "    [{}/{}] {}",
        step_num,
        total_steps,
        style(&step.name).bold()
    );

    // Determine working directory
    let work_dir = step
        .working_directory
        .as_ref()
        .map(|d| workspace.join(d))
        .unwrap_or_else(|| workspace.to_path_buf());

    // Build command
    let shell = &step.shell;
    let mut cmd = Command::new(shell);
    cmd.arg("-c").arg(script);
    cmd.current_dir(&work_dir);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Set environment variables
    for (k, v) in variables {
        cmd.env(k, v);
    }
    for (k, v) in &step.variables {
        cmd.env(k, v);
    }

    // Spawn process
    let mut child = cmd.spawn()?;

    // Stream output
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");

    let stdout_handle = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("      {}", style(&line).dim());
        }
    });

    let stderr_handle = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("      {}", style(&line).red().dim());
        }
    });

    // Wait for process
    let status = child.wait().await?;
    let _ = stdout_handle.await;
    let _ = stderr_handle.await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = status.code().unwrap_or(-1);
    let success = status.success();

    if success {
        println!(
            "      {} ({:.2}s)",
            style("✓").green(),
            duration_ms as f64 / 1000.0
        );
    } else {
        println!(
            "      {} exit code {} ({:.2}s)",
            style("✗").red(),
            exit_code,
            duration_ms as f64 / 1000.0
        );
    }

    Ok(StepResult {
        success,
        exit_code,
        duration_ms,
    })
}

/// Find pipeline file in standard locations.
pub fn find_pipeline_file(path: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = path {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }

    // Check standard locations
    let candidates = [
        ".oxide-ci/pipeline.yaml",
        ".oxide-ci/pipeline.yml",
        "oxide.yaml",
        "oxide.yml",
        ".oxide.yaml",
        ".oxide.yml",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Load and parse a pipeline file.
pub fn load_pipeline(path: &Path) -> Result<PipelineDefinition, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let definition: PipelineDefinition = serde_yaml::from_str(&content)?;
    Ok(definition)
}
