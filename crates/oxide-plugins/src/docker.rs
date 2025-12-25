use crate::{Plugin, PluginCallInput, PluginCallOutput};
use oxide_core::Result;
use std::process::Command;
use tracing::info;

pub struct DockerBuildPlugin;

impl Default for DockerBuildPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DockerBuildPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for DockerBuildPlugin {
    fn name(&self) -> &str {
        "docker-build"
    }

    fn execute(&self, input: &PluginCallInput) -> Result<PluginCallOutput> {
        let dockerfile = input
            .params
            .get("dockerfile")
            .and_then(|v| v.as_str())
            .unwrap_or("Dockerfile");

        let context = input
            .params
            .get("context")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let tags_val = input.params.get("tags");
        let tags: Vec<String> = if let Some(arr) = tags_val.and_then(|v| v.as_array()) {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else if let Some(s) = tags_val.and_then(|v| v.as_str()) {
            vec![s.to_string()]
        } else {
            vec![]
        };

        info!(
            "Building Docker image from {} in context {}",
            dockerfile, context
        );

        let mut cmd = Command::new("docker");
        cmd.arg("build").arg("-f").arg(dockerfile);

        for tag in tags {
            cmd.arg("-t").arg(tag);
        }

        cmd.arg(context);
        cmd.current_dir(&input.workspace);

        let status = cmd.status().map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to execute docker build: {}", e))
        })?;

        if status.success() {
            Ok(PluginCallOutput::success())
        } else {
            Ok(PluginCallOutput::failure("docker build failed"))
        }
    }
}
