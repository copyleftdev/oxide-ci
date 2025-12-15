//! Keygen license validation for Oxide CI.

pub mod keygen;
pub mod offline;
pub mod types;

pub use keygen::{KeygenClient, KeygenConfig};
pub use offline::{LicenseFile, OfflineValidator};
pub use types::{Entitlement, License, LicenseStatus, MachineFingerprint, ValidationResult};
