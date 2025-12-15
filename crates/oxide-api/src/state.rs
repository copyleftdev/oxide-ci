//! Application state shared across handlers.

use oxide_core::ports::{AgentRepository, EventBus, PipelineRepository, RunRepository};
use std::sync::Arc;

/// Application state shared across all handlers.
#[derive(Clone)]
pub struct AppState {
    pub pipelines: Arc<dyn PipelineRepository>,
    pub runs: Arc<dyn RunRepository>,
    pub agents: Arc<dyn AgentRepository>,
    pub event_bus: Arc<dyn EventBus>,
}

impl AppState {
    pub fn new(
        pipelines: Arc<dyn PipelineRepository>,
        runs: Arc<dyn RunRepository>,
        agents: Arc<dyn AgentRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            pipelines,
            runs,
            agents,
            event_bus,
        }
    }
}
