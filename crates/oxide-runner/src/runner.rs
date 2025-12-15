//! Core runner trait and types.

use async_trait::async_trait;
use oxide_core::Result;
use oxide_core::pipeline::StepDefinition;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Output line from step execution.
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub stream: OutputStream,
    pub content: String,
    pub line_number: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Output stream type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Result of step execution.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub exit_code: i32,
    pub success: bool,
    pub duration_ms: u64,
    pub outputs: HashMap<String, String>,
}

/// Context for step execution.
#[derive(Debug, Clone)]
pub struct StepContext {
    pub workspace: PathBuf,
    pub variables: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
    pub step: StepDefinition,
}

/// Trait for step execution.
#[async_trait]
pub trait StepRunner: Send + Sync {
    /// Execute a step, streaming output to the provided channel.
    async fn execute(
        &self,
        ctx: &StepContext,
        output_tx: mpsc::Sender<OutputLine>,
    ) -> Result<StepResult>;

    /// Check if this runner can handle the given step.
    fn can_handle(&self, step: &StepDefinition) -> bool;
}

/// Configuration for step execution.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub timeout_seconds: Option<u64>,
    pub retry_count: u32,
    pub retry_delay_ms: u64,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(3600), // 1 hour default
            retry_count: 0,
            retry_delay_ms: 1000,
        }
    }
}
