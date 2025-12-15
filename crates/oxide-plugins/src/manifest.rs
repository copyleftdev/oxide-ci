//! Plugin manifest and types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin manifest describing a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name (e.g., "oxide/checkout").
    pub name: String,
    /// Plugin version (semver).
    pub version: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// Plugin author.
    pub author: Option<String>,
    /// License identifier.
    pub license: Option<String>,
    /// Input parameters.
    #[serde(default)]
    pub inputs: Vec<PluginInput>,
    /// Output values.
    #[serde(default)]
    pub outputs: Vec<PluginOutput>,
    /// Required host functions.
    #[serde(default)]
    pub host_functions: Vec<String>,
}

/// Plugin input parameter definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInput {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

/// Plugin output value definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginOutput {
    pub name: String,
    pub description: Option<String>,
}

/// Input passed to plugin execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCallInput {
    /// Input parameters from step definition.
    pub params: HashMap<String, serde_json::Value>,
    /// Environment variables.
    pub env: HashMap<String, String>,
    /// Working directory path.
    pub workspace: String,
    /// Step name.
    pub step_name: String,
}

/// Output from plugin execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCallOutput {
    /// Whether execution succeeded.
    pub success: bool,
    /// Exit code (0 for success).
    pub exit_code: i32,
    /// Output values set by the plugin.
    #[serde(default)]
    pub outputs: HashMap<String, String>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Log messages.
    #[serde(default)]
    pub logs: Vec<LogEntry>,
}

impl PluginCallOutput {
    pub fn success() -> Self {
        Self {
            success: true,
            exit_code: 0,
            outputs: HashMap::new(),
            error: None,
            logs: vec![],
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            exit_code: 1,
            outputs: HashMap::new(),
            error: Some(error.into()),
            logs: vec![],
        }
    }
}

/// Log entry from plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Log level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Plugin reference with version.
#[derive(Debug, Clone)]
pub struct PluginRef {
    pub name: String,
    pub version: Option<String>,
}

impl PluginRef {
    /// Parse a plugin reference (e.g., "oxide/checkout@v1").
    pub fn parse(s: &str) -> Self {
        if let Some((name, version)) = s.split_once('@') {
            Self {
                name: name.to_string(),
                version: Some(version.to_string()),
            }
        } else {
            Self {
                name: s.to_string(),
                version: None,
            }
        }
    }

    /// Get the full reference string.
    pub fn full_name(&self) -> String {
        match &self.version {
            Some(v) => format!("{}@{}", self.name, v),
            None => self.name.clone(),
        }
    }
}
