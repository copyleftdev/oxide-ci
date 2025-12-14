//! Agent types.

use crate::ids::{AgentId, RunId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub labels: Vec<String>,
    pub version: Option<String>,
    pub os: Os,
    pub arch: Arch,
    pub capabilities: Vec<Capability>,
    pub max_concurrent_jobs: u32,
    pub status: AgentStatus,
    pub current_run_id: Option<RunId>,
    pub system_metrics: Option<SystemMetrics>,
    pub registered_at: DateTime<Utc>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Os {
    Linux,
    Macos,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Docker,
    Podman,
    Firecracker,
    Nix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Busy,
    Draining,
    Offline,
}

impl AgentStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, AgentStatus::Idle)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub load_average: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub name: String,
    pub labels: Vec<String>,
    pub version: String,
    pub os: Os,
    pub arch: Arch,
    pub capabilities: Vec<Capability>,
    pub max_concurrent_jobs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisconnectReason {
    Graceful,
    Timeout,
    Error,
}
