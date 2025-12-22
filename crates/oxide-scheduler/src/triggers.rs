//! Trigger matching and evaluation.

use oxide_core::pipeline::{PipelineDefinition, TriggerConfig, TriggerType};
use std::collections::HashMap;

/// Event that can trigger a pipeline.
#[derive(Debug, Clone)]
pub enum TriggerEvent {
    Push {
        branch: String,
        paths_changed: Vec<String>,
    },
    PullRequest {
        source_branch: String,
        target_branch: String,
        paths_changed: Vec<String>,
    },
    Tag {
        name: String,
    },
    Cron {
        schedule: String,
    },
    Manual {
        actor: Option<String>,
    },
    Api {
        source: Option<String>,
    },
    Webhook {
        event_type: String,
        payload: HashMap<String, serde_json::Value>,
    },
}

impl TriggerEvent {
    pub fn trigger_type(&self) -> TriggerType {
        match self {
            TriggerEvent::Push { .. } => TriggerType::Push,
            TriggerEvent::PullRequest { .. } => TriggerType::PullRequest,
            TriggerEvent::Tag { .. } => TriggerType::Push,
            TriggerEvent::Cron { .. } => TriggerType::Cron,
            TriggerEvent::Manual { .. } => TriggerType::Manual,
            TriggerEvent::Api { .. } => TriggerType::Api,
            TriggerEvent::Webhook { .. } => TriggerType::Webhook,
        }
    }
}

/// Matcher for determining if a pipeline should be triggered.
pub struct TriggerMatcher;

impl TriggerMatcher {
    pub fn new() -> Self {
        Self
    }

    /// Check if a pipeline should be triggered by an event.
    pub fn matches(&self, pipeline: &PipelineDefinition, event: &TriggerEvent) -> bool {
        if pipeline.triggers.is_empty() {
            // Default: trigger on push to any branch
            return matches!(event, TriggerEvent::Push { .. });
        }

        pipeline
            .triggers
            .iter()
            .any(|trigger| self.trigger_matches(trigger, event))
    }

    fn trigger_matches(&self, trigger: &TriggerConfig, event: &TriggerEvent) -> bool {
        match event {
            TriggerEvent::Push {
                branch,
                paths_changed,
            } => {
                if trigger.trigger_type() != TriggerType::Push {
                    return false;
                }
                self.branch_matches(trigger.branches(), branch)
                    && self.paths_match(trigger.paths(), trigger.paths_ignore(), paths_changed)
            }
            TriggerEvent::PullRequest {
                target_branch,
                paths_changed,
                ..
            } => {
                if trigger.trigger_type() != TriggerType::PullRequest {
                    return false;
                }
                self.branch_matches(trigger.branches(), target_branch)
                    && self.paths_match(trigger.paths(), trigger.paths_ignore(), paths_changed)
            }
            TriggerEvent::Tag { name } => {
                if trigger.trigger_type() != TriggerType::Push {
                    return false;
                }
                self.tag_matches(trigger.tags(), name)
            }
            TriggerEvent::Cron { schedule } => {
                trigger.trigger_type() == TriggerType::Cron
                    && trigger.cron() == Some(schedule.as_str())
            }
            TriggerEvent::Manual { .. } => trigger.trigger_type() == TriggerType::Manual,
            TriggerEvent::Api { .. } => trigger.trigger_type() == TriggerType::Api,
            TriggerEvent::Webhook { .. } => trigger.trigger_type() == TriggerType::Webhook,
        }
    }

    fn branch_matches(&self, patterns: &[String], branch: &str) -> bool {
        if patterns.is_empty() {
            return true; // Match all branches if no patterns specified
        }
        patterns.iter().any(|p| self.glob_match(p, branch))
    }

    fn tag_matches(&self, patterns: &[String], tag: &str) -> bool {
        if patterns.is_empty() {
            return false; // Don't match tags unless explicitly specified
        }
        patterns.iter().any(|p| self.glob_match(p, tag))
    }

    fn paths_match(&self, include: &[String], exclude: &[String], changed: &[String]) -> bool {
        if include.is_empty() && exclude.is_empty() {
            return true; // No path filtering
        }

        let included = if include.is_empty() {
            true
        } else {
            changed
                .iter()
                .any(|path| include.iter().any(|p| self.glob_match(p, path)))
        };

        let excluded = changed
            .iter()
            .all(|path| exclude.iter().any(|p| self.glob_match(p, path)));

        included && !excluded
    }

    fn glob_match(&self, pattern: &str, text: &str) -> bool {
        if pattern == "*" || pattern == "**" {
            return true;
        }
        if let Some(prefix) = pattern.strip_suffix("/**") {
            return text.starts_with(prefix);
        }
        if let Some(prefix) = pattern.strip_suffix("/*") {
            let prefix_slash = format!("{}/", prefix);
            if text.starts_with(&prefix_slash) {
                return !text[prefix_slash.len()..].contains('/');
            }
            return false;
        }
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                return text.starts_with(parts[0]) && text.ends_with(parts[1]);
            }
        }
        pattern == text
    }
}

impl Default for TriggerMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_match_exact() {
        let matcher = TriggerMatcher::new();
        assert!(matcher.branch_matches(&["main".to_string()], "main"));
        assert!(!matcher.branch_matches(&["main".to_string()], "develop"));
    }

    #[test]
    fn test_branch_match_glob() {
        let matcher = TriggerMatcher::new();
        assert!(matcher.branch_matches(&["feature/*".to_string()], "feature/foo"));
        assert!(matcher.branch_matches(&["release/**".to_string()], "release/v1/hotfix"));
    }

    #[test]
    fn test_empty_patterns_match_all() {
        let matcher = TriggerMatcher::new();
        assert!(matcher.branch_matches(&[], "any-branch"));
    }
}
