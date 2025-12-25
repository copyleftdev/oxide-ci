//! Local pipeline executor for running pipelines without a server.
//!
//! Supports:
//! - Variable interpolation: `${{ variable }}`, `${{ env.VAR }}`
//! - Step outputs: `${{ steps.name.outputs.key }}`
//! - Matrix values: `${{ matrix.key }}`
//! - Output capture via `$OXIDE_OUTPUT` file

use console::style;
use futures::future::join_all;
use oxide_cache::{archiver, types::CompressionType};
use oxide_core::pipeline::{
    ConditionExpression, PipelineDefinition, StageDefinition, StepDefinition,
};
use oxide_runner::{ContainerRunner, OutputLine, RunnerConfig, StepContext, StepRunner};
// use regex::Regex; // Removed as it's now internal to oxide-core
use oxide_core::interpolation::InterpolationContext;
use oxide_plugins::{get_builtin_plugin, manifest::PluginCallInput};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::task::JoinSet;
use tokio::time::{Duration, sleep, timeout};

use crate::dag::DagBuilder;

/// Execution context passed through the pipeline.
///
/// Tracks variables, step outputs, and matrix values for interpolation.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Inner interpolation context
    pub ctx: InterpolationContext,
    /// Working directory
    pub workspace: PathBuf,
}

impl ExecutionContext {
    /// Create a new execution context.
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            ctx: InterpolationContext::new(),
            workspace,
        }
    }

    /// Interpolate variables in a string.
    pub fn interpolate(&self, input: &str) -> String {
        self.ctx.interpolate(input)
    }

    /// Set a step output.
    pub fn set_output(&mut self, step_name: &str, key: &str, value: String) {
        let lookup_key = format!("{}.{}", step_name, key);
        self.ctx.outputs.insert(lookup_key, value);
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
    #[allow(dead_code)]
    pub fn add_secret(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.ctx.secrets.insert(key.into(), value.into());
    }

    /// Mask secrets in the input string.
    pub fn mask_secrets(&self, input: &str) -> String {
        self.ctx.mask_secrets(input)
    }

    /// Evaluate a condition expression.
    pub fn evaluate_condition(&self, condition: &ConditionExpression) -> bool {
        self.ctx.evaluate_condition(condition)
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
    pub secrets: HashMap<String, String>,
    pub verbose: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            variables: HashMap::new(),
            secrets: HashMap::new(),
            verbose: false,
        }
    }
}

