//! Plugin registry for fetching and managing plugins.

use crate::manifest::PluginManifest;
use oxide_core::Result;
use std::path::PathBuf;
use tracing::{debug, info};

/// Configuration for the plugin registry.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Base URL for the registry.
    pub url: String,
    /// Local cache directory.
    pub cache_dir: PathBuf,
    /// Authentication token.
    pub auth_token: Option<String>,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            url: "https://plugins.oxide.ci".to_string(),
            cache_dir: PathBuf::from("/var/oxide/plugins"),
            auth_token: None,
        }
    }
}

/// Plugin registry client.
pub struct PluginRegistry {
    config: RegistryConfig,
}

impl PluginRegistry {
    /// Create a new registry client.
    pub fn new(config: RegistryConfig) -> Self {
        Self { config }
    }

    /// Fetch a plugin from the registry.
    pub async fn fetch(&self, name: &str, version: Option<&str>) -> Result<PathBuf> {
        let version_str = version.unwrap_or("latest");
        info!(name = %name, version = %version_str, "Fetching plugin from registry");

        // Construct cache path
        let cache_name = format!("{}_{}.wasm", name.replace('/', "_"), version_str);
        let cache_path = self.config.cache_dir.join(&cache_name);

        // Check if already cached
        if cache_path.exists() {
            debug!(path = %cache_path.display(), "Plugin found in cache");
            return Ok(cache_path);
        }

        // Ensure cache directory exists
        tokio::fs::create_dir_all(&self.config.cache_dir)
            .await
            .map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to create cache dir: {}", e))
            })?;

        // Construct URL
        let url = format!("{}/{}/{}.wasm", self.config.url, name, version_str);
        debug!(url = %url, "Downloading plugin");

        // Fetch from registry
        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if let Some(token) = &self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to fetch plugin: {}", e)))?;

        if !response.status().is_success() {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(oxide_core::Error::PluginNotFound(name.to_string()));
            }
            return Err(oxide_core::Error::Internal(format!(
                "Registry returned error: {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await.map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to read plugin body: {}", e))
        })?;

        // Write to cache
        tokio::fs::write(&cache_path, bytes).await.map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to write plugin to cache: {}", e))
        })?;

        info!(path = %cache_path.display(), "Plugin downloaded and cached");
        Ok(cache_path)
    }

    /// Get plugin manifest from registry.
    pub async fn get_manifest(&self, name: &str, version: Option<&str>) -> Result<PluginManifest> {
        let version_str = version.unwrap_or("latest");
        debug!(name = %name, version = %version_str, "Fetching plugin manifest");

        // TODO: Actually fetch from registry
        Err(oxide_core::Error::PluginNotFound(name.to_string()))
    }

    /// List available versions of a plugin.
    pub async fn list_versions(&self, name: &str) -> Result<Vec<String>> {
        debug!(name = %name, "Listing plugin versions");

        // TODO: Actually fetch from registry
        Ok(vec![])
    }

    /// Check if a plugin exists in the registry.
    pub async fn exists(&self, name: &str, version: Option<&str>) -> Result<bool> {
        match self.get_manifest(name, version).await {
            Ok(_) => Ok(true),
            Err(oxide_core::Error::PluginNotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Clear local cache.
    pub async fn clear_cache(&self) -> Result<()> {
        if self.config.cache_dir.exists() {
            tokio::fs::remove_dir_all(&self.config.cache_dir)
                .await
                .map_err(|e| {
                    oxide_core::Error::Internal(format!("Failed to clear cache: {}", e))
                })?;
        }
        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new(RegistryConfig::default())
    }
}
