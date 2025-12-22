use crate::{Plugin, PluginCallInput, PluginCallOutput};
use oxide_core::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub struct CachePlugin;

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
         let key = input.params.get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oxide_core::Error::Internal("Missing 'key' input".into()))?;
        
        let paths_val = input.params.get("paths")
            .ok_or_else(|| oxide_core::Error::Internal("Missing 'paths' input".into()))?;
            
        let paths: Vec<String> = if let Some(arr) = paths_val.as_array() {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        } else if let Some(s) = paths_val.as_str() {
            vec![s.to_string()]
        } else {
             return Ok(PluginCallOutput::failure("Invalid 'paths' format"));
        };

        // Cache storage location (mock implementation for now - normally would be S3/GCS or shared vol)
        let cache_base = PathBuf::from("/tmp/oxide-cache");
        let cache_dir = cache_base.join(key);

        // Determine action: restore or save? 
        // Typically in CI, cache restores at start, saves at end (post-step).
        // Since we don't have post-steps yet, we'll look for an explicit "action" input or infer?
        // Actually, typical usage:
        // - name: Restore cache
        //   uses: cache
        //   with:
        //     key: ...
        //     paths: ...
        // If cache exists -> restore. If not -> do nothing (and expectation is a post-step saves it).
        // BUT, since we are doing a simplified plugin system without post-steps yet:
        // Let's add an explicit 'method': 'save' or 'restore'. Default 'restore'.
        
        let method = input.params.get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("restore");

        match method {
            "restore" => {
                if cache_dir.exists() {
                     info!("Restoring cache key: {}", key);
                     for path_str in &paths {
                         let src = cache_dir.join(path_str);
                         let dest = Path::new(&input.workspace).join(path_str);
                         if src.exists() {
                            if let Some(parent) = dest.parent() {
                                fs::create_dir_all(parent).map_err(|e| oxide_core::Error::Internal(e.to_string()))?;
                            }
                             copy_dir_all(&src, &dest)?;
                         }
                     }
                     let mut out = PluginCallOutput::success();
                     out.outputs.insert("cache-hit".to_string(), "true".to_string());
                     Ok(out)
                } else {
                     info!("Cache key not found: {}", key);
                     let mut out = PluginCallOutput::success();
                     out.outputs.insert("cache-hit".to_string(), "false".to_string());
                     Ok(out)
                }
            }
            "save" => {
                info!("Saving cache key: {}", key);
                 fs::create_dir_all(&cache_dir).map_err(|e| oxide_core::Error::Internal(e.to_string()))?;
                 for path_str in &paths {
                     let src = Path::new(&input.workspace).join(path_str);
                     let dest = cache_dir.join(path_str);
                     if src.exists() {
                        if let Some(parent) = dest.parent() {
                            fs::create_dir_all(parent).map_err(|e| oxide_core::Error::Internal(e.to_string()))?;
                        }
                         copy_dir_all(&src, &dest)?;
                     } else {
                         warn!("Path not found for caching: {}", path_str);
                     }
                 }
                 Ok(PluginCallOutput::success())
            }
            _ => Ok(PluginCallOutput::failure(format!("Unknown method: {}", method))),
        }
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        if !dst.exists() {
             fs::create_dir_all(dst).map_err(|e| oxide_core::Error::Internal(e.to_string()))?;
        }
        for entry in fs::read_dir(src).map_err(|e| oxide_core::Error::Internal(e.to_string()))? {
            let entry = entry.map_err(|e| oxide_core::Error::Internal(e.to_string()))?;
            let ty = entry.file_type().map_err(|e| oxide_core::Error::Internal(e.to_string()))?;
            if ty.is_dir() {
                copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.join(entry.file_name())).map_err(|e| oxide_core::Error::Internal(e.to_string()))?;
            }
        }
    } else {
         fs::copy(src, dst).map_err(|e| oxide_core::Error::Internal(e.to_string()))?;
    }
    Ok(())
}
