//! Span creation for CI/CD operations.

use tracing::{span, Level, Span};

/// CI/CD span attributes following OpenTelemetry semantic conventions.
#[derive(Default)]
pub struct CiAttributes {
    pub pipeline_id: Option<String>,
    pub pipeline_name: Option<String>,
    pub run_id: Option<String>,
    pub run_number: Option<u32>,
    pub stage_name: Option<String>,
    pub step_name: Option<String>,
    pub step_plugin: Option<String>,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub repository: Option<String>,
    pub git_ref: Option<String>,
    pub git_sha: Option<String>,
    pub author: Option<String>,
}


impl CiAttributes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pipeline(mut self, id: impl Into<String>, name: impl Into<String>) -> Self {
        self.pipeline_id = Some(id.into());
        self.pipeline_name = Some(name.into());
        self
    }

    pub fn run(mut self, id: impl Into<String>, number: u32) -> Self {
        self.run_id = Some(id.into());
        self.run_number = Some(number);
        self
    }

    pub fn stage(mut self, name: impl Into<String>) -> Self {
        self.stage_name = Some(name.into());
        self
    }

    pub fn step(mut self, name: impl Into<String>) -> Self {
        self.step_name = Some(name.into());
        self
    }

    pub fn plugin(mut self, plugin: impl Into<String>) -> Self {
        self.step_plugin = Some(plugin.into());
        self
    }

    pub fn agent(mut self, id: impl Into<String>, name: impl Into<String>) -> Self {
        self.agent_id = Some(id.into());
        self.agent_name = Some(name.into());
        self
    }

    pub fn vcs(mut self, repo: impl Into<String>, git_ref: impl Into<String>, sha: impl Into<String>) -> Self {
        self.repository = Some(repo.into());
        self.git_ref = Some(git_ref.into());
        self.git_sha = Some(sha.into());
        self
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }
}

/// Create a span for a pipeline run.
pub fn run_span(attrs: &CiAttributes) -> Span {
    span!(
        Level::INFO,
        "run.execute",
        ci.pipeline.id = attrs.pipeline_id.as_deref().unwrap_or(""),
        ci.pipeline.name = attrs.pipeline_name.as_deref().unwrap_or(""),
        ci.run.id = attrs.run_id.as_deref().unwrap_or(""),
        ci.run.number = attrs.run_number.unwrap_or(0),
        vcs.repository = attrs.repository.as_deref().unwrap_or(""),
        vcs.ref = attrs.git_ref.as_deref().unwrap_or(""),
        vcs.sha = attrs.git_sha.as_deref().unwrap_or(""),
    )
}

/// Create a span for a stage.
pub fn stage_span(attrs: &CiAttributes) -> Span {
    span!(
        Level::INFO,
        "stage.execute",
        ci.pipeline.id = attrs.pipeline_id.as_deref().unwrap_or(""),
        ci.run.id = attrs.run_id.as_deref().unwrap_or(""),
        ci.stage.name = attrs.stage_name.as_deref().unwrap_or(""),
    )
}

/// Create a span for a step.
pub fn step_span(attrs: &CiAttributes) -> Span {
    span!(
        Level::INFO,
        "step.execute",
        ci.pipeline.id = attrs.pipeline_id.as_deref().unwrap_or(""),
        ci.run.id = attrs.run_id.as_deref().unwrap_or(""),
        ci.stage.name = attrs.stage_name.as_deref().unwrap_or(""),
        ci.step.name = attrs.step_name.as_deref().unwrap_or(""),
        ci.step.plugin = attrs.step_plugin.as_deref().unwrap_or(""),
        ci.agent.id = attrs.agent_id.as_deref().unwrap_or(""),
    )
}

/// Create a span for an agent operation.
pub fn agent_span(attrs: &CiAttributes, operation: &str) -> Span {
    span!(
        Level::INFO,
        "agent.operation",
        operation = operation,
        ci.agent.id = attrs.agent_id.as_deref().unwrap_or(""),
        ci.agent.name = attrs.agent_name.as_deref().unwrap_or(""),
    )
}

/// Create a span for cache operations.
pub fn cache_span(operation: &str, key: &str) -> Span {
    span!(
        Level::DEBUG,
        "cache.operation",
        operation = operation,
        cache.key = key,
    )
}

/// Create a span for secret access.
pub fn secret_span(secret_id: &str) -> Span {
    span!(
        Level::DEBUG,
        "secret.access",
        secret.id = secret_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_attributes_builder() {
        let attrs = CiAttributes::new()
            .pipeline("pip_123", "my-pipeline")
            .run("run_456", 42)
            .stage("build")
            .step("compile")
            .vcs("acme/app", "refs/heads/main", "abc123");

        assert_eq!(attrs.pipeline_id, Some("pip_123".to_string()));
        assert_eq!(attrs.run_number, Some(42));
        assert_eq!(attrs.stage_name, Some("build".to_string()));
    }
}
