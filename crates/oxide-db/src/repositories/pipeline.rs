//! PostgreSQL implementation of PipelineRepository.

use async_trait::async_trait;
use oxide_core::ids::PipelineId;
use oxide_core::pipeline::{Pipeline, PipelineDefinition};
use oxide_core::ports::PipelineRepository;
use oxide_core::{Error, Result};
use sqlx::{PgPool, Row};

/// PostgreSQL implementation of PipelineRepository.
#[derive(Clone)]
pub struct PgPipelineRepository {
    pool: PgPool,
}

impl PgPipelineRepository {
    /// Create a new PgPipelineRepository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PipelineRepository for PgPipelineRepository {
    async fn create(&self, definition: &PipelineDefinition) -> Result<Pipeline> {
        let id = uuid::Uuid::new_v4();
        let definition_json =
            serde_json::to_value(definition).map_err(|e| Error::Serialization(e.to_string()))?;
        let now = chrono::Utc::now();

        sqlx::query(
            "INSERT INTO pipelines (id, name, description, definition, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(id)
        .bind(&definition.name)
        .bind(&definition.description)
        .bind(&definition_json)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Pipeline {
            id: PipelineId::from_uuid(id),
            name: definition.name.clone(),
            definition: definition.clone(),
            created_at: now,
            updated_at: now,
        })
    }

    async fn get(&self, id: PipelineId) -> Result<Option<Pipeline>> {
        let row = sqlx::query(
            "SELECT id, name, definition, created_at, updated_at FROM pipelines WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        match row {
            Some(r) => {
                let def_json: serde_json::Value = r.get("definition");
                let definition: PipelineDefinition = serde_json::from_value(def_json)
                    .map_err(|e| Error::Serialization(e.to_string()))?;

                Ok(Some(Pipeline {
                    id: PipelineId::from_uuid(r.get::<uuid::Uuid, _>("id")),
                    name: r.get("name"),
                    definition,
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<Pipeline>> {
        let row = sqlx::query(
            "SELECT id, name, definition, created_at, updated_at FROM pipelines WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        match row {
            Some(r) => {
                let def_json: serde_json::Value = r.get("definition");
                let definition: PipelineDefinition = serde_json::from_value(def_json)
                    .map_err(|e| Error::Serialization(e.to_string()))?;

                Ok(Some(Pipeline {
                    id: PipelineId::from_uuid(r.get::<uuid::Uuid, _>("id")),
                    name: r.get("name"),
                    definition,
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                }))
            }
            None => Ok(None),
        }
    }

    async fn list(&self, limit: u32, offset: u32) -> Result<Vec<Pipeline>> {
        let rows = sqlx::query(
            "SELECT id, name, definition, created_at, updated_at FROM pipelines ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        let mut pipelines = Vec::with_capacity(rows.len());
        for r in rows {
            let def_json: serde_json::Value = r.get("definition");
            let definition: PipelineDefinition = serde_json::from_value(def_json)
                .map_err(|e| Error::Serialization(e.to_string()))?;

            pipelines.push(Pipeline {
                id: PipelineId::from_uuid(r.get::<uuid::Uuid, _>("id")),
                name: r.get("name"),
                definition,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            });
        }

        Ok(pipelines)
    }

    async fn update(&self, id: PipelineId, definition: &PipelineDefinition) -> Result<Pipeline> {
        let definition_json =
            serde_json::to_value(definition).map_err(|e| Error::Serialization(e.to_string()))?;
        let now = chrono::Utc::now();

        let row = sqlx::query(
            "UPDATE pipelines SET name = $2, description = $3, definition = $4, updated_at = $5 WHERE id = $1 RETURNING id, name, definition, created_at, updated_at"
        )
        .bind(id.as_uuid())
        .bind(&definition.name)
        .bind(&definition.description)
        .bind(&definition_json)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        let def_json: serde_json::Value = row.get("definition");
        let definition: PipelineDefinition =
            serde_json::from_value(def_json).map_err(|e| Error::Serialization(e.to_string()))?;

        Ok(Pipeline {
            id: PipelineId::from_uuid(row.get::<uuid::Uuid, _>("id")),
            name: row.get("name"),
            definition,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    async fn delete(&self, id: PipelineId) -> Result<()> {
        sqlx::query("DELETE FROM pipelines WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }
}
