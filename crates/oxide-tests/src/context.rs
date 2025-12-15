//! Test context providing access to all test infrastructure.

use crate::containers::{MinioContainer, NatsContainer, PostgresContainer};
use oxide_db::Database;
use oxide_nats::NatsEventBus;

/// Test context with all services running.
///
/// Drop this to stop all containers.
pub struct TestContext {
    pub postgres: PostgresContainer,
    pub nats: NatsContainer,
    pub minio: MinioContainer,
    pub db: Database,
    pub event_bus: NatsEventBus,
}

impl TestContext {
    /// Create a new test context with all containers running.
    pub async fn new() -> anyhow::Result<Self> {
        crate::init_test_logging();

        // Start containers in parallel
        let (postgres, nats, minio) = tokio::try_join!(
            PostgresContainer::start(),
            NatsContainer::start(),
            MinioContainer::start(),
        )?;

        // Connect to services
        let db = Database::connect(postgres.connection_string()).await?;
        db.migrate().await?;

        let event_bus = NatsEventBus::connect(nats.url()).await?;

        Ok(Self {
            postgres,
            nats,
            minio,
            db,
            event_bus,
        })
    }

    /// Create context with only PostgreSQL.
    pub async fn postgres_only() -> anyhow::Result<PostgresOnlyContext> {
        crate::init_test_logging();
        
        let postgres = PostgresContainer::start().await?;
        let db = Database::connect(postgres.connection_string()).await?;
        db.migrate().await?;

        Ok(PostgresOnlyContext { postgres, db })
    }

    /// Create context with only NATS.
    pub async fn nats_only() -> anyhow::Result<NatsOnlyContext> {
        crate::init_test_logging();
        
        let nats = NatsContainer::start().await?;
        let event_bus = NatsEventBus::connect(nats.url()).await?;

        Ok(NatsOnlyContext { nats, event_bus })
    }

    /// Get database connection string.
    pub fn db_url(&self) -> &str {
        self.postgres.connection_string()
    }

    /// Get NATS URL.
    pub fn nats_url(&self) -> &str {
        self.nats.url()
    }

    /// Get MinIO endpoint.
    pub fn minio_endpoint(&self) -> &str {
        self.minio.endpoint()
    }
}

/// Minimal context with only PostgreSQL.
pub struct PostgresOnlyContext {
    pub postgres: PostgresContainer,
    pub db: Database,
}

impl PostgresOnlyContext {
    pub fn db_url(&self) -> &str {
        self.postgres.connection_string()
    }
}

/// Minimal context with only NATS.
pub struct NatsOnlyContext {
    pub nats: NatsContainer,
    pub event_bus: NatsEventBus,
}

impl NatsOnlyContext {
    pub fn nats_url(&self) -> &str {
        self.nats.url()
    }
}
