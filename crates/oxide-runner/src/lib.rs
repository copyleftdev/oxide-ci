//! Step execution engine for Oxide CI.

pub mod container;
pub mod environments;
pub mod firecracker;
pub mod nix;
pub mod runner;
pub mod shell;

pub use container::ContainerRunner;
pub use environments::{ContainerConfig, Environment, EnvironmentFactory, HostEnvironment};
pub use firecracker::{FirecrackerConfig, FirecrackerEnvironment, TapDevice, VmState};
pub use nix::{BinaryCacheConfig, NixConfig, NixEnvironment};
pub use runner::{OutputLine, OutputStream, RunnerConfig, StepContext, StepResult, StepRunner};
pub use shell::ShellRunner;
