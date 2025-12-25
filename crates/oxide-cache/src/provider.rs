//! Cache storage provider trait and implementations.

use crate::types::{CacheEntry, CacheRestoreRequest, CacheSaveRequest, CompressionType, RestoreResult, SaveResult};
use async_trait::async_trait;
use oxide_core::Result;
use std::path::PathBuf;
use sha2::{Sha256, Digest};
use std::io::Read;

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

/// Filesystem-based cache provider.
pub struct FilesystemProvider {
    root_dir: PathBuf,
}

impl FilesystemProvider {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    fn key_path(&self, key: &str, scope: Option<&str>) -> PathBuf {
        let sanitized_key = key.replace(['/', '\\', ':'], "_");
        let filename = format!("{}.tar.bin", sanitized_key);
        
        match scope {
            Some(s) => self.root_dir.join(s).join(&filename),
            None => self.root_dir.join(&filename),
        }
    }

    fn meta_path(&self, key_path: &std::path::Path) -> PathBuf {
        let mut meta = key_path.as_os_str().to_owned();
        meta.push(".meta");
        PathBuf::from(meta)
    }

    async fn read_meta(&self, key_path: &std::path::Path) -> Option<CacheEntry> {
        let meta_path = self.meta_path(key_path);
        if meta_path.exists() {
             match tokio::fs::read_to_string(&meta_path).await {
                 Ok(content) => serde_json::from_str(&content).ok(),
                 Err(_) => None,
             }
        } else {
            None
        }
    }
}

fn compute_checksum(path: &std::path::Path) -> oxide_core::Result<String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| oxide_core::Error::Internal(format!("Failed to open for checksum: {}", e)))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer)
            .map_err(|e| oxide_core::Error::Internal(format!("Failed to read for checksum: {}", e)))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[async_trait]
impl CacheProvider for FilesystemProvider {
    async fn restore(&self, request: &CacheRestoreRequest) -> Result<RestoreResult> {
        let start = std::time::Instant::now();
        let scope = request.scope.as_deref();
        
        // Determine base dir
        let base_dir = request.base_dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());

