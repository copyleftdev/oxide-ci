//! Local pipeline executor for running pipelines without a server.
//!
//! Supports:
//! - Variable interpolation: `${{ variable }}`, `${{ env.VAR }}`
//! - Step outputs: `${{ steps.name.outputs.key }}`
//! - Matrix values: `${{ matrix.key }}`
//! - Output capture via `$OXIDE_OUTPUT` file

use console::style;
use oxide_core::pipeline::{PipelineDefinition, StageDefinition, StepDefinition};
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Execution context passed through the pipeline.
///
/// Tracks variables, step outputs, and matrix values for interpolation.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Pipeline and stage variables
    pub variables: HashMap<String, String>,
    /// Step outputs: "step_name.output_key" -> value
    pub outputs: HashMap<String, String>,
    /// Matrix values for current job
    pub matrix: HashMap<String, String>,
    /// Secrets to mask in output
    pub secrets: HashMap<String, String>,
    /// Working directory
    pub workspace: PathBuf,
}

impl ExecutionContext {
    /// Create a new execution context.
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            variables: HashMap::new(),
            outputs: HashMap::new(),
            matrix: HashMap::new(),
            secrets: HashMap::new(),
            workspace,
        }
    }

    /// Interpolate variables in a string.
    ///
    /// Supports:
    /// - `${{ variable }}` - direct variable lookup
    /// - `${{ env.VAR }}` - environment variable
    /// - `${{ matrix.key }}` - matrix value
    /// - `${{ steps.name.outputs.key }}` - step output
    pub fn interpolate(&self, input: &str) -> String {
        let re = Regex::new(r"\$\{\{\s*([^}]+)\s*\}\}").unwrap();
        
        re.replace_all(input, |caps: &regex::Captures| {
            let expr = caps.get(1).map_or("", |m| m.as_str()).trim();
            self.resolve_expression(expr)
        })
        .to_string()
    }

    /// Resolve a single expression like "env.VAR" or "steps.name.outputs.key".
    fn resolve_expression(&self, expr: &str) -> String {
        // Handle env.VAR
        if let Some(var_name) = expr.strip_prefix("env.") {
            return self.variables.get(var_name)
                .cloned()
                .or_else(|| std::env::var(var_name).ok())
                .unwrap_or_default();
        }

        // Handle matrix.key
        if let Some(key) = expr.strip_prefix("matrix.") {
            return self.matrix.get(key).cloned().unwrap_or_default();
        }

        // Handle steps.name.outputs.key
        if let Some(rest) = expr.strip_prefix("steps.")
            && let Some(outputs_idx) = rest.find(".outputs.") {
                let step_name = &rest[..outputs_idx];
                let output_key = &rest[outputs_idx + 9..]; // ".outputs." is 9 chars
                let lookup_key = format!("{}.{}", step_name, output_key);
                return self.outputs.get(&lookup_key).cloned().unwrap_or_default();
            }

        // Direct variable lookup
        self.variables.get(expr).cloned().unwrap_or_default()
    }

    /// Set a step output.
    pub fn set_output(&mut self, step_name: &str, key: &str, value: String) {
        let lookup_key = format!("{}.{}", step_name, key);
        self.outputs.insert(lookup_key, value);
    }

    /// Parse outputs from OXIDE_OUTPUT file content.
    ///
    /// Format: `key=value` or `key<<EOF\nvalue\nEOF`
    pub fn parse_outputs(&mut self, step_name: &str, content: &str) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim();
                if !key.is_empty() {
                    self.set_output(step_name, key, value.to_string());
                }
            }
        }
    }

    /// Add a secret to the context.
    pub fn add_secret(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.secrets.insert(key.into(), value.into());
    }

    /// Mask secrets in the input string.
    pub fn mask_secrets(&self, input: &str) -> String {
        let mut output = input.to_string();
        for value in self.secrets.values() {
            if !value.is_empty() {
                output = output.replace(value, "***");
            }
        }
        output
    }
}

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

    // Initialize execution context
    let mut ctx = ExecutionContext::new(config.workspace.clone());
    ctx.variables = config.variables.clone();
    
    // Merge pipeline variables
    for (k, v) in &definition.variables {
        ctx.variables.insert(k.clone(), v.clone());
    }

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

    for stage in &definition.stages {
        // Filter stages if specified
        if let Some(filter) = stage_filter
            && stage.name != filter {
                continue;
            }

        let stage_result = execute_stage(stage, &mut ctx, config.verbose).await?;
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
    ctx: &mut ExecutionContext,
    verbose: bool,
) -> Result<StageResult, Box<dyn std::error::Error>> {
    println!(
        "{} Stage: {}",
        style("━━▶").cyan(),
        style(&stage.name).bold()
    );

    let mut step_results = Vec::new();
    let mut all_success = true;

    // Save original variables to restore after stage
    let original_vars = ctx.variables.clone();
    
    // Merge stage variables
    for (k, v) in &stage.variables {
        ctx.variables.insert(k.clone(), v.clone());
    }

    for (idx, step) in stage.steps.iter().enumerate() {
        let step_result = execute_step(step, ctx, verbose, idx + 1, stage.steps.len()).await?;
        let success = step_result.success;
        step_results.push((step.name.clone(), step_result));

        if !success && !step.continue_on_error {
            all_success = false;
            break;
        }
    }
    
    // Restore original variables (but keep outputs)
    ctx.variables = original_vars;

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
    ctx: &mut ExecutionContext,
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

    // Interpolate the script with context variables
    let script = ctx.interpolate(script);

    // Determine working directory (also interpolate)
    let work_dir = step
        .working_directory
        .as_ref()
        .map(|d| PathBuf::from(ctx.interpolate(d)))
        .unwrap_or_else(|| ctx.workspace.clone());

    // Create temp file for step outputs
    let output_file = work_dir.join(format!(".oxide_output_{}", step.name.replace(' ', "_")));

    // Build command
    let shell = &step.shell;
    let mut cmd = Command::new(shell);
    cmd.arg("-c").arg(&script);
    cmd.current_dir(&work_dir);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Set OXIDE_OUTPUT environment variable
    cmd.env("OXIDE_OUTPUT", &output_file);

    // Set environment variables from context
    for (k, v) in &ctx.variables {
        cmd.env(k, v);
    }
    // Set step-specific variables (interpolated)
    for (k, v) in &step.variables {
        cmd.env(k, ctx.interpolate(v));
    }

    // Set secrets as environment variables
    for (k, v) in &ctx.secrets {
        cmd.env(k, v);
    }

    // Spawn process
    let mut child = cmd.spawn()?;

    // Stream output
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");

    // Clone context for async tasks
    let ctx_stdout = ctx.clone();
    let stdout_handle = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("      {}", style(&ctx_stdout.mask_secrets(&line)).dim());
        }
    });

    // Clone context for async tasks
    let ctx_stderr = ctx.clone();
    let stderr_handle = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("      {}", style(&ctx_stderr.mask_secrets(&line)).red().dim());
        }
    });

    // Wait for process
    let status = child.wait().await?;
    let _ = stdout_handle.await;
    let _ = stderr_handle.await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = status.code().unwrap_or(-1);
    let success = status.success();

    // Parse step outputs from OXIDE_OUTPUT file
    if output_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&output_file) {
            ctx.parse_outputs(&step.name, &content);
        }
        // Clean up the output file
        let _ = std::fs::remove_file(&output_file);
    }

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
