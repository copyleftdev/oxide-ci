use crate::{Plugin, PluginCallInput, PluginCallOutput};
use oxide_core::Result;
use std::process::Command;
use tracing::info;

pub struct GitCheckoutPlugin;

impl Default for GitCheckoutPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl GitCheckoutPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for GitCheckoutPlugin {
    fn name(&self) -> &str {
        "git-checkout"
    }

    fn execute(&self, input: &PluginCallInput) -> Result<PluginCallOutput> {
        let repo = input
            .params
            .get("repository")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oxide_core::Error::Internal("Missing 'repository' input".into()))?;

        let ref_name = input
            .params
            .get("ref")
            .and_then(|v| v.as_str())
            .unwrap_or("main");

        let path = input
            .params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        info!("Checking out {}@{} to {}", repo, ref_name, path);

        // Run git clone/checkout logic
        // For simplicity, we'll use Command to run git
        // 1. git clone
        let status = Command::new("git")
            .args(["clone", repo, path])
            .current_dir(&input.workspace)
            .status()
            .map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to execute git clone: {}", e))
            })?;

        if !status.success() {
            return Ok(PluginCallOutput::failure("git clone failed"));
        }

        // 2. git checkout if ref is specified (and not just cloned default)
        if ref_name != "main" && ref_name != "master" {
            let status = Command::new("git")
                .args(["checkout", ref_name])
                .current_dir(std::path::Path::new(&input.workspace).join(path))
                .status()
                .map_err(|e| {
                    oxide_core::Error::Internal(format!("Failed to execute git checkout: {}", e))
                })?;

            if !status.success() {
                return Ok(PluginCallOutput::failure("git checkout failed"));
            }
        }

        Ok(PluginCallOutput::success())
    }
}
