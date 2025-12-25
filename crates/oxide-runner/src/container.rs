//! Container-based step execution using Docker.

use crate::runner::{OutputLine, OutputStream, RunnerConfig, StepContext, StepResult, StepRunner};
use async_trait::async_trait;
use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use futures::StreamExt;
use oxide_core::Result;
use oxide_core::pipeline::StepDefinition;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tracing::{debug, error, info, warn};

/// Container runner for executing commands in Docker containers.
pub struct ContainerRunner {
    docker: Docker,
    config: RunnerConfig,
}

impl ContainerRunner {
    /// Create a new container runner.
    pub fn new(config: RunnerConfig) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults().map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to connect to Docker: {}", e))
        })?;

        Ok(Self { docker, config })
    }

    /// Create with an existing Docker client.
    pub fn with_docker(docker: Docker, config: RunnerConfig) -> Self {
        Self { docker, config }
    }

    async fn execute_in_container(
        &self,
        image: &str,
        command: &str,
        ctx: &StepContext,
        output_tx: mpsc::Sender<OutputLine>,
    ) -> Result<StepResult> {
        let start = std::time::Instant::now();
        let container_name = format!("oxide-{}", uuid::Uuid::new_v4());

        info!(
            image = %image,
            container = %container_name,
            command = %command,
            "Starting container execution"
        );

        // Build environment variables
        let env: Vec<String> = ctx
            .variables
            .iter()
            .chain(ctx.secrets.iter())
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // Create container config
        let container_config = Config {
            image: Some(image.to_string()),
            cmd: Some(vec![
                "sh".to_string(),
                "-c".to_string(),
                command.to_string(),
            ]),
            env: Some(env),
            working_dir: Some("/workspace".to_string()),
            host_config: Some(bollard::models::HostConfig {
                binds: Some(vec![format!("{}:/workspace", ctx.workspace.display())]),
                auto_remove: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Create container
        let create_options = CreateContainerOptions {
            name: &container_name,
            platform: None,
        };

        self.docker
            .create_container(Some(create_options), container_config)
            .await
            .map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to create container: {}", e))
            })?;

        // Start container
        self.docker
            .start_container(&container_name, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to start container: {}", e))
            })?;

        // Stream logs
        let log_options = LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut log_stream = self.docker.logs(&container_name, Some(log_options));
        let mut stdout_line_num = 0u32;
        let mut stderr_line_num = 0u32;

        while let Some(log_result) = log_stream.next().await {
            match log_result {
                Ok(LogOutput::StdOut { message }) => {
                    stdout_line_num += 1;
                    let content = String::from_utf8_lossy(&message).trim_end().to_string();
                    let output = OutputLine {
                        stream: OutputStream::Stdout,
                        content,
                        line_number: stdout_line_num,
                        timestamp: chrono::Utc::now(),
                    };
                    if output_tx.send(output).await.is_err() {
                        break;
                    }
                }
                Ok(LogOutput::StdErr { message }) => {
                    stderr_line_num += 1;
                    let content = String::from_utf8_lossy(&message).trim_end().to_string();
                    let output = OutputLine {
                        stream: OutputStream::Stderr,
                        content,
                        line_number: stderr_line_num,
                        timestamp: chrono::Utc::now(),
                    };
                    if output_tx.send(output).await.is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(error = %e, "Error reading container logs");
                    break;
                }
            }
        }

        // Wait for container to finish
        let wait_options = WaitContainerOptions {
            condition: "not-running",
        };

        let wait_result = if let Some(timeout_secs) = self.config.timeout_seconds {
            match timeout(
                Duration::from_secs(timeout_secs),
                self.docker
                    .wait_container(&container_name, Some(wait_options))
                    .next(),
            )
            .await
            {
                Ok(Some(result)) => result,
                Ok(None) => {
                    return Err(oxide_core::Error::Internal(
                        "Container wait returned no result".to_string(),
                    ));
                }
                Err(_) => {
                    warn!(timeout_secs, "Container execution timed out");
                    let _ = self
                        .docker
                        .kill_container::<String>(&container_name, None)
                        .await;
                    return Err(oxide_core::Error::Internal(
                        "Container execution timed out".to_string(),
                    ));
                }
            }
        } else {
            self.docker
                .wait_container(&container_name, Some(wait_options))
                .next()
                .await
                .ok_or_else(|| {
                    oxide_core::Error::Internal("Container wait returned no result".to_string())
                })?
        };

        let exit_code = wait_result
            .map_err(|e| oxide_core::Error::Internal(format!("Container wait failed: {}", e)))?
            .status_code as i32;

        // Cleanup container
        let remove_options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };

        if let Err(e) = self
            .docker
            .remove_container(&container_name, Some(remove_options))
            .await
        {
            warn!(container = %container_name, error = %e, "Failed to remove container");
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        debug!(
            container = %container_name,
            exit_code,
            duration_ms,
            "Container execution completed"
        );

        Ok(StepResult {
            exit_code,
            success: exit_code == 0,
            duration_ms,
            outputs: HashMap::new(),
        })
    }
}

#[async_trait]
impl StepRunner for ContainerRunner {
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

        // Get image from step variables or use default
        // Get image from step configuration or variables
        let image = if let Some(env) = &ctx.step.environment {
            if let Some(container_config) = &env.container {
                container_config.image.clone()
            } else {
                ctx.step
                    .variables
                    .get("OXIDE_CONTAINER_IMAGE")
                    .cloned()
                    .unwrap_or_else(|| "alpine:latest".to_string())
            }
        } else {
            ctx.step
                .variables
                .get("OXIDE_CONTAINER_IMAGE")
                .cloned()
                .unwrap_or_else(|| "alpine:latest".to_string())
        };

        // Handle retries
        let mut last_error = None;
        for attempt in 0..=self.config.retry_count {
            if attempt > 0 {
                info!(attempt, "Retrying container execution");
                tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
            }

            match self
                .execute_in_container(&image, command, ctx, output_tx.clone())
                .await
            {
                Ok(result) if result.success => return Ok(result),
                Ok(result) if attempt == self.config.retry_count => return Ok(result),
                Ok(_) => {
                    warn!(attempt, "Container execution failed, will retry");
                }
                Err(e) if attempt == self.config.retry_count => {
                    error!(error = %e, "Container execution failed after all retries");
                    return Err(e);
                }
                Err(e) => {
                    warn!(error = %e, attempt, "Container execution error, will retry");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| oxide_core::Error::Internal("Unknown error".to_string())))
    }

    fn can_handle(&self, step: &StepDefinition) -> bool {
        // Handle steps that have container environment configured
        if step.run.is_none() {
            return false;
        }

        if let Some(env) = &step.environment
            && env.container.is_some()
        {
            return true;
        }
        // Or env_type container?
        // pipeline.rs says: `env_type: EnvironmentType`

        step.variables.contains_key("OXIDE_CONTAINER_IMAGE")
    }
}
