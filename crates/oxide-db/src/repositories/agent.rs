//! PostgreSQL implementation of AgentRepository.

use async_trait::async_trait;
use oxide_core::agent::{Agent, AgentStatus, Arch, Capability, Os, SystemMetrics};
use oxide_core::ids::AgentId;
use oxide_core::ports::AgentRepository;
use oxide_core::{Error, Result};
use sqlx::{PgPool, Row};

pub struct PgAgentRepository {
    pool: PgPool,
}

impl PgAgentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn status_to_str(status: &AgentStatus) -> &'static str {
        match status {
            AgentStatus::Idle => "idle",
            AgentStatus::Busy => "busy",
            AgentStatus::Draining => "draining",
            AgentStatus::Offline => "offline",
        }
    }

    fn str_to_status(s: &str) -> AgentStatus {
        match s {
            "idle" => AgentStatus::Idle,
            "busy" => AgentStatus::Busy,
            "draining" => AgentStatus::Draining,
            _ => AgentStatus::Offline,
        }
    }

    fn os_to_str(os: &Os) -> &'static str {
        match os {
            Os::Linux => "linux",
            Os::Macos => "macos",
            Os::Windows => "windows",
        }
    }

    fn str_to_os(s: &str) -> Os {
        match s {
            "macos" => Os::Macos,
            "windows" => Os::Windows,
            _ => Os::Linux,
        }
    }

    fn arch_to_str(arch: &Arch) -> &'static str {
        match arch {
            Arch::X86_64 => "amd64",
            Arch::Aarch64 => "arm64",
        }
    }

    fn str_to_arch(s: &str) -> Arch {
        match s {
            "arm64" => Arch::Aarch64,
            _ => Arch::X86_64,
        }
    }

    fn row_to_agent(&self, r: &sqlx::postgres::PgRow) -> Result<Agent> {
        let capabilities: Vec<Capability> = serde_json::from_value(r.get("capabilities"))
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let system_metrics: Option<SystemMetrics> = r
            .get::<Option<serde_json::Value>, _>("system_metrics")
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let os_str: String = r.get("os");
        let arch_str: String = r.get("arch");
        let status_str: String = r.get("status");
        Ok(Agent {
            id: AgentId::from_uuid(r.get::<uuid::Uuid, _>("id")),
            name: r.get("name"),
            labels: r.get("labels"),
            version: r.get("version"),
            os: Self::str_to_os(&os_str),
            arch: Self::str_to_arch(&arch_str),
            capabilities,
            max_concurrent_jobs: r.get::<i32, _>("max_concurrent_jobs") as u32,
            status: Self::str_to_status(&status_str),
            current_run_id: r
                .get::<Option<uuid::Uuid>, _>("current_run_id")
                .map(oxide_core::ids::RunId::from_uuid),
            system_metrics,
            registered_at: r.get("registered_at"),
            last_heartbeat_at: r.get("last_heartbeat_at"),
        })
    }
}

#[async_trait]
impl AgentRepository for PgAgentRepository {
    async fn register(&self, agent: &Agent) -> Result<AgentId> {
        let capabilities_json = serde_json::to_value(&agent.capabilities)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let metrics_json = agent
            .system_metrics
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| Error::Serialization(e.to_string()))?;
        sqlx::query("INSERT INTO agents (id, name, labels, version, os, arch, capabilities, max_concurrent_jobs, status, current_run_id, system_metrics, registered_at, last_heartbeat_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) ON CONFLICT (name) DO UPDATE SET labels = EXCLUDED.labels, version = EXCLUDED.version, capabilities = EXCLUDED.capabilities, max_concurrent_jobs = EXCLUDED.max_concurrent_jobs, status = EXCLUDED.status, system_metrics = EXCLUDED.system_metrics, last_heartbeat_at = EXCLUDED.last_heartbeat_at, updated_at = NOW()")
            .bind(agent.id.as_uuid())
            .bind(&agent.name)
            .bind(&agent.labels)
            .bind(&agent.version)
            .bind(Self::os_to_str(&agent.os))
            .bind(Self::arch_to_str(&agent.arch))
            .bind(&capabilities_json)
            .bind(agent.max_concurrent_jobs as i32)
            .bind(Self::status_to_str(&agent.status))
            .bind(agent.current_run_id.map(|run_id| *run_id.as_uuid()))
            .bind(&metrics_json)
            .bind(agent.registered_at)
            .bind(agent.last_heartbeat_at)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(agent.id)
    }

    async fn get(&self, id: AgentId) -> Result<Option<Agent>> {
        let row = sqlx::query("SELECT id, name, labels, version, os, arch, capabilities, max_concurrent_jobs, status, current_run_id, system_metrics, registered_at, last_heartbeat_at FROM agents WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        match row {
            Some(r) => Ok(Some(self.row_to_agent(&r)?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<Agent>> {
        let rows = sqlx::query("SELECT id, name, labels, version, os, arch, capabilities, max_concurrent_jobs, status, current_run_id, system_metrics, registered_at, last_heartbeat_at FROM agents ORDER BY registered_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        rows.iter().map(|r| self.row_to_agent(r)).collect()
    }

    async fn list_available(&self, labels: &[String]) -> Result<Vec<Agent>> {
        let rows = sqlx::query("SELECT id, name, labels, version, os, arch, capabilities, max_concurrent_jobs, status, current_run_id, system_metrics, registered_at, last_heartbeat_at FROM agents WHERE status = 'idle' AND labels @> $1")
            .bind(labels)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        rows.iter().map(|r| self.row_to_agent(r)).collect()
    }

    async fn update(&self, agent: &Agent) -> Result<()> {
        let capabilities_json = serde_json::to_value(&agent.capabilities)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let metrics_json = agent
            .system_metrics
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| Error::Serialization(e.to_string()))?;
        sqlx::query("UPDATE agents SET labels = $2, version = $3, capabilities = $4, max_concurrent_jobs = $5, status = $6, current_run_id = $7, system_metrics = $8, last_heartbeat_at = $9, updated_at = NOW() WHERE id = $1")
            .bind(agent.id.as_uuid())
            .bind(&agent.labels)
            .bind(&agent.version)
            .bind(&capabilities_json)
            .bind(agent.max_concurrent_jobs as i32)
            .bind(Self::status_to_str(&agent.status))
            .bind(agent.current_run_id.map(|run_id| *run_id.as_uuid()))
            .bind(&metrics_json)
            .bind(agent.last_heartbeat_at)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn heartbeat(&self, id: AgentId) -> Result<()> {
        sqlx::query(
            "UPDATE agents SET last_heartbeat_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn deregister(&self, id: AgentId) -> Result<()> {
        sqlx::query("DELETE FROM agents WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_stale(&self, threshold_seconds: u64) -> Result<Vec<Agent>> {
        let threshold = chrono::Utc::now() - chrono::Duration::seconds(threshold_seconds as i64);
        let rows = sqlx::query("SELECT id, name, labels, version, os, arch, capabilities, max_concurrent_jobs, status, current_run_id, system_metrics, registered_at, last_heartbeat_at FROM agents WHERE last_heartbeat_at < $1 AND status != 'offline'")
            .bind(threshold)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        rows.iter().map(|r| self.row_to_agent(r)).collect()
    }
}
