//! CLI configuration management.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// CLI configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliConfig {
    /// API server URL.
    #[serde(default = "default_api_url")]
    pub api_url: String,
    /// Authentication token.
    pub token: Option<String>,
    /// Default project.
    pub project: Option<String>,
    /// Output format.
    #[serde(default)]
    pub output_format: OutputFormat,
}

fn default_api_url() -> String {
    "http://localhost:8080".to_string()
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Yaml,
}

impl CliConfig {
    /// Load configuration from file.
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(serde_yaml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to file.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Get the configuration file path.
    pub fn config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let dirs = directories::ProjectDirs::from("ci", "oxide", "oxide-cli")
            .ok_or("Could not determine config directory")?;
        Ok(dirs.config_dir().join("config.yaml"))
    }

    /// Set a configuration value.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "api_url" => self.api_url = value.to_string(),
            "token" => self.token = Some(value.to_string()),
            "project" => self.project = Some(value.to_string()),
            "output_format" => {
                self.output_format = match value {
                    "table" => OutputFormat::Table,
                    "json" => OutputFormat::Json,
                    "yaml" => OutputFormat::Yaml,
                    _ => return Err(format!("Invalid output format: {}", value)),
                };
            }
            _ => return Err(format!("Unknown config key: {}", key)),
        }
        Ok(())
    }
}
