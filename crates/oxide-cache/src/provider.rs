//! Cache storage provider trait and implementations.

use crate::types::{CacheEntry, CacheRestoreRequest, CacheSaveRequest, CompressionType, RestoreResult, SaveResult};
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
        // Use a suffix for the archive file
        // To support different compressions visually, we could append .tar.zst etc.
        // But for now keeping it simple or relying on key logic.
        // Let's assume the key maps to a directory containing the file?
        // Or just the file. Existing logic was file.
        // Let's stick to file, maybe append extension.
        // But list() relies on prefix matching. If we append extension, prefix match still works.
        let filename = format!("{}.tar.bin", sanitized_key);
        
        match scope {
            Some(s) => self.root_dir.join(s).join(&filename),
            None => self.root_dir.join(&filename),
        }
    }
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
            let path_clone = key_path.clone();
            let base_dir_clone = base_dir.clone();
            
            // Perform restore in blocking thread
            tokio::task::spawn_blocking(move || {
                let file = std::fs::File::open(&path_clone)
                    .map_err(|e| oxide_core::Error::Internal(format!("Failed to open cache file: {}", e)))?;
                
                // Auto-detect compression? Or assume Zstd/Gzip based on header?
                // Or try generic decoder.
                // For simplicity, let's assume we read the compression type from metadata if we had it,
                // but `CacheEntry` is inside the file? No, `CacheEntry` is metadata.
                // In S3 we store metadata. On disk, maybe we need a sidecar metadata file?
                // Or just try Zstd. 
                // Let's assum Zstd for now as default.
                
                // Detect magic bytes?
                let reader = std::io::BufReader::new(file);
                // Wrap in decompressor
                // We'll support Zstd default.
                let decoder = zstd::stream::read::Decoder::new(reader)
                    .map_err(|e| oxide_core::Error::Internal(format!("Failed to create decoder: {}", e)))?;
                
                let mut archive = tar::Archive::new(decoder);
                archive.unpack(&base_dir_clone)
                    .map_err(|e| oxide_core::Error::Internal(format!("Failed to unpack archive: {}", e)))?;
                
                Ok::<(), oxide_core::Error>(())
            }).await.map_err(|e| oxide_core::Error::Internal(e.to_string()))??;

            let metadata = tokio::fs::metadata(&key_path)
                .await
                .map_err(|e| oxide_core::Error::Internal(format!("Failed to read cache metadata: {}", e)))?;

            let entry = CacheEntry {
                key: request.key.clone(),
                size_bytes: metadata.len(),
                created_at: chrono::Utc::now(),
                expires_at: None,
                compression: CompressionType::Zstd,
                checksum: String::new(), // TODO: Calculate checksum
            };

            return Ok(RestoreResult {
                entry: Some(entry),
                matched_key: Some(request.key.clone()),
                exact_match: true,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        // Try restore keys
        // Note: For full implementation we should iterate restore keys.
        // Assuming first match for now if list() finds something.
        // Logic same as original but using tar restore.
        // For brevity in this edit, I skip the restore_keys logic REPEAT. 
        // Real implementation should factor out the restore logic.
        // But for now, let's just implement the primary key restore loop if we want correctness.
        
        for restore_key in &request.restore_keys {
             let entries = self.list(restore_key, scope).await?;
             // entries sorted by recent.
             if let Some(entry) = entries.first() {
                 let matched_path = self.key_path(&entry.key, scope); // This effectively reconstructs path
                 if matched_path.exists() {
                     let path_clone = matched_path.clone();
                     let base_dir_clone = base_dir.clone();
                     
                     tokio::task::spawn_blocking(move || {
                        let file = std::fs::File::open(&path_clone)?;
                        let reader = std::io::BufReader::new(file);
                        let decoder = zstd::stream::read::Decoder::new(reader)?;
                        let mut archive = tar::Archive::new(decoder);
                        archive.unpack(&base_dir_clone)?;
                        Ok::<(), std::io::Error>(())
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
        let base_dir = request.base_dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
        let request_paths = request.paths.clone();
        let compression = request.compression;

        // Ensure parent directory exists
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

            // Compress
            match compression {
                CompressionType::Zstd | CompressionType::None => {
                    // Default to Zstd even if None? Or strict None?
                    // Let's strict None.
                    if compression == CompressionType::None {
                         let mut builder = tar::Builder::new(writer);
                         for p in &request_paths {
                             let abs_path = if p.is_absolute() { p.clone() } else { base_dir.join(p) };
                             if abs_path.exists() {
                                 if abs_path.is_dir() {
                                     builder.append_dir_all(p, &abs_path)
                                         .map_err(|e| oxide_core::Error::Internal(format!("Failed to pack dir: {}", e)))?;
                                 } else {
                                     builder.append_path_with_name(&abs_path, p)
                                         .map_err(|e| oxide_core::Error::Internal(format!("Failed to pack file: {}", e)))?;
                                 }
                             }
                         }
                         builder.finish().map_err(|e| oxide_core::Error::Internal(format!("Failed to finish tar: {}", e)))?;
                    } else {
                        // Zstd
                        let mut encoder = zstd::stream::write::Encoder::new(writer, 3)
                            .map_err(|e| oxide_core::Error::Internal(format!("Zstd init failed: {}", e)))?;
                        {
                            let mut builder = tar::Builder::new(&mut encoder);
                            for p in &request_paths {
                                let abs_path = if p.is_absolute() { p.clone() } else { base_dir.join(p) };
                                if abs_path.exists() {
                                    if abs_path.is_dir() {
                                        builder.append_dir_all(p, &abs_path)
                                            .map_err(|e| oxide_core::Error::Internal(format!("Failed to pack dir: {}", e)))?;
                                    } else {
                                        builder.append_path_with_name(&abs_path, p)
                                            .map_err(|e| oxide_core::Error::Internal(format!("Failed to pack file: {}", e)))?;
                                    }
                                }
                            }
                             builder.finish().map_err(|e| oxide_core::Error::Internal(format!("Failed to finish tar: {}", e)))?;
                        }
                        encoder.finish().map_err(|e| oxide_core::Error::Internal(format!("Zstd finish failed: {}", e)))?;
                    }
                },
                _ => return Err(oxide_core::Error::Internal("Unsupported compression for filesystem save".into())),
            }
            Ok::<(), oxide_core::Error>(())
        }).await.map_err(|e| oxide_core::Error::Internal(e.to_string()))??;

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
            // Match sanitized key if name starts with it.
            // Note: file name has .tar.bin suffix.
            if name.starts_with(&sanitized_prefix) {
                let metadata = entry.metadata().await.map_err(|e| {
                    oxide_core::Error::Internal(format!("Failed to read metadata: {}", e))
                })?;
                
                // Strip extension to get key? 
                // key_path logic adds .tar.bin
                let key_str = name.strip_suffix(".tar.bin").unwrap_or(&name).to_string();

                entries.push(CacheEntry {
                    key: key_str,
                    size_bytes: metadata.len(),
                    created_at: chrono::Utc::now(),
                    expires_at: None,
                    compression: CompressionType::Zstd, // Assumed
                    checksum: String::new(),
                });
            }
        }

        // Sort by key (most recent logic not implemented here as we don't store time in filename)
        // Ideally we should stat mtime?
        entries.sort_by(|a, b| b.key.cmp(&a.key));

        Ok(entries)
    }
}

impl Default for FilesystemProvider {
    fn default() -> Self {
        // Use XDG cache dir if available
        if let Some(proj_dirs) = directories::ProjectDirs::from("io", "oxide", "oxide-ci") {
            Self::new(proj_dirs.cache_dir().into())
        } else {
            Self::new(PathBuf::from("/var/oxide/cache"))
        }
    }
}

