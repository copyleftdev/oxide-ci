//! License types and structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// License information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// License ID.
    pub id: String,
    /// License key.
    pub key: String,
    /// License status.
    pub status: LicenseStatus,
    /// Licensee name.
    pub name: String,
    /// Licensed features/entitlements.
    pub entitlements: Vec<Entitlement>,
    /// License metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Expiration date.
    pub expires_at: Option<DateTime<Utc>>,
    /// Created date.
    pub created_at: DateTime<Utc>,
    /// Last validated.
    pub validated_at: Option<DateTime<Utc>>,
}

/// License status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseStatus {
    Active,
    Inactive,
    Expired,
    Suspended,
    Banned,
}

/// Feature entitlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entitlement {
    /// Entitlement code.
    pub code: String,
    /// Human-readable name.
    pub name: Option<String>,
    /// Usage limit (if applicable).
    pub limit: Option<u64>,
    /// Current usage.
    #[serde(default)]
    pub usage: u64,
}

/// License validation result.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the license is valid.
    pub valid: bool,
    /// The license (if valid).
    pub license: Option<License>,
    /// Error message (if invalid).
    pub error: Option<String>,
    /// Validation timestamp.
    pub validated_at: DateTime<Utc>,
    /// Whether validation was done offline.
    pub offline: bool,
}

impl ValidationResult {
    pub fn success(license: License, offline: bool) -> Self {
        Self {
            valid: true,
            license: Some(license),
            error: None,
            validated_at: Utc::now(),
            offline,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            valid: false,
            license: None,
            error: Some(error.into()),
            validated_at: Utc::now(),
            offline: false,
        }
    }
}

/// Machine fingerprint for node-locked licenses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineFingerprint {
    /// Machine ID.
    pub id: String,
    /// Hostname.
    pub hostname: String,
    /// Platform.
    pub platform: String,
    /// CPU cores.
    pub cores: u32,
}

impl MachineFingerprint {
    /// Generate fingerprint for current machine.
    pub fn current() -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let platform = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();

        // Generate a deterministic ID from hostname + platform
        let mut hasher = DefaultHasher::new();
        hostname.hash(&mut hasher);
        platform.hash(&mut hasher);
        arch.hash(&mut hasher);
        let id = format!("{:016x}", hasher.finish());

        Self {
            id,
            hostname,
            platform: format!("{}-{}", platform, arch),
            cores: num_cpus::get() as u32,
        }
    }
}
