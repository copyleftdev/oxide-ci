//! Step execution engine for Oxide CI.

pub mod container;
pub mod environments;
pub mod runner;
pub mod shell;

pub use container::ContainerRunner;
pub use environments::{ContainerConfig, Environment, EnvironmentFactory, HostEnvironment};
pub use runner::{OutputLine, OutputStream, RunnerConfig, StepContext, StepResult, StepRunner};
pub use shell::ShellRunner;
