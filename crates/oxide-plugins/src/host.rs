//! WASM plugin host using Extism.

use crate::manifest::{PluginCallInput, PluginCallOutput, PluginRef};
use dashmap::DashMap;
use extism::{Manifest, Plugin, Wasm};
use oxide_core::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Configuration for the plugin host.
#[derive(Debug, Clone)]
pub struct PluginHostConfig {
    /// Directory for plugin cache.
    pub cache_dir: PathBuf,
    /// Default timeout for plugin execution.
    pub default_timeout: Duration,
    /// Memory limit in bytes.
    pub memory_limit_bytes: Option<u64>,
    /// Whether to allow network access.
    pub allow_network: bool,
}

impl Default for PluginHostConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from("/var/oxide/plugins"),
            default_timeout: Duration::from_secs(300), // 5 minutes
            memory_limit_bytes: Some(256 * 1024 * 1024), // 256 MB
            allow_network: false,
        }
    }
}

/// WASM plugin host.
pub struct PluginHost {
    config: PluginHostConfig,
    plugins: DashMap<String, Arc<LoadedPlugin>>,
}

/// A loaded plugin ready for execution.
struct LoadedPlugin {
    #[allow(dead_code)]
    manifest_path: Option<PathBuf>,
    wasm_bytes: Vec<u8>,
}

impl PluginHost {
    /// Create a new plugin host.
    pub fn new(config: PluginHostConfig) -> Self {
        Self {
            config,
            plugins: DashMap::new(),
        }
    }

    /// Load a plugin from a file.
    pub async fn load_from_file(&self, path: &PathBuf) -> Result<String> {
        let wasm_bytes = tokio::fs::read(path)
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to read plugin: {}", e)))?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        info!(name = %name, path = %path.display(), "Loading plugin from file");

        let loaded = LoadedPlugin {
            manifest_path: None,
            wasm_bytes,
        };

        self.plugins.insert(name.clone(), Arc::new(loaded));
        Ok(name)
    }

    /// Load a plugin by reference.
    pub async fn load(&self, plugin_ref: &str) -> Result<()> {
        let pref = PluginRef::parse(plugin_ref);
        let full_name = pref.full_name();

        if self.plugins.contains_key(&full_name) {
            debug!(plugin = %full_name, "Plugin already loaded");
            return Ok(());
        }

        // Try to find plugin in cache directory
        let plugin_path = self
            .config
            .cache_dir
            .join(pref.name.replace('/', "_"))
            .with_extension("wasm");

        if plugin_path.exists() {
            self.load_from_file(&plugin_path).await?;
            return Ok(());
        }

        // Plugin not found
        Err(oxide_core::Error::PluginNotFound(pref.full_name()))
    }

    /// Execute a plugin.
    pub async fn call(
        &self,
        plugin_ref: &str,
        input: &PluginCallInput,
    ) -> Result<PluginCallOutput> {
        let pref = PluginRef::parse(plugin_ref);
        let full_name = pref.full_name();

        let loaded = self
            .plugins
            .get(&full_name)
            .or_else(|| self.plugins.get(&pref.name))
            .ok_or_else(|| oxide_core::Error::PluginNotFound(full_name.clone()))?;

        info!(plugin = %full_name, "Executing plugin");

        // Serialize input
        let input_json = serde_json::to_vec(input).map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to serialize input: {}", e))
        })?;

        // Clone needed data for the blocking task
        let wasm_bytes = loaded.wasm_bytes.clone();
        let timeout = self.config.default_timeout;
        let _allow_network = self.config.allow_network; // Reserved for when we configure WASI

        // Execute in blocking task to avoid stalling async runtime
        let output_bytes = tokio::task::spawn_blocking(move || {
            // Create Extism manifest
            let wasm = Wasm::data(wasm_bytes);
            let manifest = Manifest::new([wasm]).with_timeout(timeout);

            // Create plugin instance
            // Note: with_wasi(true) enables WASI. Check allow_network usage later.
            let mut plugin = Plugin::new(&manifest, [], true).map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to create plugin: {}", e))
            })?;

            // Call the "run" function
            plugin
                .call::<&[u8], Vec<u8>>("run", &input_json)
                .map_err(|e| oxide_core::Error::Internal(format!("Plugin execution failed: {}", e)))
        })
        .await
        .map_err(|e| oxide_core::Error::Internal(format!("Plugin task join error: {}", e)))??;

        // Deserialize output
        let output: PluginCallOutput = serde_json::from_slice(&output_bytes).map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to parse plugin output: {}", e))
        })?;

        if output.success {
            info!(plugin = %full_name, "Plugin completed successfully");
        } else {
            warn!(plugin = %full_name, error = ?output.error, "Plugin failed");
        }

        Ok(output)
    }

    /// Check if a plugin is loaded.
    pub fn is_loaded(&self, plugin_ref: &str) -> bool {
        let pref = PluginRef::parse(plugin_ref);
        self.plugins.contains_key(&pref.full_name()) || self.plugins.contains_key(&pref.name)
    }

    /// Unload a plugin.
    pub fn unload(&self, plugin_ref: &str) {
        let pref = PluginRef::parse(plugin_ref);
        self.plugins.remove(&pref.full_name());
        self.plugins.remove(&pref.name);
    }

    /// Get list of loaded plugins.
    pub fn loaded_plugins(&self) -> Vec<String> {
        self.plugins.iter().map(|e| e.key().clone()).collect()
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new(PluginHostConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_ref_parsing() {
        let pref = PluginRef::parse("oxide/checkout@v1");
        assert_eq!(pref.name, "oxide/checkout");
        assert_eq!(pref.version, Some("v1".to_string()));

        let pref = PluginRef::parse("my-plugin");
        assert_eq!(pref.name, "my-plugin");
        assert_eq!(pref.version, None);
    }
}
