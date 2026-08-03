//! Container-based step execution using Docker.

use crate::runner::{OutputLine, OutputStream, RunnerConfig, StepContext, StepResult, StepRunner};
use async_trait::async_trait;
use bollard::Docker;
use bollard::auth::DockerCredentials;
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
use futures::StreamExt;
use oxide_core::Result;
use oxide_core::pipeline::StepDefinition;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tracing::{debug, error, info, warn};

/// Split an image reference into the name and tag Docker's pull API expects.
///
/// The naive `rfind(':')` is wrong twice over: a registry port (`reg:5000/img`)
/// puts a colon before the path, and a digest (`img@sha256:...`) puts one after
/// it. Both must resolve to "no tag".
fn split_image_reference(image: &str) -> (String, String) {
    if image.contains('@') {
        // Digest-pinned: Docker takes the whole reference and no tag.
        return (image.to_string(), String::new());
    }
    match image.rfind(':') {
        Some(index) if !image[index..].contains('/') => {
            (image[..index].to_string(), image[index + 1..].to_string())
        }
        _ => (image.to_string(), "latest".to_string()),
    }
}

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

    /// Pull `image` unless it is already present locally.
    ///
    /// Docker will not fetch an image on container creation, so without this a
    /// pipeline fails on any host with a cold image cache — a fresh agent, a
    /// new contributor, a clean CI runner (#51).
    async fn ensure_image(
        &self,
        image: &str,
        credentials: Option<DockerCredentials>,
        output_tx: &mpsc::Sender<OutputLine>,
    ) -> Result<()> {
        if self.docker.inspect_image(image).await.is_ok() {
            debug!(image = %image, "Image already present");
            return Ok(());
        }

        let (from_image, tag) = split_image_reference(image);
        info!(image = %image, "Pulling image");

        let options = CreateImageOptions {
            from_image: from_image.as_str(),
            tag: tag.as_str(),
            ..Default::default()
        };

        let mut stream = self.docker.create_image(Some(options), None, credentials);
        let mut line_number = 0u32;

        while let Some(update) = stream.next().await {
            match update {
                Ok(info) => {
                    // Layer-level progress carries an id and would flood the
                    // log; the id-less entries are the milestones a human
                    // wants ("Pulling from ...", "Downloaded newer image").
                    if let (Some(status), None) = (info.status.as_ref(), info.id.as_ref()) {
                        line_number += 1;
                        let _ = output_tx
                            .send(OutputLine {
                                stream: OutputStream::Stdout,
                                content: format!("{}: {}", image, status),
                                line_number,
                                timestamp: chrono::Utc::now(),
                            })
                            .await;
                    }
                }
                Err(e) => {
                    return Err(oxide_core::Error::Internal(format!(
                        "Failed to pull image `{}`: {}",
                        image, e
                    )));
                }
            }
        }

        info!(image = %image, "Image pulled");
        Ok(())
    }

    async fn execute_in_container(
        &self,
        image: &str,
        command: &str,
        ctx: &StepContext,
        credentials: Option<DockerCredentials>,
        output_tx: mpsc::Sender<OutputLine>,
    ) -> Result<StepResult> {
        let start = std::time::Instant::now();
        let container_name = format!("oxide-{}", uuid::Uuid::new_v4());

        self.ensure_image(image, credentials, &output_tx).await?;

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

        // A container that exits non-zero is a failed step, not a failed engine.
        // Bollard surfaces that case as DockerContainerWaitError carrying the
        // exit code, so unwrapping it with `?` would turn every failing test
        // suite into an internal error — and skip the cleanup below, leaking
        // the container.
        let exit_code = match wait_result {
            Ok(response) => response.status_code as i32,
            Err(bollard::errors::Error::DockerContainerWaitError { code, .. }) => code as i32,
            Err(e) => {
                let remove_options = RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                };
                if let Err(cleanup) = self
                    .docker
                    .remove_container(&container_name, Some(remove_options))
                    .await
                {
                    warn!(container = %container_name, error = %cleanup, "Failed to remove container");
                }
                return Err(oxide_core::Error::Internal(format!(
                    "Container wait failed: {}",
                    e
                )));
            }
        };

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

        // Registry credentials, if the step configured a private registry.
        // `password_secret` names a secret rather than holding one, so it is
        // resolved against the step's secrets.
        let credentials = ctx
            .step
            .environment
            .as_ref()
            .and_then(|env| env.container.as_ref())
            .and_then(|container| container.registry.as_ref())
            .map(|auth| DockerCredentials {
                username: auth.username.clone(),
                password: auth
                    .password_secret
                    .as_ref()
                    .and_then(|name| ctx.secrets.get(name).cloned()),
                serveraddress: auth.url.clone(),
                ..Default::default()
            });

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
                .execute_in_container(&image, command, ctx, credentials.clone(), output_tx.clone())
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

#[cfg(test)]
mod tests {
    use super::split_image_reference;

    #[test]
    fn splits_a_plain_tag() {
        assert_eq!(
            split_image_reference("python:3.12-slim"),
            ("python".to_string(), "3.12-slim".to_string())
        );
    }

    #[test]
    fn defaults_to_latest_when_no_tag_is_given() {
        assert_eq!(
            split_image_reference("alpine"),
            ("alpine".to_string(), "latest".to_string())
        );
    }

    #[test]
    fn keeps_namespaced_images_intact() {
        assert_eq!(
            split_image_reference("ghcr.io/owner/image:v2"),
            ("ghcr.io/owner/image".to_string(), "v2".to_string())
        );
    }

    #[test]
    fn a_registry_port_is_not_a_tag() {
        // The colon here belongs to the port, not to a tag.
        assert_eq!(
            split_image_reference("registry.local:5000/team/app"),
            (
                "registry.local:5000/team/app".to_string(),
                "latest".to_string()
            )
        );
    }

    #[test]
    fn a_digest_is_not_a_tag() {
        let digest = "python@sha256:0123456789abcdef";
        assert_eq!(
            split_image_reference(digest),
            (digest.to_string(), String::new())
        );
    }
}
