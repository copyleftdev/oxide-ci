use crate::{Plugin, PluginCallInput, PluginCallOutput};
use oxide_cache::{
    CacheProvider, CacheRestoreRequest, CacheSaveRequest, CompressionType, FilesystemProvider,
};
use oxide_core::Result;
use std::path::PathBuf;
use tracing::info;

pub struct CachePlugin;

impl Default for CachePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CachePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for CachePlugin {
    fn name(&self) -> &str {
        "cache"
    }

    fn execute(&self, input: &PluginCallInput) -> Result<PluginCallOutput> {
        let key = input
            .params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oxide_core::Error::Internal("Missing 'key' input".into()))?;

        // Restore keys (optional)
        let restore_keys: Vec<String> = input
            .params
            .get("restore-keys")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let paths_val = input
            .params
            .get("paths")
            .ok_or_else(|| oxide_core::Error::Internal("Missing 'paths' input".into()))?;

        let paths: Vec<PathBuf> = if let Some(arr) = paths_val.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str().map(PathBuf::from))
                .collect()
        } else if let Some(s) = paths_val.as_str() {
            vec![PathBuf::from(s)]
        } else {
            return Ok(PluginCallOutput::failure("Invalid 'paths' format"));
        };

        // Cache provider (using default local FS provider for now)
        // In the future this could be injected or configured via Env
        let provider = FilesystemProvider::default();

        let method = input
            .params
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("restore");

        // Runtime for async provider calls
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to build runtime: {}", e)))?;

        match method {
            "restore" => {
                info!("Restoring cache key: {}", key);

                let req = CacheRestoreRequest {
                    key: key.to_string(),
                    restore_keys,
                    paths,
                    scope: None, // Could use pipeline ID if available in env
                    base_dir: Some(PathBuf::from(&input.workspace)),
                };

                let res = rt.block_on(provider.restore(&req))?;

                let mut out = PluginCallOutput::success();
                if res.matched_key.is_some() {
                    info!(
                        "Cache HIT: {}",
                        res.matched_key.as_deref().unwrap_or("unknown")
                    );
                    out.outputs
                        .insert("cache-hit".to_string(), "true".to_string());
                } else {
                    info!("Cache MISS: {}", key);
                    out.outputs
                        .insert("cache-hit".to_string(), "false".to_string());
                }
                Ok(out)
            }
            "save" => {
                info!("Saving cache key: {}", key);

                let req = CacheSaveRequest {
                    key: key.to_string(),
                    paths,
                    ttl_seconds: None, // Default TTL
                    scope: None,
                    base_dir: Some(PathBuf::from(&input.workspace)),
                    compression: CompressionType::Zstd,
                };

                rt.block_on(provider.save(&req))?;

                Ok(PluginCallOutput::success())
            }
            _ => Ok(PluginCallOutput::failure(format!(
                "Unknown method: {}",
                method
            ))),
        }
    }
}
