//! Multi-provider secret management for Oxide CI.

pub mod manager;
pub mod native;
pub mod providers;

pub use manager::{SecretManager, SecretManagerConfig};
pub use native::NativeProvider;
pub use providers::{EnvProvider, FileProvider, SecretProvider, SecretValue};
