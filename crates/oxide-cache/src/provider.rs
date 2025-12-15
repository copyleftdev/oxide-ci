//! Cache storage provider trait and implementations.

use crate::types::{CacheEntry, CacheRestoreRequest, CacheSaveRequest, RestoreResult, SaveResult};
use async_trait::async_trait;
use oxide_core::Result;
use std::path::PathBuf;

/// Trait for cache storage backends.
#[async_trait]
pub trait CacheProvider: Send + Sync {
    /// Restore a cache entry.
    async fn restore(&self, request: &CacheRestoreRequest) -> Result<RestoreResult>;

    /// Save a cache entry.
    async fn save(&self, request: &CacheSaveRequest) -> Result<SaveResult>;

    /// Check if a key exists.
    async fn exists(&self, key: &str, scope: Option<&str>) -> Result<bool>;

    /// Delete a cache entry.
    async fn delete(&self, key: &str, scope: Option<&str>) -> Result<()>;

    /// List entries matching a prefix.
    async fn list(&self, prefix: &str, scope: Option<&str>) -> Result<Vec<CacheEntry>>;
}

/// Filesystem-based cache provider for local development.
pub struct FilesystemProvider {
    root_dir: PathBuf,
}

impl FilesystemProvider {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    fn key_path(&self, key: &str, scope: Option<&str>) -> PathBuf {
        let sanitized_key = key.replace(['/', '\\', ':'], "_");
        match scope {
            Some(s) => self.root_dir.join(s).join(&sanitized_key),
            None => self.root_dir.join(&sanitized_key),
        }
    }
}

#[async_trait]
impl CacheProvider for FilesystemProvider {
    async fn restore(&self, request: &CacheRestoreRequest) -> Result<RestoreResult> {
        let start = std::time::Instant::now();
        let scope = request.scope.as_deref();

        // Try exact key match first
        let key_path = self.key_path(&request.key, scope);
        if key_path.exists() {
            let metadata = tokio::fs::metadata(&key_path)
                .await
                .map_err(|e| oxide_core::Error::Internal(format!("Failed to read cache: {}", e)))?;

            let entry = CacheEntry {
                key: request.key.clone(),
                size_bytes: metadata.len(),
                created_at: chrono::Utc::now(),
                expires_at: None,
                compression: crate::types::CompressionType::Zstd,
                checksum: String::new(),
            };

            return Ok(RestoreResult {
                entry: Some(entry),
                matched_key: Some(request.key.clone()),
                exact_match: true,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        // Try restore keys
        for restore_key in &request.restore_keys {
            let entries = self.list(restore_key, scope).await?;
            if let Some(entry) = entries.first() {
                return Ok(RestoreResult {
                    entry: Some(entry.clone()),
                    matched_key: Some(entry.key.clone()),
                    exact_match: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }

        // Cache miss
        Ok(RestoreResult {
            entry: None,
            matched_key: None,
            exact_match: false,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn save(&self, request: &CacheSaveRequest) -> Result<SaveResult> {
        let start = std::time::Instant::now();
        let scope = request.scope.as_deref();
        let key_path = self.key_path(&request.key, scope);

        // Ensure parent directory exists
        if let Some(parent) = key_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to create cache dir: {}", e))
            })?;
        }

        // For now, just create a placeholder file
        // In a real implementation, this would tar and compress the paths
        tokio::fs::write(&key_path, b"cache_placeholder")
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to write cache: {}", e)))?;

        let metadata = tokio::fs::metadata(&key_path)
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to read cache: {}", e)))?;

        let entry = CacheEntry {
            key: request.key.clone(),
            size_bytes: metadata.len(),
            created_at: chrono::Utc::now(),
            expires_at: request
                .ttl_seconds
                .map(|ttl| chrono::Utc::now() + chrono::Duration::seconds(ttl as i64)),
            compression: request.compression,
            checksum: String::new(),
        };

        Ok(SaveResult {
            entry,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn exists(&self, key: &str, scope: Option<&str>) -> Result<bool> {
        let key_path = self.key_path(key, scope);
        Ok(key_path.exists())
    }

    async fn delete(&self, key: &str, scope: Option<&str>) -> Result<()> {
        let key_path = self.key_path(key, scope);
        if key_path.exists() {
            tokio::fs::remove_file(&key_path).await.map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to delete cache: {}", e))
            })?;
        }
        Ok(())
    }

    async fn list(&self, prefix: &str, scope: Option<&str>) -> Result<Vec<CacheEntry>> {
        let search_dir = match scope {
            Some(s) => self.root_dir.join(s),
            None => self.root_dir.clone(),
        };

        if !search_dir.exists() {
            return Ok(vec![]);
        }

        let mut entries = vec![];
        let sanitized_prefix = prefix.replace(['/', '\\', ':'], "_");

        let mut read_dir = tokio::fs::read_dir(&search_dir)
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to read cache dir: {}", e)))?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to read entry: {}", e)))?
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&sanitized_prefix) {
                let metadata = entry.metadata().await.map_err(|e| {
                    oxide_core::Error::Internal(format!("Failed to read metadata: {}", e))
                })?;

                entries.push(CacheEntry {
                    key: name,
                    size_bytes: metadata.len(),
                    created_at: chrono::Utc::now(),
                    expires_at: None,
                    compression: crate::types::CompressionType::Zstd,
                    checksum: String::new(),
                });
            }
        }

        // Sort by key (most recent first for prefix matches)
        entries.sort_by(|a, b| b.key.cmp(&a.key));

        Ok(entries)
    }
}

impl Default for FilesystemProvider {
    fn default() -> Self {
        Self::new(PathBuf::from("/var/oxide/cache"))
    }
}
