//! Built-in `docker-build` plugin.

use crate::{Plugin, PluginCallInput, PluginCallOutput};
use oxide_core::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;
use tracing::{info, warn};

/// Parameters this plugin understands. Anything else is reported to the user
/// rather than silently dropped.
const KNOWN_PARAMS: &[&str] = &["dockerfile", "file", "context", "tags", "cache_from"];

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

/// Read a string parameter, accepting several spellings of the same key.
fn param_str<'a>(params: &'a HashMap<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|k| params.get(*k).and_then(|v| v.as_str()))
}

/// Parse a parameter that may be a YAML list, a single string, or a block
/// scalar holding one entry per line.
///
/// A block scalar written with `|` keeps its trailing newline, so treating the
/// value as one opaque string passes `"image:tag\n"` to docker and fails with
/// "invalid argument". Each line is trimmed, blanks are dropped, and `#` lines
/// are treated as comments the way `docker/metadata-action` does.
fn parse_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .flat_map(|item| match item {
                Value::String(s) => split_lines(s),
                other => split_lines(&other.to_string()),
            })
            .collect(),
        Some(Value::String(s)) => split_lines(s),
        _ => vec![],
    }
}

fn split_lines(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(String::from)
        .collect()
}

impl Plugin for DockerBuildPlugin {
    fn name(&self) -> &str {
        "docker-build"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn execute(&self, input: &PluginCallInput) -> Result<PluginCallOutput> {
        // `file` is the GitHub Actions spelling; `dockerfile` is ours.
        let dockerfile = param_str(&input.params, &["dockerfile", "file"]).unwrap_or("Dockerfile");
        let context = param_str(&input.params, &["context"]).unwrap_or(".");

        let tags = parse_list(input.params.get("tags"));
        let cache_from = parse_list(input.params.get("cache_from"));

        for key in input.params.keys() {
            if !KNOWN_PARAMS.contains(&key.as_str()) {
                warn!(
                    param = %key,
                    "docker-build ignoring unsupported parameter (supported: {})",
                    KNOWN_PARAMS.join(", ")
                );
            }
        }

        info!(
            "Building Docker image from {} in context {}",
            dockerfile, context
        );

        let mut cmd = Command::new("docker");
        cmd.arg("build").arg("-f").arg(dockerfile);

        for tag in &tags {
            cmd.arg("-t").arg(tag);
        }

        for source in &cache_from {
            cmd.arg("--cache-from").arg(source);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_single_tag_string() {
        assert_eq!(parse_list(Some(&json!("app:latest"))), vec!["app:latest"]);
    }

    #[test]
    fn parses_yaml_list() {
        assert_eq!(
            parse_list(Some(&json!(["app:latest", "app:v1"]))),
            vec!["app:latest", "app:v1"]
        );
    }

    #[test]
    fn strips_trailing_newline_from_block_scalar() {
        // Regression: #50 comment — `tags: |` keeps the trailing newline and
        // docker rejected the argument.
        assert_eq!(
            parse_list(Some(&json!("docker.io/loxal/toolbox:latest\n"))),
            vec!["docker.io/loxal/toolbox:latest"]
        );
    }

    #[test]
    fn splits_block_scalar_into_one_tag_per_line() {
        let value = json!("registry/app:latest\nregistry/app:sha-abc123\n");
        assert_eq!(
            parse_list(Some(&value)),
            vec!["registry/app:latest", "registry/app:sha-abc123"]
        );
    }

    #[test]
    fn ignores_blank_and_commented_lines() {
        let value = json!("# ${{ REGISTRY }}/app:${{ sha }}\n\n  registry/app:latest  \n");
        assert_eq!(parse_list(Some(&value)), vec!["registry/app:latest"]);
    }

    #[test]
    fn missing_value_yields_no_entries() {
        assert!(parse_list(None).is_empty());
        assert!(parse_list(Some(&json!(null))).is_empty());
        assert!(parse_list(Some(&json!("   \n  \n"))).is_empty());
    }

    #[test]
    fn dockerfile_accepts_both_spellings() {
        let mut params = HashMap::new();
        params.insert("file".to_string(), json!("build/Dockerfile"));
        assert_eq!(
            param_str(&params, &["dockerfile", "file"]),
            Some("build/Dockerfile")
        );

        // Our own spelling wins when both are present.
        params.insert("dockerfile".to_string(), json!("Dockerfile.ci"));
        assert_eq!(
            param_str(&params, &["dockerfile", "file"]),
            Some("Dockerfile.ci")
        );
    }

    #[test]
    fn unknown_param_list_covers_every_read_key() {
        // Guards the warning from firing on parameters the plugin does use.
        for key in ["dockerfile", "file", "context", "tags", "cache_from"] {
            assert!(
                KNOWN_PARAMS.contains(&key),
                "{key} missing from KNOWN_PARAMS"
            );
        }
    }
}
