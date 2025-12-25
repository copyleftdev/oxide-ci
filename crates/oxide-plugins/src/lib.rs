//! WASM plugin host for Oxide CI using Extism.

pub mod host;
pub mod manifest;
pub mod registry;

// New modules
pub mod cache;
pub mod docker;
pub mod git;
pub mod rust_toolchain;

pub use host::{PluginHost, PluginHostConfig};
pub use manifest::{
    LogEntry, LogLevel, PluginCallInput, PluginCallOutput, PluginInput, PluginManifest,
    PluginOutput, PluginRef,
};
pub use registry::{PluginRegistry, RegistryConfig};

use oxide_core::Result;

/// Trait for native plugins.
pub trait Plugin: Send + Sync {
    /// Get the plugin name.
    fn name(&self) -> &str;
    /// Execute the plugin.
    fn execute(&self, input: &PluginCallInput) -> Result<PluginCallOutput>;
}

/// Get a built-in plugin by name.
pub fn get_builtin_plugin(name: &str) -> Option<Box<dyn Plugin>> {
    match name {
        "git-checkout" | "oxide/checkout" => Some(Box::new(git::GitCheckoutPlugin::new())),
        "cache" | "oxide/cache" => Some(Box::new(cache::CachePlugin::new())),
        "docker-build" | "oxide/docker-build" => Some(Box::new(docker::DockerBuildPlugin::new())),
        "rust-toolchain" | "dtolnay/rust-toolchain" => Some(Box::new(rust_toolchain::RustToolchainPlugin::new())),
        _ => None,
    }
}
