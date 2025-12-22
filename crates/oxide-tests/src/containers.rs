//! Testcontainer configurations for integration tests.

use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::{minio::MinIO, nats::Nats, postgres::Postgres};

/// PostgreSQL container for database tests.
pub struct PostgresContainer {
    #[allow(dead_code)] // Kept to maintain container lifetime
    container: ContainerAsync<Postgres>,
    connection_string: String,
}

impl PostgresContainer {
    pub async fn start() -> anyhow::Result<Self> {
        let container = Postgres::default().with_tag("16-alpine").start().await?;

        let host = container.get_host().await?;
        let port = container.get_host_port_ipv4(5432).await?;

        let connection_string = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);

        Ok(Self {
            container,
            connection_string,
        })
    }

    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }
}

/// NATS container with JetStream for event bus tests.
pub struct NatsContainer {
    #[allow(dead_code)] // Kept to maintain container lifetime
    container: ContainerAsync<Nats>,
    url: String,
}

impl NatsContainer {
    pub async fn start() -> anyhow::Result<Self> {
        let container = Nats::default()
            .with_tag("2.10-alpine")
            .with_cmd(["-js"]) // Enable JetStream
            .start()
            .await?;

        let host = container.get_host().await?;
        let port = container.get_host_port_ipv4(4222).await?;

        let url = format!("nats://{}:{}", host, port);

        Ok(Self { container, url })
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

/// MinIO container for cache/storage tests.
pub struct MinioContainer {
    #[allow(dead_code)] // Kept to maintain container lifetime
    container: ContainerAsync<MinIO>,
    endpoint: String,
    access_key: String,
    secret_key: String,
}

impl MinioContainer {
    pub async fn start() -> anyhow::Result<Self> {
        let container = MinIO::default().with_tag("latest").start().await?;

        let host = container.get_host().await?;
        let port = container.get_host_port_ipv4(9000).await?;

        let endpoint = format!("http://{}:{}", host, port);
        let access_key = "minioadmin".to_string();
        let secret_key = "minioadmin".to_string();

        Ok(Self {
            container,
            endpoint,
            access_key,
            secret_key,
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn access_key(&self) -> &str {
        &self.access_key
    }

    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires docker"]
    async fn test_postgres_container_starts() {
        let pg = PostgresContainer::start().await.unwrap();
        assert!(pg.connection_string().contains("postgres://"));
    }

    #[tokio::test]
    #[ignore = "requires docker"]
    async fn test_nats_container_starts() {
        let nats = NatsContainer::start().await.unwrap();
        assert!(nats.url().contains("nats://"));
    }

    #[tokio::test]
    #[ignore = "requires docker"]
    async fn test_minio_container_starts() {
        let minio = MinioContainer::start().await.unwrap();
        assert!(minio.endpoint().contains("http://"));
    }
}
