//! Agent configuration.

use oxide_core::agent::{Arch, Capability, Os};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent name (must be unique).
    pub name: String,
    /// Labels for job matching.
    #[serde(default)]
    pub labels: Vec<String>,
    /// NATS server URL.
    #[serde(default = "default_nats_url")]
    pub nats_url: String,
    /// Maximum concurrent jobs.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_jobs: u32,
    /// Workspace directory for job execution.
    #[serde(default = "default_workspace_dir")]
    pub workspace_dir: PathBuf,
    /// Heartbeat interval in seconds.
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,
    /// Capabilities this agent supports.
    #[serde(default)]
    pub capabilities: Vec<Capability>,
}

fn default_nats_url() -> String {
    "nats://localhost:4222".to_string()
}

fn default_max_concurrent() -> u32 {
    4
}

fn default_workspace_dir() -> PathBuf {
    PathBuf::from("/var/oxide/workspace")
}

fn default_heartbeat_interval() -> u64 {
    10
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "oxide-agent".to_string(),
            labels: vec![],
            nats_url: default_nats_url(),
            max_concurrent_jobs: default_max_concurrent(),
            workspace_dir: default_workspace_dir(),
            heartbeat_interval_secs: default_heartbeat_interval(),
            capabilities: vec![Capability::Docker],
        }
    }
}

impl AgentConfig {
    /// Load configuration from a YAML file.
    pub fn from_file(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        serde_yaml::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Detect the current OS.
    pub fn detect_os() -> Os {
        #[cfg(target_os = "linux")]
        return Os::Linux;
        #[cfg(target_os = "macos")]
        return Os::Macos;
        #[cfg(target_os = "windows")]
        return Os::Windows;
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return Os::Linux;
    }

    /// Detect the current architecture.
    pub fn detect_arch() -> Arch {
        #[cfg(target_arch = "x86_64")]
        return Arch::X86_64;
        #[cfg(target_arch = "aarch64")]
        return Arch::Aarch64;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        return Arch::X86_64;
    }
}
