//! Firecracker MicroVM execution environment.

use crate::environments::Environment;
use oxide_core::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Firecracker VM configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirecrackerConfig {
    pub kernel: String,
    pub rootfs: String,
    pub vcpu_count: u32,
    pub memory_mb: u32,
    pub disk_size_gb: Option<u32>,
    pub network: bool,
    pub boot_timeout_seconds: u32,
    pub socket_path: Option<PathBuf>,
}

impl Default for FirecrackerConfig {
    fn default() -> Self {
        Self {
            kernel: "vmlinux".to_string(),
            rootfs: "rootfs.ext4".to_string(),
            vcpu_count: 2,
            memory_mb: 2048,
            disk_size_gb: Some(10),
            network: true,
            boot_timeout_seconds: 30,
            socket_path: None,
        }
    }
}

/// Firecracker VM state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    NotStarted,
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
}

/// Firecracker MicroVM execution environment.
pub struct FirecrackerEnvironment {
    workspace: PathBuf,
    config: FirecrackerConfig,
    vm_id: String,
    state: VmState,
    socket_path: PathBuf,
    ssh_port: Option<u16>,
}

impl FirecrackerEnvironment {
    pub fn new(workspace: PathBuf, config: FirecrackerConfig) -> Self {
        let vm_id = uuid::Uuid::new_v4().to_string();
        let socket_path = config
            .socket_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/firecracker-{}.sock", vm_id)));

        Self {
            workspace,
            config,
            vm_id,
            state: VmState::NotStarted,
            socket_path,
            ssh_port: None,
        }
    }

    /// Check if Firecracker is available on the system.
    pub async fn is_available() -> bool {
        Command::new("firecracker")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Get the current VM state.
    pub fn state(&self) -> VmState {
        self.state
    }

    /// Start the Firecracker VM.
    pub async fn start(&mut self) -> Result<()> {
        info!(vm_id = %self.vm_id, "Starting Firecracker VM");
        self.state = VmState::Starting;

        // Start firecracker process
        let mut cmd = Command::new("firecracker");
        cmd.arg("--api-sock")
            .arg(&self.socket_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let _child = cmd.spawn().map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to start Firecracker: {}", e))
        })?;

        // Wait for socket to be ready
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Configure the VM via API
        self.configure_vm().await?;

        // Start the VM
        self.start_instance().await?;

        self.state = VmState::Running;
        info!(vm_id = %self.vm_id, "Firecracker VM started");
        Ok(())
    }

