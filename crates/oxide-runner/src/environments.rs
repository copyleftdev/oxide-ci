//! Execution environment management.

use oxide_core::Result;
use oxide_core::pipeline::EnvironmentType;
use std::path::PathBuf;
use tracing::info;

/// Trait for execution environments.
#[async_trait::async_trait]
pub trait Environment: Send + Sync {
    /// Prepare the execution environment.
    async fn prepare(&self) -> Result<()>;

    /// Get the working directory.
    fn working_dir(&self) -> &PathBuf;

    /// Cleanup the execution environment.
    async fn cleanup(&self) -> Result<()>;
}

/// Host environment (runs directly on the agent).
pub struct HostEnvironment {
    workspace: PathBuf,
}

impl HostEnvironment {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait::async_trait]
impl Environment for HostEnvironment {
    async fn prepare(&self) -> Result<()> {
        info!(workspace = %self.workspace.display(), "Preparing host environment");
        tokio::fs::create_dir_all(&self.workspace)
            .await
            .map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to create workspace: {}", e))
            })?;
        Ok(())
    }

    fn working_dir(&self) -> &PathBuf {
        &self.workspace
    }

    async fn cleanup(&self) -> Result<()> {
        info!(workspace = %self.workspace.display(), "Cleaning up host environment");
        // Optionally remove workspace
        Ok(())
    }
}

/// Container environment configuration.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub image: String,
    pub registry: Option<String>,
    pub credentials: Option<RegistryCredentials>,
    pub volumes: Vec<VolumeMount>,
    pub network: Option<String>,
    pub privileged: bool,
}

#[derive(Debug, Clone)]
pub struct RegistryCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub host_path: PathBuf,
    pub container_path: String,
    pub read_only: bool,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            image: "alpine:latest".to_string(),
            registry: None,
            credentials: None,
            volumes: vec![],
            network: None,
            privileged: false,
        }
    }
}

/// Factory for creating execution environments.
pub struct EnvironmentFactory;

impl EnvironmentFactory {
    /// Create an environment based on the type.
    pub fn create(env_type: EnvironmentType, workspace: PathBuf) -> Box<dyn Environment> {
        match env_type {
            EnvironmentType::Host => Box::new(HostEnvironment::new(workspace)),
            EnvironmentType::Container => {
                // Container environment would require Docker client
                // For now, fall back to host
                Box::new(HostEnvironment::new(workspace))
            }
            EnvironmentType::Firecracker => {
                // Firecracker not yet implemented
                Box::new(HostEnvironment::new(workspace))
            }
            EnvironmentType::Nix => {
                // Nix environment not yet implemented
                Box::new(HostEnvironment::new(workspace))
            }
        }
    }
}