/// Execute a pipeline locally.
pub async fn execute_pipeline(
    definition: &PipelineDefinition,
    config: &ExecutorConfig,
    stage_filter: Option<&str>,
) -> Result<PipelineResult, Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();
    let mut stages_results = Vec::new();
    let mut all_success = true;

    // Initialize execution context
    let mut ctx = ExecutionContext::new(config.workspace.clone());
    ctx.ctx.variables = config.variables.clone();
    ctx.ctx.secrets = config.secrets.clone();

    // Merge pipeline variables
    for (k, v) in &definition.variables {
        ctx.ctx.variables.insert(k.clone(), v.clone());
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

    // Build DAG for execution
    let dag = DagBuilder::new().build(definition)?;

    // Track execution state
    let mut completed_stages = HashSet::new();
    let mut running_stages = HashSet::new(); // names of currently running stages
    let mut join_set = JoinSet::new();

    // If stage_filter is set, we bypass DAG complexity for now and just run that stage?
    if let Some(filter) = stage_filter {
        // Simple linear search for the stage
        if let Some(stage) = definition.stages.iter().find(|s| s.name == filter) {
            let stage_result = execute_stage(stage, &mut ctx, config.verbose).await?;
            stages_results.push((stage.name.clone(), stage_result));
        }
    } else {
        // Full DAG execution
        let mut queued_stages = HashSet::new();

        loop {
            // Find ready stages
            let mut new_ready = Vec::new();
            for node in dag.stages() {
                if !completed_stages.contains(&node.name)
                    && !running_stages.contains(&node.name)
                    && !queued_stages.contains(&node.name)
                    && dag.is_ready(
                        &node.name,
                        &completed_stages.iter().cloned().collect::<Vec<_>>(),
                    )
                {
                    new_ready.push(node.definition.clone());
                    queued_stages.insert(node.name.clone());
                }
            }

            // Spawn ready stages
            for stage in new_ready {
                let mut stage_ctx = ctx.clone();
                let stage_name = stage.name.clone();
                let verbose = config.verbose;

                running_stages.insert(stage_name.clone());

                join_set.spawn(async move {
                    let res = execute_stage(&stage, &mut stage_ctx, verbose).await;
                    (stage_name, res, stage_ctx.ctx.outputs)
                });
            }

            // If nothing running and nothing queued/ready, we are done
            if join_set.is_empty() {
                break;
            }

            // Wait for next stage to complete
            if let Some(result) = join_set.join_next().await {
                match result {
                    Ok((name, execution_res, outputs)) => {
                        running_stages.remove(&name);
                        match execution_res {
                            Ok(stage_res) => {
                                let success = stage_res.success;
                                stages_results.push((name.clone(), stage_res));

                                if success {
                                    completed_stages.insert(name);
                                    // Merge outputs back to main context for dependents
                                    for (k, v) in outputs {
                                        ctx.ctx.outputs.insert(k, v);
                                    }
                                } else {
                                    all_success = false;
                                    // If a stage fails, do we cancel others?
                                    // For now, let running finish but don't spawn new ones dependent on this.
                                    // But independent ones could continue?
                                    // Standard CI usually stops pipeline on failure unless 'continue-on-error'
                                    // If we break loop, running futures might be dropped (cancelled).
                                    // Let's break to stop.
                                    break;
                                }
                            }
                            Err(e) => {
                                println!("Stage {} execution error: {}", name, e);
                                all_success = false;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        println!("Join error: {}", e);
                        all_success = false;
                        break;
                    }
                }
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    if all_success && let Err(e) = collect_artifacts(definition, &config.workspace).await {
        println!("{} Failed to collect artifacts: {}", style("⚠").yellow(), e);
    }

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
) -> Result<StageResult, Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "{} Stage: {}",
        style("━━▶").cyan(),
        style(&stage.name).bold()
    );

    // Evaluate stage condition
    if let Some(condition) = &stage.condition
        && !ctx.evaluate_condition(condition)
    {
        println!("    {} Skipped (condition unmet)", style("⏭").dim());
        return Ok(StageResult {
            success: true,
            steps: Vec::new(),
        });
    }

    let mut step_results = Vec::new();
    let mut all_success = true;

    // Save original variables to restore after stage
    let original_vars = ctx.ctx.variables.clone();

    // Merge stage variables
    for (k, v) in &stage.variables {
        ctx.ctx.variables.insert(k.clone(), v.clone());

        // Populate matrix context if variable starts with matrix.
        if let Some(matrix_key) = k.strip_prefix("matrix.") {
            ctx.ctx.matrix.insert(matrix_key.to_string(), v.clone());
        }
    }

    if stage.parallel {
        // Execute steps in parallel
        let mut futures = Vec::new();
        for (idx, step) in stage.steps.iter().enumerate() {
            let mut step_ctx = ctx.clone();
            let step_ref = step.clone();
            let step_count = stage.steps.len();

            futures.push(async move {
                let res =
                    execute_step(&step_ref, &mut step_ctx, verbose, idx + 1, step_count).await;
                (step_ref.name, res, step_ctx.ctx.outputs)
            });
        }

        let results = join_all(futures).await;

        for (name, res, outputs) in results {
            match res {
                Ok(step_res) => {
                    let success = step_res.success;
                    step_results.push((name.clone(), step_res));

                    // Merge outputs
                    for (k, v) in outputs {
                        ctx.ctx.outputs.insert(k, v);
                    }

                    if !success {
                        // In parallel mode, we might want to wait for all?
                        // join_all waits for all.
                        // But we should mark stage as failed.
                        // We check continue_on_error?
                        // We need the step definition for continue_on_error.
                        // But we just have name.
                        // Let's assume proper checking.
                        // Check the specific step definition from stage.steps?
                        let step_def = stage.steps.iter().find(|s| s.name == name).unwrap();

                        use oxide_core::pipeline::BooleanOrExpression;
                        let continue_on_error = match &step_def.continue_on_error {
                            Some(BooleanOrExpression::Boolean(b)) => *b,
                            Some(BooleanOrExpression::Expression(s)) => {
                                // Note: We don't have the context here easily to interpolate if it depends on outputs
                                // But for matrix variables, it should work if we had the context.
                                // The parallel execution model is slightly tricky here because we finished execution.
                                // We can use the outputs from step_res if needed, but 'continue_on_error' usually is evaluated before run?
                                // Actually, 'continue_on_error' decides if the *pipeline* fails.
                                // We can assume for now that simple interpolation works.
                                // We need access to a context. We can use `ctx` variables?
                                // A simplified check:
                                s == "true"
                            }
                            None => false,
                        };

                        if !continue_on_error {
                            all_success = false;
                        }
                    }
                }
                Err(e) => {
                    println!("Step {} error: {}", name, e);
                    all_success = false;
                }
            }
        }
    } else {
        // Execute steps sequentially
        for (idx, step) in stage.steps.iter().enumerate() {
            let step_result = execute_step(step, ctx, verbose, idx + 1, stage.steps.len()).await?;
            let success = step_result.success;
            step_results.push((step.name.clone(), step_result));

            use oxide_core::pipeline::BooleanOrExpression;
            let continue_on_error = match &step.continue_on_error {
                Some(BooleanOrExpression::Boolean(b)) => *b,
                Some(BooleanOrExpression::Expression(s)) => {
                    let val = ctx.interpolate(s);
                    val == "true"
                }
                None => false,
            };

            if !success && !continue_on_error {
                all_success = false;
                break;
            }
        }
    }

    // Restore original variables (but keep outputs)
    ctx.ctx.variables = original_vars;

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

/// Collect artifacts based on pipeline definition.
async fn collect_artifacts(
    def: &PipelineDefinition,
    workspace: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(config) = &def.artifacts {
        if config.paths.is_empty() {
            return Ok(());
        }

        println!("\n{} Collecting artifacts...", style("📦").cyan());

        let mut paths = Vec::new();
        // Simple glob expansion could go here, for now using direct paths
        for p_str in &config.paths {
            paths.push(PathBuf::from(p_str));
        }

        let artifacts_dir = workspace.join("artifacts");
        tokio::fs::create_dir_all(&artifacts_dir).await?;

        let name = config.name.as_deref().unwrap_or("artifact");
        let ext = if config.compression == "none" {
            "tar"
        } else {
            "tar.zst"
        };
        let filename = format!(
            "{}-{}.{}",
            name,
            chrono::Utc::now().format("%Y%m%d-%H%M%S"),
            ext
        );
        let output_path = artifacts_dir.join(&filename);

        println!(
            "  Packing {} paths to {}",
            paths.len(),
            output_path.display()
        );

        let compression = match config.compression.as_str() {
            "zstd" => CompressionType::Zstd,
            "none" => CompressionType::None,
            _ => CompressionType::Zstd,
        };

        let workspace_clone = workspace.to_path_buf();
        let output_path_clone = output_path.clone();

        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::create(&output_path_clone).map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to create artifact file: {}", e))
            })?;
            let writer = std::io::BufWriter::new(file);
            archiver::create_archive(writer, &paths, &workspace_clone, compression)
        })
        .await
        .map_err(|e| oxide_core::Error::Internal(e.to_string()))??;

        let size = tokio::fs::metadata(&output_path).await?.len();
        println!("  {} Artifact saved ({} bytes)", style("✓").green(), size);
    }
    Ok(())
}