    /// Configure the VM via Firecracker API.
    async fn configure_vm(&self) -> Result<()> {
        // Set boot source
        let boot_source = serde_json::json!({
            "kernel_image_path": self.config.kernel,
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
        });
        self.api_put("boot-source", &boot_source).await?;

        // Set machine config
        let machine_config = serde_json::json!({
            "vcpu_count": self.config.vcpu_count,
            "mem_size_mib": self.config.memory_mb
        });
        self.api_put("machine-config", &machine_config).await?;

        // Set root drive
        let rootfs = serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": self.config.rootfs,
            "is_root_device": true,
            "is_read_only": false
        });
        self.api_put("drives/rootfs", &rootfs).await?;

        // Configure network if enabled
        if self.config.network {
            let network = serde_json::json!({
                "iface_id": "eth0",
                "guest_mac": "AA:FC:00:00:00:01",
                "host_dev_name": format!("tap-{}", &self.vm_id[..8])
            });
            self.api_put("network-interfaces/eth0", &network).await?;
        }

        Ok(())
    }

    /// Start the VM instance.
    async fn start_instance(&self) -> Result<()> {
        let action = serde_json::json!({
            "action_type": "InstanceStart"
        });
        self.api_put("actions", &action).await?;

        // Wait for boot
        tokio::time::sleep(tokio::time::Duration::from_secs(
            self.config.boot_timeout_seconds as u64 / 2,
        ))
        .await;

        Ok(())
    }

    /// Make an API call to Firecracker.
    async fn api_put(&self, endpoint: &str, body: &serde_json::Value) -> Result<()> {
        let url = format!("http://localhost/{}", endpoint);

        // Use Unix socket transport
        debug!(endpoint = %endpoint, "Firecracker API PUT");

        // For now, use curl as a fallback since reqwest doesn't support Unix sockets directly
        let body_str = serde_json::to_string(body).map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to serialize request: {}", e))
        })?;

        let output = Command::new("curl")
            .args([
                "--unix-socket",
                self.socket_path.to_str().unwrap_or(""),
                "-X",
                "PUT",
                "-H",
                "Content-Type: application/json",
                "-d",
                &body_str,
                &url,
            ])
            .output()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("API call failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(oxide_core::Error::Internal(format!(
                "Firecracker API error: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Run a command in the VM via SSH.
    pub async fn run_command(&self, command: &str) -> Result<std::process::Output> {
        let ssh_port = self.ssh_port.unwrap_or(22);

        let output = Command::new("ssh")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "UserKnownHostsFile=/dev/null",
                "-p",
                &ssh_port.to_string(),
                "root@172.16.0.2", // Default guest IP
                command,
            ])
            .output()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("SSH command failed: {}", e)))?;

        Ok(output)
    }

    /// Stop the VM gracefully.
    pub async fn stop(&mut self) -> Result<()> {
        if self.state != VmState::Running {
            return Ok(());
        }

        info!(vm_id = %self.vm_id, "Stopping Firecracker VM");
        self.state = VmState::Stopping;

        // Send shutdown command via SSH
        let _ = self.run_command("poweroff").await;

        // Wait for graceful shutdown
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Force kill if still running
        let action = serde_json::json!({
            "action_type": "SendCtrlAltDel"
        });
        let _ = self.api_put("actions", &action).await;

        self.state = VmState::Stopped;
        info!(vm_id = %self.vm_id, "Firecracker VM stopped");
        Ok(())
    }

    /// Pause the VM.
    pub async fn pause(&mut self) -> Result<()> {
        if self.state != VmState::Running {
            return Err(oxide_core::Error::Internal("VM is not running".to_string()));
        }

        let action = serde_json::json!({
            "action_type": "Pause"
        });
        self.api_put("vm", &action).await?;
        self.state = VmState::Paused;
        Ok(())
    }

    /// Resume a paused VM.
    pub async fn resume(&mut self) -> Result<()> {
        if self.state != VmState::Paused {
            return Err(oxide_core::Error::Internal("VM is not paused".to_string()));
        }

        let action = serde_json::json!({
            "action_type": "Resume"
        });
        self.api_put("vm", &action).await?;
        self.state = VmState::Running;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Environment for FirecrackerEnvironment {
    async fn prepare(&self) -> Result<()> {
        info!(workspace = %self.workspace.display(), "Preparing Firecracker environment");

        // Ensure workspace exists
        tokio::fs::create_dir_all(&self.workspace)
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to create workspace: {}", e)))?;

        // Check if Firecracker is available
        if !Self::is_available().await {
            return Err(oxide_core::Error::Internal(
                "Firecracker is not available on this system".to_string(),
            ));
        }

        // Verify kernel and rootfs exist
        if !PathBuf::from(&self.config.kernel).exists() {
            warn!(kernel = %self.config.kernel, "Kernel image not found locally");
        }

        if !PathBuf::from(&self.config.rootfs).exists() {
            warn!(rootfs = %self.config.rootfs, "Root filesystem not found locally");
        }

        info!("Firecracker environment prepared");
        Ok(())
    }

    fn working_dir(&self) -> &PathBuf {
        &self.workspace
    }

    async fn cleanup(&self) -> Result<()> {
        info!(workspace = %self.workspace.display(), "Cleaning up Firecracker environment");

        // Remove socket file
        if self.socket_path.exists() {
            let _ = tokio::fs::remove_file(&self.socket_path).await;
        }

        Ok(())
    }
}

/// TAP network device manager.
pub struct TapDevice {
    name: String,
    ip: String,
    netmask: String,
}

impl TapDevice {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ip: "172.16.0.1".to_string(),
            netmask: "255.255.255.0".to_string(),
        }
    }

    /// Create the TAP device.
    pub async fn create(&self) -> Result<()> {
        // Create TAP device
        Command::new("ip")
            .args(["tuntap", "add", &self.name, "mode", "tap"])
            .status()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to create TAP: {}", e)))?;

        // Set IP address
        Command::new("ip")
            .args([
                "addr",
                "add",
                &format!("{}/{}", self.ip, self.netmask),
                "dev",
                &self.name,
            ])
            .status()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to set IP: {}", e)))?;

        // Bring up the device
        Command::new("ip")
            .args(["link", "set", &self.name, "up"])
            .status()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to bring up TAP: {}", e)))?;

        Ok(())
    }

    /// Delete the TAP device.
    pub async fn delete(&self) -> Result<()> {
        Command::new("ip")
            .args(["tuntap", "del", &self.name, "mode", "tap"])
            .status()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to delete TAP: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firecracker_config_default() {
        let config = FirecrackerConfig::default();
        assert_eq!(config.vcpu_count, 2);
        assert_eq!(config.memory_mb, 2048);
        assert!(config.network);
    }

    #[test]
    fn test_vm_state_transitions() {
        let config = FirecrackerConfig::default();
        let env = FirecrackerEnvironment::new(PathBuf::from("/tmp"), config);
        assert_eq!(env.state(), VmState::NotStarted);
    }
}
