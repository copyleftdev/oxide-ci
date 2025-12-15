//! Agent matching for job assignment.

use oxide_core::Result;
use oxide_core::agent::{Agent, AgentStatus, Capability};
use oxide_core::pipeline::AgentSelector;
use oxide_core::ports::AgentRepository;
use std::sync::Arc;

/// Matcher for assigning jobs to agents.
pub struct AgentMatcher {
    repository: Arc<dyn AgentRepository>,
}

impl AgentMatcher {
    pub fn new(repository: Arc<dyn AgentRepository>) -> Self {
        Self { repository }
    }

    /// Find an available agent matching the given requirements.
    pub async fn find_available(
        &self,
        labels: &[String],
        capabilities: &[Capability],
    ) -> Result<Option<Agent>> {
        let agents = self.repository.list_available(labels).await?;

        for agent in agents {
            if self.matches_capabilities(&agent, capabilities) {
                return Ok(Some(agent));
            }
        }

        Ok(None)
    }

    /// Find the best agent for a job based on selector and current load.
    pub async fn find_best(
        &self,
        selector: Option<&AgentSelector>,
        capabilities: &[Capability],
    ) -> Result<Option<Agent>> {
        let labels = selector.map(|s| s.labels.clone()).unwrap_or_default();

        // If a specific agent name is requested
        if let Some(ref name) = selector.and_then(|s| s.name.clone()) {
            let agents = self.repository.list().await?;
            return Ok(agents.into_iter().find(|a| {
                a.name == *name
                    && a.status == AgentStatus::Idle
                    && self.matches_capabilities(a, capabilities)
            }));
        }

        // Find available agents with matching labels
        let available = self.repository.list_available(&labels).await?;

        // Filter by capabilities and sort by load (prefer idle agents)
        let mut candidates: Vec<_> = available
            .into_iter()
            .filter(|a| self.matches_capabilities(a, capabilities))
            .collect();

        // Sort by: idle first, then by fewest current jobs
        candidates.sort_by(|a, b| {
            let a_idle = a.status == AgentStatus::Idle;
            let b_idle = b.status == AgentStatus::Idle;
            b_idle.cmp(&a_idle)
        });

        Ok(candidates.into_iter().next())
    }

    /// Check if all required capabilities are satisfied.
    fn matches_capabilities(&self, agent: &Agent, required: &[Capability]) -> bool {
        required.iter().all(|cap| agent.capabilities.contains(cap))
    }

    /// Get agents that match specific labels.
    pub async fn find_by_labels(&self, labels: &[String]) -> Result<Vec<Agent>> {
        self.repository.list_available(labels).await
    }

    /// Check if any agent is available for the given requirements.
    pub async fn has_available(
        &self,
        labels: &[String],
        capabilities: &[Capability],
    ) -> Result<bool> {
        Ok(self.find_available(labels, capabilities).await?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use oxide_core::agent::{Arch, Os};
    use oxide_core::ids::AgentId;
    use std::sync::Mutex;

    struct MockAgentRepository {
        agents: Mutex<Vec<Agent>>,
    }

    #[async_trait]
    impl AgentRepository for MockAgentRepository {
        async fn register(&self, _agent: &Agent) -> Result<AgentId> {
            Ok(AgentId::default())
        }

        async fn get(&self, id: AgentId) -> Result<Option<Agent>> {
            Ok(self
                .agents
                .lock()
                .unwrap()
                .iter()
                .find(|a| a.id == id)
                .cloned())
        }

        async fn list(&self) -> Result<Vec<Agent>> {
            Ok(self.agents.lock().unwrap().clone())
        }

        async fn list_available(&self, labels: &[String]) -> Result<Vec<Agent>> {
            Ok(self
                .agents
                .lock()
                .unwrap()
                .iter()
                .filter(|a| {
                    a.status == AgentStatus::Idle
                        && (labels.is_empty() || labels.iter().all(|l| a.labels.contains(l)))
                })
                .cloned()
                .collect())
        }

        async fn update(&self, _agent: &Agent) -> Result<()> {
            Ok(())
        }

        async fn heartbeat(&self, _id: AgentId) -> Result<()> {
            Ok(())
        }

        async fn deregister(&self, _id: AgentId) -> Result<()> {
            Ok(())
        }

        async fn get_stale(&self, _threshold_seconds: u64) -> Result<Vec<Agent>> {
            Ok(vec![])
        }
    }

    fn make_agent(name: &str, labels: Vec<&str>, capabilities: Vec<Capability>) -> Agent {
        Agent {
            id: AgentId::default(),
            name: name.to_string(),
            labels: labels.iter().map(|s| s.to_string()).collect(),
            version: Some("1.0".to_string()),
            os: Os::Linux,
            arch: Arch::X86_64,
            capabilities,
            max_concurrent_jobs: 4,
            status: AgentStatus::Idle,
            current_run_id: None,
            system_metrics: None,
            registered_at: chrono::Utc::now(),
            last_heartbeat_at: Some(chrono::Utc::now()),
        }
    }

    #[tokio::test]
    async fn test_find_by_labels() {
        let repo = Arc::new(MockAgentRepository {
            agents: Mutex::new(vec![
                make_agent(
                    "linux-docker",
                    vec!["linux", "docker"],
                    vec![Capability::Docker],
                ),
                make_agent("linux-nix", vec!["linux", "nix"], vec![Capability::Nix]),
            ]),
        });

        let matcher = AgentMatcher::new(repo);

        let agents = matcher
            .find_by_labels(&["linux".to_string(), "docker".to_string()])
            .await
            .unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "linux-docker");
    }

    #[tokio::test]
    async fn test_find_by_capability() {
        let repo = Arc::new(MockAgentRepository {
            agents: Mutex::new(vec![
                make_agent("agent1", vec![], vec![Capability::Docker]),
                make_agent("agent2", vec![], vec![Capability::Nix]),
            ]),
        });

        let matcher = AgentMatcher::new(repo);

        let agent = matcher
            .find_available(&[], &[Capability::Docker])
            .await
            .unwrap();
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().name, "agent1");
    }
}