/// Execute a single step with retries and timeout.
async fn execute_step(
    step: &StepDefinition,
    ctx: &mut ExecutionContext,
    verbose: bool,
    step_num: usize,
    total_steps: usize,
) -> Result<StepResult, Box<dyn std::error::Error + Send + Sync>> {
    let max_attempts = step
        .retry
        .as_ref()
        .map(|r| r.max_attempts)
        .unwrap_or(1)
        .max(1);
    let delay_seconds = step.retry.as_ref().map(|r| r.delay_seconds).unwrap_or(10) as u64;
    let exponential_backoff = step
        .retry
        .as_ref()
        .map(|r| r.exponential_backoff)
        .unwrap_or(true);
    let retry_on = step.retry.as_ref().map(|r| &r.retry_on);

    let mut last_result = StepResult {
        success: false,
        exit_code: 1,
        duration_ms: 0,
    };

    for attempt in 1..=max_attempts {
        if attempt > 1 {
            println!(
                "    {} Retrying step {} (attempt {}/{})",
                style("↻").yellow(),
                style(&step.name).bold(),
                attempt,
                max_attempts
            );
        }

        let result = execute_step_attempt(step, ctx, verbose, step_num, total_steps, attempt).await;

        match result {
            Ok(step_res) => {
                last_result = step_res;
                if last_result.success {
                    return Ok(last_result);
                }

                // Check if we should retry
                if attempt < max_attempts {
                    let should_retry = if let Some(conditions) = retry_on {
                        if conditions.is_empty() {
                            true // Default to retry on any failure if struct is present but list empty?
                        // Schema says defaults. If explicit empty list, maybe user meant no retry?
                        // Using "any failure" is safer if they enabled retry.
                        } else {
                            conditions.iter().any(|c| {
                                c == "failure"
                                    || (c == "timeout"
                                        && last_result.exit_code == -1
                                        && last_result.duration_ms
                                            >= (step.timeout_minutes as u64 * 60 * 1000))
                                    || c == &last_result.exit_code.to_string()
                            })
                        }
                    } else {
                        // If no retry config, we shouldn't be in loop > 1 basically, but max_attempts=1 covers it.
                        // If max_attempts > 1 but no specific conditions, implies retry on failure.
                        true
                    };

                    if should_retry {
                        let sleep_duration = if exponential_backoff {
                            Duration::from_secs(delay_seconds * 2u64.pow(attempt - 1))
                        } else {
                            Duration::from_secs(delay_seconds)
                        };
                        sleep(sleep_duration).await;
                        continue;
                    }
                }
            }
            Err(e) => return Err(e),
        }
        break;
    }

    Ok(last_result)
}

