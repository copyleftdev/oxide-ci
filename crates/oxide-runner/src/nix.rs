//! Nix flakes execution environment.

use crate::environments::Environment;
use oxide_core::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Nix environment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NixConfig {
    pub flake: Option<String>,
    pub packages: Vec<String>,
    pub shell_hook: Option<String>,
    pub pure: bool,
    pub sandbox: bool,
    pub substituters: Vec<String>,
}

impl Default for NixConfig {
    fn default() -> Self {
        Self {
            flake: None,
            packages: vec![],
            shell_hook: None,
            pure: true,
            sandbox: true,
            substituters: vec!["https://cache.nixos.org".to_string()],
        }
    }
}

/// Nix execution environment.
pub struct NixEnvironment {
    workspace: PathBuf,
    config: NixConfig,
}

impl NixEnvironment {
    pub fn new(workspace: PathBuf, config: NixConfig) -> Self {
        Self { workspace, config }
    }

    /// Check if Nix is available on the system.
    pub async fn is_available() -> bool {
        Command::new("nix")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Build the nix command arguments.
    fn build_nix_args(&self) -> Vec<String> {
        let mut args = vec!["develop".to_string()];

        // Add flake reference
        if let Some(ref flake) = self.config.flake {
            args.push(flake.clone());
        } else {
            args.push(".".to_string());
        }

        // Pure mode
        if self.config.pure {
            args.push("--ignore-environment".to_string());
        }

        // Additional substituters
        if !self.config.substituters.is_empty() {
            let substituters = self.config.substituters.join(" ");
            args.push("--option".to_string());
            args.push("substituters".to_string());
            args.push(substituters);
        }

        args
    }

    /// Run a command in the Nix environment.
    pub async fn run_command(&self, command: &str) -> Result<std::process::Output> {
        let mut nix_args = self.build_nix_args();
        nix_args.push("--command".to_string());
        nix_args.push(command.to_string());

        debug!(args = ?nix_args, "Running nix develop");

        let output = Command::new("nix")
            .args(&nix_args)
            .current_dir(&self.workspace)
            .output()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to run nix: {}", e)))?;

        Ok(output)
    }

    /// Run a command with nix-shell for legacy support.
    pub async fn run_shell_command(&self, command: &str) -> Result<std::process::Output> {
        let mut args = vec![];

        // Add packages
        for pkg in &self.config.packages {
            args.push("-p".to_string());
            args.push(pkg.clone());
        }

        // Pure mode
        if self.config.pure {
            args.push("--pure".to_string());
        }

        args.push("--run".to_string());
        args.push(command.to_string());

        debug!(args = ?args, "Running nix-shell");

        let output = Command::new("nix-shell")
            .args(&args)
            .current_dir(&self.workspace)
            .output()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to run nix-shell: {}", e)))?;

        Ok(output)
    }

    /// Evaluate a flake attribute.
    pub async fn eval_flake(&self, attr: &str) -> Result<String> {
        let flake_ref = self.config.flake.as_deref().unwrap_or(".");
        let full_ref = format!("{}#{}", flake_ref, attr);

        let output = Command::new("nix")
            .args(["eval", "--json", &full_ref])
            .current_dir(&self.workspace)
            .output()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to eval flake: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(oxide_core::Error::Internal(format!(
                "Flake evaluation failed: {}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Build a flake output.
    pub async fn build_flake(&self, attr: Option<&str>) -> Result<PathBuf> {
        let flake_ref = self.config.flake.as_deref().unwrap_or(".");
        let full_ref = match attr {
            Some(a) => format!("{}#{}", flake_ref, a),
            None => flake_ref.to_string(),
        };

        let output = Command::new("nix")
            .args(["build", &full_ref, "--no-link", "--print-out-paths"])
            .current_dir(&self.workspace)
            .output()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to build flake: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(oxide_core::Error::Internal(format!(
                "Flake build failed: {}",
                stderr
            )));
        }

        let path = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        Ok(PathBuf::from(path))
    }
}

#[async_trait::async_trait]
impl Environment for NixEnvironment {
    async fn prepare(&self) -> Result<()> {
        info!(workspace = %self.workspace.display(), "Preparing Nix environment");

        // Ensure workspace exists
        tokio::fs::create_dir_all(&self.workspace)
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to create workspace: {}", e)))?;

        // Check if Nix is available
        if !Self::is_available().await {
            return Err(oxide_core::Error::Internal(
                "Nix is not available on this system".to_string(),
            ));
        }

        // If using a flake, ensure it exists or can be fetched
        if let Some(ref flake) = self.config.flake
            && !flake.starts_with("github:")
                && !flake.starts_with("git+")
                && !flake.starts_with("path:")
            {
                // Local flake reference
                let flake_path = if flake.starts_with('.') {
                    self.workspace.join(flake.trim_start_matches('.').trim_start_matches('/'))
                } else {
                    self.workspace.join(flake)
                };

                // Check for flake.nix
                let flake_file = if flake_path.is_dir() {
                    flake_path.join("flake.nix")
                } else {
                    self.workspace.join("flake.nix")
                };

                if !flake_file.exists() {
                    warn!(path = %flake_file.display(), "flake.nix not found");
                }
            }

        info!("Nix environment prepared");
        Ok(())
    }

    fn working_dir(&self) -> &PathBuf {
        &self.workspace
    }

    async fn cleanup(&self) -> Result<()> {
        info!(workspace = %self.workspace.display(), "Cleaning up Nix environment");
        // Nix environments are stateless, nothing to clean up
        Ok(())
    }
}

/// Binary cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryCacheConfig {
    pub url: String,
    pub public_key: Option<String>,
    pub priority: u32,
}

impl BinaryCacheConfig {
    pub fn nixos_cache() -> Self {
        Self {
            url: "https://cache.nixos.org".to_string(),
            public_key: Some("cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=".to_string()),
            priority: 40,
        }
    }

    /// Format as nix.conf substituters entry.
    pub fn to_substituter(&self) -> String {
        self.url.clone()
    }

    /// Format as nix.conf trusted-public-keys entry.
    pub fn to_trusted_key(&self) -> Option<String> {
        self.public_key.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nix_config_default() {
        let config = NixConfig::default();
        assert!(config.pure);
        assert!(config.sandbox);
        assert!(config.substituters.contains(&"https://cache.nixos.org".to_string()));
    }

    #[test]
    fn test_build_nix_args() {
        let config = NixConfig {
            flake: Some(".#devShell".to_string()),
            pure: true,
            ..Default::default()
        };
        let env = NixEnvironment::new(PathBuf::from("/tmp"), config);
        let args = env.build_nix_args();

        assert!(args.contains(&"develop".to_string()));
        assert!(args.contains(&".#devShell".to_string()));
        assert!(args.contains(&"--ignore-environment".to_string()));
    }

    #[test]
    fn test_binary_cache_config() {
        let cache = BinaryCacheConfig::nixos_cache();
        assert_eq!(cache.url, "https://cache.nixos.org");
        assert!(cache.public_key.is_some());
    }
}
