//! Distributed cache for Oxide CI (S3/R2 compatible).

pub mod compression;
pub mod keys;
pub mod provider;
pub mod types;
pub mod archiver;

pub use compression::{compress, decompress};
pub use keys::{generate_key, matches_prefix, sanitize_key};
pub use provider::{CacheProvider, FilesystemProvider};
pub use types::{
    CacheEntry, CacheRestoreRequest, CacheSaveRequest, CacheStats, CompressionType, RestoreResult,
    SaveResult,
};
