//! Cache types and requests.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Request to restore a cache entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheRestoreRequest {
    /// Primary cache key.
    pub key: String,
    /// Fallback keys to try if primary misses.
    #[serde(default)]
    pub restore_keys: Vec<String>,
    /// Paths to restore to.
    pub paths: Vec<PathBuf>,
    /// Scope for cache isolation (e.g., pipeline ID).
    pub scope: Option<String>,
}

/// Request to save a cache entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSaveRequest {
    /// Cache key.
    pub key: String,
    /// Paths to cache.
    pub paths: Vec<PathBuf>,
    /// Time-to-live in seconds.
    pub ttl_seconds: Option<u64>,
    /// Scope for cache isolation.
    pub scope: Option<String>,
    /// Compression algorithm.
    #[serde(default)]
    pub compression: CompressionType,
}

/// Compression algorithm.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    None,
    #[default]
    Zstd,
    Gzip,
    Lz4,
}

/// A cached entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Cache key.
    pub key: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// When the entry was created.
    pub created_at: DateTime<Utc>,
    /// When the entry expires.
    pub expires_at: Option<DateTime<Utc>>,
    /// Compression used.
    pub compression: CompressionType,
    /// Checksum of the content.
    pub checksum: String,
}

/// Result of a cache restore operation.
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// The matched cache entry, if any.
    pub entry: Option<CacheEntry>,
    /// The key that matched (may be a restore key).
    pub matched_key: Option<String>,
    /// Whether it was an exact match.
    pub exact_match: bool,
    /// Time taken to restore in milliseconds.
    pub duration_ms: u64,
}

/// Result of a cache save operation.
#[derive(Debug, Clone)]
pub struct SaveResult {
    /// The saved cache entry.
    pub entry: CacheEntry,
    /// Time taken to save in milliseconds.
    pub duration_ms: u64,
}

/// Cache statistics.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub uploads: u64,
    pub total_bytes_downloaded: u64,
    pub total_bytes_uploaded: u64,
}
