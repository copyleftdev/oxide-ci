//! Test helper functions and utilities.

use oxide_api::{AppState, build_app};
use oxide_db::{Database, PgAgentRepository, PgPipelineRepository, PgRunRepository};
use oxide_nats::NatsEventBus;
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Start an API server for testing and return its address.
pub async fn start_test_server(
    db: Database,
    event_bus: NatsEventBus,
) -> anyhow::Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
    let state = Arc::new(AppState::new(
        Arc::new(PgPipelineRepository::new(db.pool().clone())),
        Arc::new(PgRunRepository::new(db.pool().clone())),
        Arc::new(PgAgentRepository::new(db.pool().clone())),
        Arc::new(MockApprovalRepository),
        Arc::new(event_bus),
    ));

    let app = build_app(state);
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Ok((addr, handle))
}

pub struct MockApprovalRepository;

#[async_trait::async_trait]
impl oxide_core::ports::ApprovalRepository for MockApprovalRepository {
    async fn create(&self, _gate: &oxide_core::approval::ApprovalGate) -> oxide_core::Result<()> {
        Ok(())
    }

    async fn get(
        &self,
        _id: oxide_core::ids::ApprovalGateId,
    ) -> oxide_core::Result<Option<oxide_core::approval::ApprovalGate>> {
        Ok(None)
    }

    async fn update(&self, _gate: &oxide_core::approval::ApprovalGate) -> oxide_core::Result<()> {
        Ok(())
    }

    async fn list(
        &self,
        _run_id: Option<oxide_core::ids::RunId>,
    ) -> oxide_core::Result<Vec<oxide_core::approval::ApprovalGate>> {
        Ok(vec![])
    }
}

/// Create an HTTP client for testing.
pub fn test_client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create test client")
}

/// API test client with base URL.
pub struct ApiTestClient {
    client: Client,
    base_url: String,
}

impl ApiTestClient {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            client: test_client(),
            base_url: format!("http://{}", addr),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn get(&self, path: &str) -> reqwest::Result<reqwest::Response> {
        self.client.get(self.url(path)).send().await
    }

    pub async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> reqwest::Result<reqwest::Response> {
        self.client.post(self.url(path)).json(body).send().await
    }

    pub async fn put<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> reqwest::Result<reqwest::Response> {
        self.client.put(self.url(path)).json(body).send().await
    }

    pub async fn delete(&self, path: &str) -> reqwest::Result<reqwest::Response> {
        self.client.delete(self.url(path)).send().await
    }

    /// Check health endpoint.
    pub async fn health(&self) -> anyhow::Result<bool> {
        let resp = self.get("/health").await?;
        Ok(resp.status().is_success())
    }
}

/// Wait for a condition with timeout.
pub async fn wait_for<F, Fut>(
    timeout: std::time::Duration,
    interval: std::time::Duration,
    mut condition: F,
) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        tokio::time::sleep(interval).await;
    }
    false
}

/// Assert that a future completes within a timeout.
pub async fn assert_completes_within<F, T>(future: F, timeout: std::time::Duration) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(timeout, future)
        .await
        .expect("Operation timed out")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wait_for_immediate() {
        let result = wait_for(
            std::time::Duration::from_secs(1),
            std::time::Duration::from_millis(10),
            || async { true },
        )
        .await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_wait_for_timeout() {
        let result = wait_for(
            std::time::Duration::from_millis(100),
            std::time::Duration::from_millis(10),
            || async { false },
        )
        .await;
        assert!(!result);
    }
}
