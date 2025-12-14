//! Cache types.

use crate::ids::{CacheEntryId, RunId, StepId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub id: CacheEntryId,
    pub key: String,
    pub version: Option<String>,
    pub size_bytes: u64,
    pub compression: Compression,
    pub checksum_sha256: String,
    pub scope: CacheScope,
    pub storage_path: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_accessed_at: DateTime<Utc>,
    pub access_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compression {
    Gzip,
    Zstd,
    Lz4,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheScope {
    Pipeline,
    Project,
    Organization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheRestoreRequest {
    pub run_id: RunId,
    pub step_id: StepId,
    pub key: String,
    pub restore_keys: Vec<String>,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSaveRequest {
    pub run_id: RunId,
    pub step_id: StepId,
    pub key: String,
    pub paths: Vec<String>,
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheEvictionReason {
    Expired,
    Capacity,
    Manual,
    VersionChange,
}
