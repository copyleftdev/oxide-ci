//! WASM plugin host for Oxide CI using Extism.

pub mod host;
pub mod manifest;
pub mod registry;

pub use host::{PluginHost, PluginHostConfig};
pub use manifest::{
    LogEntry, LogLevel, PluginCallInput, PluginCallOutput, PluginInput, PluginManifest,
    PluginOutput, PluginRef,
};
pub use registry::{PluginRegistry, RegistryConfig};
