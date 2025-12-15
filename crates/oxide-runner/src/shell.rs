//! Shell-based step execution on the host.

use crate::runner::{OutputLine, OutputStream, RunnerConfig, StepContext, StepResult, StepRunner};
use async_trait::async_trait;
use oxide_core::Result;
use oxide_core::pipeline::StepDefinition;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tracing::{debug, error, info, warn};

/// Shell runner for executing commands on the host.
pub struct ShellRunner {
    config: RunnerConfig,
}

impl ShellRunner {
    pub fn new(config: RunnerConfig) -> Self {
        Self { config }
    }

    async fn execute_command(
        &self,
        command: &str,
        ctx: &StepContext,
        output_tx: mpsc::Sender<OutputLine>,
    ) -> Result<StepResult> {
        let start = std::time::Instant::now();

        info!(command = %command, workspace = %ctx.workspace.display(), "Executing shell command");

        // Build environment
        let mut env_vars: HashMap<String, String> = std::env::vars().collect();
        env_vars.extend(ctx.variables.clone());
        env_vars.extend(ctx.secrets.clone());

        // Spawn the process
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&ctx.workspace)
            .envs(&env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to spawn process: {}", e)))?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Stream stdout
        let stdout_tx = output_tx.clone();
        let stdout_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut line_num = 0u32;

            while let Ok(Some(line)) = lines.next_line().await {
                line_num += 1;
                let output = OutputLine {
                    stream: OutputStream::Stdout,
                    content: line,
                    line_number: line_num,
                    timestamp: chrono::Utc::now(),
                };
                if stdout_tx.send(output).await.is_err() {
                    break;
                }
            }
        });

        // Stream stderr
        let stderr_tx = output_tx;
        let stderr_handle = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut line_num = 0u32;

            while let Ok(Some(line)) = lines.next_line().await {
                line_num += 1;
                let output = OutputLine {
                    stream: OutputStream::Stderr,
                    content: line,
                    line_number: line_num,
                    timestamp: chrono::Utc::now(),
                };
                if stderr_tx.send(output).await.is_err() {
                    break;
                }
            }
        });

        // Wait for process with optional timeout
        let wait_result = if let Some(timeout_secs) = self.config.timeout_seconds {
            match timeout(Duration::from_secs(timeout_secs), child.wait()).await {
                Ok(result) => result,
                Err(_) => {
                    warn!(timeout_secs, "Command timed out, killing process");
                    let _ = child.kill().await;
                    return Err(oxide_core::Error::Internal("Command timed out".to_string()));
                }
            }
        } else {
            child.wait().await
        };

        // Wait for output streaming to complete
        let _ = stdout_handle.await;
        let _ = stderr_handle.await;

        let status = wait_result.map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to wait for process: {}", e))
        })?;

        let exit_code = status.code().unwrap_or(-1);
        let duration_ms = start.elapsed().as_millis() as u64;

        debug!(exit_code, duration_ms, "Command completed");

        Ok(StepResult {
            exit_code,
            success: exit_code == 0,
            duration_ms,
            outputs: HashMap::new(),
        })
    }
}

impl Default for ShellRunner {
    fn default() -> Self {
        Self::new(RunnerConfig::default())
    }
}

#[async_trait]
impl StepRunner for ShellRunner {
    async fn execute(
        &self,
        ctx: &StepContext,
        output_tx: mpsc::Sender<OutputLine>,
    ) -> Result<StepResult> {
        let command = ctx
            .step
            .run
            .as_ref()
            .ok_or_else(|| oxide_core::Error::Internal("No command to run".to_string()))?;

        // Handle retries
        let mut last_error = None;
        for attempt in 0..=self.config.retry_count {
            if attempt > 0 {
                info!(attempt, "Retrying command");
                tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
            }

            match self.execute_command(command, ctx, output_tx.clone()).await {
                Ok(result) if result.success => return Ok(result),
                Ok(result) if attempt == self.config.retry_count => return Ok(result),
                Ok(_) => {
                    warn!(attempt, "Command failed, will retry");
                }
                Err(e) if attempt == self.config.retry_count => {
                    error!(error = %e, "Command failed after all retries");
                    return Err(e);
                }
                Err(e) => {
                    warn!(error = %e, attempt, "Command error, will retry");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| oxide_core::Error::Internal("Unknown error".to_string())))
    }

    fn can_handle(&self, step: &StepDefinition) -> bool {
        step.run.is_some() && step.plugin.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_step(cmd: &str) -> StepDefinition {
        StepDefinition {
            name: "test".to_string(),
            display_name: None,
            run: Some(cmd.to_string()),
            plugin: None,
            shell: "bash".to_string(),
            working_directory: None,
            environment: None,
            variables: Default::default(),
            secrets: vec![],
            condition: None,
            timeout_minutes: 30,
            retry: None,
            continue_on_error: false,
            outputs: vec![],
        }
    }

    #[tokio::test]
    async fn test_shell_runner_success() {
        let runner = ShellRunner::default();
        let (tx, mut rx) = mpsc::channel(100);

        let ctx = StepContext {
            workspace: PathBuf::from("/tmp"),
            variables: HashMap::new(),
            secrets: HashMap::new(),
            step: make_step("echo hello"),
        };

        let result = runner.execute(&ctx, tx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.exit_code, 0);

        // Check output
        let line = rx.recv().await.unwrap();
        assert_eq!(line.content, "hello");
    }

    #[tokio::test]
    async fn test_shell_runner_failure() {
        let runner = ShellRunner::default();
        let (tx, _rx) = mpsc::channel(100);

        let ctx = StepContext {
            workspace: PathBuf::from("/tmp"),
            variables: HashMap::new(),
            secrets: HashMap::new(),
            step: make_step("exit 1"),
        };

        let result = runner.execute(&ctx, tx).await.unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, 1);
    }
}