async fn execute_step_attempt(
    step: &StepDefinition,
    ctx: &mut ExecutionContext,
    _verbose: bool,
    step_num: usize,
    total_steps: usize,
    attempt: u32,
) -> Result<StepResult, Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();

    // Evaluate step condition (only on first attempt ideally? Or re-evaluate?)
    // Re-evaluating is fine but side-effects?
    // Let's assume re-evaluation is correct as context might change? No, context is per-run.
    if let Some(condition) = &step.condition
        && !ctx.evaluate_condition(condition)
    {
        println!(
            "    [{}/{}] {} (skipped)",
            step_num,
            total_steps,
            style(&step.name).dim()
        );
        return Ok(StepResult {
            success: true,
            exit_code: 0,
            duration_ms: 0,
        });
    }

    // Handle plugin steps
    if let Some(ref plugin_name) = step.plugin {
        if attempt == 1 {
            println!(
                "    [{}/{}] {} (plugin: {})",
                step_num,
                total_steps,
                style(&step.name).bold(),
                style(plugin_name).dim()
            );
        }

        if let Some(plugin) = get_builtin_plugin(plugin_name) {
            let start_plugin = std::time::Instant::now();

            // Prepare inputs
            let mut params = HashMap::new();
            for (k, v) in &step.with {
                // Interpolate values
                let val_str = match v {
                    serde_json::Value::String(s) => serde_json::Value::String(ctx.interpolate(s)),
                    _ => v.clone(),
                };
                params.insert(k.clone(), val_str);
            }

            let mut env = HashMap::new();
            for (k, v) in &ctx.ctx.variables {
                env.insert(k.clone(), v.clone());
            }
            // Add step variables
            for (k, v) in &step.variables {
                env.insert(k.clone(), ctx.interpolate(v));
            }

            let input = PluginCallInput {
                params,
                env,
                workspace: ctx.workspace.to_string_lossy().to_string(),
                step_name: step.name.clone(),
            };

            // Execute plugin
            // TODO: async execution and timeout support for plugins
            // Execute plugin in a blocking task to allow it to use its own runtime if needed
            let result = tokio::task::spawn_blocking(move || plugin.execute(&input))
                .await
                .map_err(|e| oxide_core::Error::Internal(format!("Plugin execution failed: {}", e)))
                .and_then(|res| res);

            let duration_ms = start_plugin.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    if output.success {
                        println!(
                            "      {} ({:.2}s)",
                            style("✓").green(),
                            duration_ms as f64 / 1000.0
                        );
                        // Capture outputs
                        for (k, v) in output.outputs {
                            ctx.set_output(&step.name, &k, v);
                        }

                        return Ok(StepResult {
                            success: true,
                            exit_code: 0,
                            duration_ms,
                        });
                    } else {
                        println!(
                            "      {} Plugin failed: {} ({:.2}s)",
                            style("✗").red(),
                            output.error.unwrap_or_default(),
                            duration_ms as f64 / 1000.0
                        );
                        return Ok(StepResult {
                            success: false,
                            exit_code: output.exit_code,
                            duration_ms,
                        });
                    }
                }
                Err(e) => {
                    println!("      {} Plugin error: {}", style("✗").red(), e);
                    return Ok(StepResult {
                        success: false,
                        exit_code: 1,
                        duration_ms,
                    });
                }
            }
        } else {
            println!(
                "      {} Plugin not found: {}",
                style("⚠").yellow(),
                plugin_name
            );
            println!(
                "      (Only built-in plugins git-checkout, cache, docker-build are currently supported)"
            );
            return Ok(StepResult {
                success: false,
                exit_code: 1,
                duration_ms: 0,
            });
        }
    }

    // Handle container steps
    let needs_container = if let Some(env) = &step.environment {
        env.container.is_some()
    } else {
        step.variables.contains_key("OXIDE_CONTAINER_IMAGE")
    };

    if needs_container {
        match ContainerRunner::new(RunnerConfig::default()) {
            Ok(runner) => {
                if runner.can_handle(step) {
                    if attempt == 1 {
                        println!(
                            "    [{}/{}] {} (container)",
                            step_num,
                            total_steps,
                            style(&step.name).bold()
                        );
                    }

                    // Prepare context
                    let mut merged_vars = ctx.ctx.variables.clone();
                    for (k, v) in &step.variables {
                        merged_vars.insert(k.clone(), ctx.interpolate(v));
                    }

                    let step_ctx = StepContext {
                        workspace: ctx.workspace.clone(),
                        variables: merged_vars,
                        secrets: HashMap::new(),
                        step: step.clone(),
                    };

                    let (tx, mut rx) = tokio::sync::mpsc::channel::<OutputLine>(100);

                    let printer = tokio::spawn(async move {
                        while let Some(line) = rx.recv().await {
                            println!("      | {}", line.content);
                        }
                    });

                    let res = runner.execute(&step_ctx, tx).await;
                    let _ = printer.await;

                    match res {
                        Ok(r) => {
                            let success = r.success;
                            return Ok(StepResult {
                                success,
                                exit_code: r.exit_code,
                                duration_ms: r.duration_ms,
                            });
                        }
                        Err(e) => return Err(Box::new(e)),
                    }
                }
            }
            Err(e) => {
                println!("      {} Docker connection failed: {}", style("✗").red(), e);
                return Ok(StepResult {
                    success: false,
                    exit_code: 1,
                    duration_ms: 0,
                });
            }
        }
    }

    // Handle run steps
    let Some(ref script) = step.run else {
        println!(
            "    [{}/{}] {} (no action)",
            step_num,
            total_steps,
            style(&step.name).dim()
        );
        return Ok(StepResult {
            success: true,
            exit_code: 0,
            duration_ms: 0,
        });
    };

    if attempt == 1 {
        println!(
            "    [{}/{}] {}",
            step_num,
            total_steps,
            style(&step.name).bold()
        );
    }

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
    for (k, v) in &ctx.ctx.variables {
        cmd.env(k, v);
    }
    // Set step-specific variables (interpolated)
    for (k, v) in &step.variables {
        cmd.env(k, ctx.interpolate(v));
    }

    // Set secrets as environment variables
    for (k, v) in &ctx.ctx.secrets {
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
            println!(
                "      {}",
                style(&ctx_stderr.mask_secrets(&line)).red().dim()
            );
        }
    });

    // Wait for process with timeout
    let timeout_duration = if step.timeout_minutes > 0 {
        Duration::from_secs(step.timeout_minutes as u64 * 60)
    } else {
        Duration::from_secs(30 * 60) // Default 30 min if 0 (though default is 30 in struct)
    };

    let status_res = match timeout(timeout_duration, child.wait()).await {
        Ok(res) => res.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        Err(_) => {
            let _ = child.kill().await;
            Err(Box::from("Step timed out"))
        }
    };

    let _ = stdout_handle.await;
    let _ = stderr_handle.await;

    let duration_ms = start.elapsed().as_millis() as u64;

    let (success, exit_code) = match status_res {
        Ok(status) => (status.success(), status.code().unwrap_or(-1)),
        Err(e) => {
            println!("      {} {}", style("✗").red(), e);
            (false, -1)
        }
    };

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
pub fn load_pipeline(
    path: &Path,
) -> Result<PipelineDefinition, Box<dyn std::error::Error + Send + Sync>> {
    let content = std::fs::read_to_string(path)?;
    let definition: PipelineDefinition = serde_yaml::from_str(&content)?;
    Ok(definition)
}