        // Try exact key match first
        let key_path = self.key_path(&request.key, scope);
        if key_path.exists() {
            // Verify checksum if meta exists
            let meta = self.read_meta(&key_path).await;
            if let Some(entry) = &meta {
                // Perform verification
                let path_clone = key_path.clone();
                let expected = entry.checksum.clone();
                if !expected.is_empty() {
                    let actual = tokio::task::spawn_blocking(move || compute_checksum(&path_clone)).await
                        .map_err(|e| oxide_core::Error::Internal(e.to_string()))??;
                    
                    if actual != expected {
                        return Err(oxide_core::Error::Internal(format!("Cache checksum mismatch for key {}. Expected {}, got {}", request.key, expected, actual)));
                    }
                }
            }

            let path_clone = key_path.clone();
            let base_dir_clone = base_dir.clone();
            
            // Perform restore in blocking thread
            tokio::task::spawn_blocking(move || {
                let file = std::fs::File::open(&path_clone)
                    .map_err(|e| oxide_core::Error::Internal(format!("Failed to open cache file: {}", e)))?;
                let reader = std::io::BufReader::new(file);
                crate::archiver::extract_archive(reader, &base_dir_clone, CompressionType::Zstd)
            }).await.map_err(|e| oxide_core::Error::Internal(e.to_string()))??;

            let metadata = tokio::fs::metadata(&key_path)
                .await
                .map_err(|e| oxide_core::Error::Internal(format!("Failed to read cache metadata: {}", e)))?;

            // Use meta if available, else construct
            let entry = meta.unwrap_or_else(|| CacheEntry {
                key: request.key.clone(),
                size_bytes: metadata.len(),
                created_at: chrono::Utc::now(),
                expires_at: None,
                compression: CompressionType::Zstd,
                checksum: String::new(), 
            });

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
                 let matched_path = self.key_path(&entry.key, scope);
                 
                 // Verify checksum
                 let meta = self.read_meta(&matched_path).await;
                 if let Some(meta_entry) = &meta {
                    let path_clone = matched_path.clone();
                    let expected = meta_entry.checksum.clone();
                    if !expected.is_empty() {
                        let actual = tokio::task::spawn_blocking(move || compute_checksum(&path_clone)).await
                             .map_err(|e| oxide_core::Error::Internal(e.to_string()))??;
                        if actual != expected {
                            // Warn? Or fail specific match?
                            // Let's skip this match if corrupted and try next?
                            // list() returned it, but validation failed.
                            // For safety, skip it.
                            continue;
                        }
                    }
                 }

                 if matched_path.exists() {
                     let path_clone = matched_path.clone();
                     let base_dir_clone = base_dir.clone();
                     
                     tokio::task::spawn_blocking(move || {
                        let file = std::fs::File::open(&path_clone)?;
                        let reader = std::io::BufReader::new(file);
                        crate::archiver::extract_archive(reader, &base_dir_clone, CompressionType::Zstd).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                     }).await.map_err(|e| oxide_core::Error::Internal(e.to_string()))?
                        .map_err(|e| oxide_core::Error::Internal(format!("Failed to restore backup match: {}", e)))?;

                     return Ok(RestoreResult {
                        entry: Some(entry.clone()),
                        matched_key: Some(entry.key.clone()),
                        exact_match: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                 }
             }
        }

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
        let base_dir = request.base_dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
        let request_paths = request.paths.clone();
        let compression = request.compression;

        if let Some(parent) = key_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to create cache dir: {}", e))
            })?;
        }
        
        let path_clone = key_path.clone();

        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::create(&path_clone)
                .map_err(|e| oxide_core::Error::Internal(format!("Failed to create cache file: {}", e)))?;
            let writer = std::io::BufWriter::new(file);

            crate::archiver::create_archive(writer, &request_paths, &base_dir, compression)
        }).await.map_err(|e| oxide_core::Error::Internal(e.to_string()))??;

        // Compute Checksum
        let path_clone_sum = key_path.clone();
        let checksum = tokio::task::spawn_blocking(move || compute_checksum(&path_clone_sum)).await
             .map_err(|e| oxide_core::Error::Internal(e.to_string()))??;

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
            compression,
            checksum,
        };

        // Save metadata
        let meta_path = self.meta_path(&key_path);
        let meta_json = serde_json::to_string(&entry).unwrap();
        tokio::fs::write(&meta_path, meta_json).await
             .map_err(|e| oxide_core::Error::Internal(format!("Failed to write metadata: {}", e)))?;


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
            // Also delete metadata
            let meta_path = self.meta_path(&key_path);
            if meta_path.exists() {
                let _ = tokio::fs::remove_file(&meta_path).await;
            }
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
            // Just look for .tar.bin, ignore .meta for main loop
            if name.starts_with(&sanitized_prefix) && name.ends_with(".tar.bin") {
                // Try reading metadata first
                let path = entry.path();
                
                // key from filename
                let key_str = name.strip_suffix(".tar.bin").unwrap().to_string();

                // Reconstruct self to reuse helper? No, we have path.
                // call read_meta manually?
                let mut meta_path = path.as_os_str().to_owned();
                meta_path.push(".meta");
                let meta_path = PathBuf::from(meta_path);
                
                let cache_entry = if meta_path.exists() {
                     if let Ok(content) = tokio::fs::read_to_string(&meta_path).await {
                         serde_json::from_str::<CacheEntry>(&content).ok()
                     } else {
                         None
                     }
                } else {
                    None
                };

                if let Some(e) = cache_entry {
                    entries.push(e);
                } else {
                    // Fallback to basic info from file
                     let metadata = entry.metadata().await.map_err(|e| {
                        oxide_core::Error::Internal(format!("Failed to read metadata: {}", e))
                    })?;
                    
                    entries.push(CacheEntry {
                        key: key_str,
                        size_bytes: metadata.len(),
                        created_at: chrono::Utc::now(), // Inaccurate
                        expires_at: None,
                        compression: CompressionType::Zstd,
                        checksum: String::new(),
                    });
                }
            }
        }

        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at)); // Now we have created_at if meta exists!

        Ok(entries)
    }
}

impl Default for FilesystemProvider {
    fn default() -> Self {
        if let Some(proj_dirs) = directories::ProjectDirs::from("io", "oxide", "oxide-ci") {
            Self::new(proj_dirs.cache_dir().into())
        } else {
            Self::new(PathBuf::from("/var/oxide/cache"))
        }
    }
}

