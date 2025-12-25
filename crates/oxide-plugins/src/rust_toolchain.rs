use crate::{Plugin, PluginCallInput, PluginCallOutput};
use oxide_core::Result;
use std::process::Command;
use tracing::info;

pub struct RustToolchainPlugin;

impl Default for RustToolchainPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl RustToolchainPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for RustToolchainPlugin {
    fn name(&self) -> &str {
        "rust-toolchain"
    }

    fn execute(&self, input: &PluginCallInput) -> Result<PluginCallOutput> {
        // Parse inputs
        let toolchain = input
            .params
            .get("toolchain")
            .and_then(|v| v.as_str())
            .unwrap_or("stable");

        let components = input
            .params
            .get("components")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let targets = input
            .params
            .get("targets")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let profile = input
            .params
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        info!("Installing Rust toolchain: {}", toolchain);

        // check if rustup is installed
        let status = Command::new("rustup")
            .arg("--version")
            .output();

        if status.is_err() {
            return Ok(PluginCallOutput::failure("rustup not found in PATH"));
        }

        // Install toolchain
        let mut install_cmd = Command::new("rustup");
        install_cmd.args(["toolchain", "install", toolchain, "--profile", profile]);
        
        if !components.is_empty() {
            for component in components.split(',') {
                 install_cmd.arg("-c");
                 install_cmd.arg(component.trim());
            }
        }

        if !targets.is_empty() {
            for target in targets.split(',') {
                install_cmd.arg("-t");
                install_cmd.arg(target.trim());
            }
        }
        
        // Don't override if not asking? Actually actions/checkout usually overrides default.
        // We'll set it as override for the directory? Or just install.
        // "rust-toolchain" action usually sets it as default or override.
        // Let's just install for now.

        let output = install_cmd
            .output()
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to execute rustup install: {}", e)))?;

        if !output.status.success() {
             let stderr = String::from_utf8_lossy(&output.stderr);
             return Ok(PluginCallOutput::failure(&format!("rustup install failed: {}", stderr)));
        }

        // Set as default or override?
        // If we want to use it, we should probably set override for the workspace.
        let status = Command::new("rustup")
            .args(["override", "set", toolchain])
            .current_dir(&input.workspace)
            .status()
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to set override: {}", e)))?;

        if !status.success() {
             return Ok(PluginCallOutput::failure("rustup override set failed"));
        }
        
        // Report version
        let version_out = Command::new("cargo")
            .arg("--version")
            .current_dir(&input.workspace)
            .output()
            .ok();
            
        if let Some(out) = version_out {
             info!("Rust version: {}", String::from_utf8_lossy(&out.stdout).trim());
        }

        Ok(PluginCallOutput::success())
    }
}
