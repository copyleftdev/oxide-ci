//! Pipeline scheduling and orchestration for Oxide CI.

pub mod agents;
pub mod dag;
pub mod matrix;
pub mod queue;
pub mod scheduler;
pub mod service;
pub mod triggers;

pub use agents::AgentMatcher;
pub use dag::{DagBuilder, DagError, DagNode, PipelineDag};
pub use matrix::{MatrixExpander, MatrixExpansion, MatrixJob};
pub use queue::{Priority, QueueManager, QueuedJob};
pub use scheduler::Scheduler;
pub use service::SchedulerService;
pub use triggers::{TriggerEvent, TriggerMatcher};
