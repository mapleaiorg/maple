//! PostgreSQL storage implementation

use super::traits::*;
use crate::error::StorageError;
use async_trait::async_trait;
use palm_shared_state::{Activity, PlaygroundConfig, ResonatorStatus};
use palm_types::{
    instance::{AgentInstance, HealthStatus},
    AgentSpec, AgentSpecId, Deployment, DeploymentId, InstanceId, PalmEventEnvelope,
};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

/// PostgreSQL-backed storage
#[derive(Debug, Clone)]
pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    /// Connect to PostgreSQL and initialize schema
    pub async fn new(
        url: &str,
        max_connections: u32,
        connect_timeout_secs: u64,
    ) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(connect_timeout_secs))
            .connect(url)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let storage = Self { pool };
        storage.initialize_schema().await?;
        Ok(storage)
    }

    async fn initialize_schema(&self) -> Result<(), StorageError> {
        let statements = [
            r#"
            CREATE TABLE IF NOT EXISTS agent_specs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                data JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"CREATE INDEX IF NOT EXISTS agent_specs_name_version ON agent_specs(name, version);"#,
            r#"
            CREATE TABLE IF NOT EXISTS deployments (
                id UUID PRIMARY KEY,
                spec_id TEXT NOT NULL,
                status TEXT,
                data JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"CREATE INDEX IF NOT EXISTS deployments_spec_id ON deployments(spec_id);"#,
            r#"
            CREATE TABLE IF NOT EXISTS instances (
                id UUID PRIMARY KEY,
                deployment_id UUID NOT NULL,
                status TEXT,
                health TEXT,
                data JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"CREATE INDEX IF NOT EXISTS instances_deployment_id ON instances(deployment_id);"#,
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id UUID PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                source TEXT NOT NULL,
                severity TEXT NOT NULL,
                platform TEXT NOT NULL,
                actor TEXT,
                correlation_id TEXT,
                data JSONB NOT NULL
            );
            "#,
            r#"CREATE INDEX IF NOT EXISTS events_timestamp ON events(timestamp DESC);"#,
            r#"
            CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                instance_id UUID NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                reason TEXT NOT NULL,
                size_bytes BIGINT NOT NULL
            );
            "#,
            r#"CREATE INDEX IF NOT EXISTS snapshots_instance_id ON snapshots(instance_id);"#,
            r#"
            CREATE TABLE IF NOT EXISTS resonators (
                id TEXT PRIMARY KEY,
                status TEXT,
                last_activity TIMESTAMPTZ,
                data JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS activities (
                sequence BIGSERIAL PRIMARY KEY,
                id UUID NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                actor_type TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                summary TEXT NOT NULL,
                data JSONB NOT NULL
            );
            "#,
            r#"CREATE INDEX IF NOT EXISTS activities_timestamp ON activities(timestamp DESC);"#,
            r#"
            CREATE TABLE IF NOT EXISTS playground_state (
                id TEXT PRIMARY KEY,
                data JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
        ];

        for stmt in statements {
            sqlx::query(stmt)
                .execute(&self.pool)
                .await
                .map_err(|e| StorageError::Query(e.to_string()))?;
        }

        Ok(())
    }

    fn serialize_spec_id(id: &AgentSpecId) -> String {
        id.to_string()
    }

    fn to_json<T: serde::Serialize>(value: &T) -> Result<Value, StorageError> {
        serde_json::to_value(value)
            .map_err(|e| StorageError::InvalidData(format!("json serialize error: {}", e)))
    }

    fn from_json<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, StorageError> {
        serde_json::from_value(value)
            .map_err(|e| StorageError::InvalidData(format!("json deserialize error: {}", e)))
    }
}

#[async_trait]
impl SpecStorage for PostgresStorage {
    async fn get_spec(&self, id: &AgentSpecId) -> StorageResult<Option<AgentSpec>> {
        let id_str = Self::serialize_spec_id(id);
        let row = sqlx::query("SELECT data FROM agent_specs WHERE id = $1")
            .bind(id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        match row {
            Some(record) => {
                let data: Value = record
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Ok(Some(Self::from_json(data)?))
            }
            None => Ok(None),
        }
    }

    async fn list_specs(&self) -> StorageResult<Vec<AgentSpec>> {
        let rows = sqlx::query("SELECT data FROM agent_specs ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Self::from_json(data)
            })
            .collect()
    }

    async fn upsert_spec(&self, spec: AgentSpec) -> StorageResult<()> {
        let id_str = Self::serialize_spec_id(&spec.id);
        let data = Self::to_json(&spec)?;
        let updated_at = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO agent_specs (id, name, version, data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id)
            DO UPDATE SET
                name = EXCLUDED.name,
                version = EXCLUDED.version,
                data = EXCLUDED.data,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(id_str)
        .bind(spec.name)
        .bind(spec.version.to_string())
        .bind(data)
        .bind(spec.created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn delete_spec(&self, id: &AgentSpecId) -> StorageResult<bool> {
        let id_str = Self::serialize_spec_id(id);
        let result = sqlx::query("DELETE FROM agent_specs WHERE id = $1")
            .bind(id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }

    async fn get_spec_by_name(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> StorageResult<Option<AgentSpec>> {
        let row = if let Some(version) = version {
            sqlx::query("SELECT data FROM agent_specs WHERE name = $1 AND version = $2 LIMIT 1")
                .bind(name)
                .bind(version)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StorageError::Query(e.to_string()))?
        } else {
            sqlx::query(
                "SELECT data FROM agent_specs WHERE name = $1 ORDER BY updated_at DESC LIMIT 1",
            )
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?
        };

        match row {
            Some(record) => {
                let data: Value = record
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Ok(Some(Self::from_json(data)?))
            }
            None => Ok(None),
        }
    }
}

#[async_trait]
impl DeploymentStorage for PostgresStorage {
    async fn get_deployment(&self, id: &DeploymentId) -> StorageResult<Option<Deployment>> {
        let row = sqlx::query("SELECT data FROM deployments WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        match row {
            Some(record) => {
                let data: Value = record
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Ok(Some(Self::from_json(data)?))
            }
            None => Ok(None),
        }
    }

    async fn list_deployments(&self) -> StorageResult<Vec<Deployment>> {
        let rows = sqlx::query("SELECT data FROM deployments ORDER BY updated_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Self::from_json(data)
            })
            .collect()
    }

    async fn list_deployments_for_spec(
        &self,
        spec_id: &AgentSpecId,
    ) -> StorageResult<Vec<Deployment>> {
        let spec_str = Self::serialize_spec_id(spec_id);
        let rows =
            sqlx::query("SELECT data FROM deployments WHERE spec_id = $1 ORDER BY updated_at DESC")
                .bind(spec_str)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| StorageError::Query(e.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Self::from_json(data)
            })
            .collect()
    }

    async fn upsert_deployment(&self, deployment: Deployment) -> StorageResult<()> {
        let data = Self::to_json(&deployment)?;
        let spec_id = Self::serialize_spec_id(&deployment.agent_spec_id);

        sqlx::query(
            r#"
            INSERT INTO deployments (id, spec_id, status, data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id)
            DO UPDATE SET
                spec_id = EXCLUDED.spec_id,
                status = EXCLUDED.status,
                data = EXCLUDED.data,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(deployment.id.as_uuid())
        .bind(spec_id)
        .bind(format!("{:?}", deployment.status))
        .bind(data)
        .bind(deployment.created_at)
        .bind(deployment.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn delete_deployment(&self, id: &DeploymentId) -> StorageResult<bool> {
        let result = sqlx::query("DELETE FROM deployments WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }
}

#[async_trait]
impl InstanceStorage for PostgresStorage {
    async fn get_instance(&self, id: &InstanceId) -> StorageResult<Option<AgentInstance>> {
        let row = sqlx::query("SELECT data FROM instances WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        match row {
            Some(record) => {
                let data: Value = record
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Ok(Some(Self::from_json(data)?))
            }
            None => Ok(None),
        }
    }

    async fn list_instances(&self) -> StorageResult<Vec<AgentInstance>> {
        let rows = sqlx::query("SELECT data FROM instances ORDER BY updated_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Self::from_json(data)
            })
            .collect()
    }

    async fn list_instances_for_deployment(
        &self,
        deployment_id: &DeploymentId,
    ) -> StorageResult<Vec<AgentInstance>> {
        let rows = sqlx::query(
            "SELECT data FROM instances WHERE deployment_id = $1 ORDER BY updated_at DESC",
        )
        .bind(deployment_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Self::from_json(data)
            })
            .collect()
    }

    async fn upsert_instance(&self, instance: AgentInstance) -> StorageResult<()> {
        let data = Self::to_json(&instance)?;
        let status = format!("{:?}", instance.status);
        let health = match &instance.health {
            HealthStatus::Healthy => "healthy".to_string(),
            HealthStatus::Degraded { .. } => "degraded".to_string(),
            HealthStatus::Unhealthy { .. } => "unhealthy".to_string(),
            HealthStatus::Unknown => "unknown".to_string(),
        };

        sqlx::query(
            r#"
            INSERT INTO instances (id, deployment_id, status, health, data, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id)
            DO UPDATE SET
                deployment_id = EXCLUDED.deployment_id,
                status = EXCLUDED.status,
                health = EXCLUDED.health,
                data = EXCLUDED.data,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(instance.id.as_uuid())
        .bind(instance.deployment_id.as_uuid())
        .bind(status)
        .bind(health)
        .bind(data)
        .bind(instance.last_heartbeat)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn delete_instance(&self, id: &InstanceId) -> StorageResult<bool> {
        let result = sqlx::query("DELETE FROM instances WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_unhealthy_instances(&self) -> StorageResult<Vec<AgentInstance>> {
        let rows = sqlx::query(
            "SELECT data FROM instances WHERE health = 'unhealthy' OR health = 'degraded'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Self::from_json(data)
            })
            .collect()
    }
}

#[async_trait]
impl EventStorage for PostgresStorage {
    async fn store_event(&self, event: PalmEventEnvelope) -> StorageResult<()> {
        let data = Self::to_json(&event)?;
        sqlx::query(
            r#"
            INSERT INTO events (id, timestamp, source, severity, platform, actor, correlation_id, data)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(event.id)
        .bind(event.timestamp)
        .bind(format!("{:?}", event.source))
        .bind(format!("{:?}", event.severity))
        .bind(format!("{:?}", event.platform))
        .bind(event.actor)
        .bind(event.correlation_id)
        .bind(data)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(())
    }

    async fn get_recent_events(&self, limit: usize) -> StorageResult<Vec<PalmEventEnvelope>> {
        let rows = sqlx::query("SELECT data FROM events ORDER BY timestamp DESC LIMIT $1")
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let mut events: Vec<PalmEventEnvelope> = rows
            .into_iter()
            .map(|row| {
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Self::from_json(data)
            })
            .collect::<Result<_, _>>()?;
        events.reverse();
        Ok(events)
    }

    async fn get_events_for_deployment(
        &self,
        deployment_id: &DeploymentId,
        limit: usize,
    ) -> StorageResult<Vec<PalmEventEnvelope>> {
        let dep_str = deployment_id.to_string();
        let rows = sqlx::query(
            "SELECT data FROM events WHERE data::text ILIKE $1 ORDER BY timestamp DESC LIMIT $2",
        )
        .bind(format!("%{}%", dep_str))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        let mut events: Vec<PalmEventEnvelope> = rows
            .into_iter()
            .map(|row| {
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Self::from_json(data)
            })
            .collect::<Result<_, _>>()?;
        events.reverse();
        Ok(events)
    }

    async fn get_events_for_instance(
        &self,
        instance_id: &InstanceId,
        limit: usize,
    ) -> StorageResult<Vec<PalmEventEnvelope>> {
        let inst_str = instance_id.to_string();
        let rows = sqlx::query(
            "SELECT data FROM events WHERE data::text ILIKE $1 ORDER BY timestamp DESC LIMIT $2",
        )
        .bind(format!("%{}%", inst_str))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        let mut events: Vec<PalmEventEnvelope> = rows
            .into_iter()
            .map(|row| {
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Self::from_json(data)
            })
            .collect::<Result<_, _>>()?;
        events.reverse();
        Ok(events)
    }
}

#[async_trait]
impl SnapshotStorage for PostgresStorage {
    async fn create_snapshot(
        &self,
        instance_id: &InstanceId,
        reason: &str,
    ) -> StorageResult<String> {
        let snapshot_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO snapshots (id, instance_id, created_at, reason, size_bytes)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&snapshot_id)
        .bind(instance_id.as_uuid())
        .bind(now)
        .bind(reason)
        .bind(1024_i64)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(snapshot_id)
    }

    async fn list_snapshots(&self, instance_id: &InstanceId) -> StorageResult<Vec<SnapshotInfo>> {
        let rows = sqlx::query(
            "SELECT id, created_at, reason, size_bytes FROM snapshots WHERE instance_id = $1 ORDER BY created_at DESC",
        )
        .bind(instance_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        let snapshots = rows
            .into_iter()
            .map(|row| {
                let id: String = row
                    .try_get("id")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                let created_at = row
                    .try_get("created_at")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                let reason = row
                    .try_get("reason")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                let size_bytes: i64 = row
                    .try_get("size_bytes")
                    .map_err(|e| StorageError::Query(e.to_string()))?;

                Ok(SnapshotInfo {
                    id,
                    instance_id: instance_id.clone(),
                    created_at,
                    reason,
                    size_bytes: size_bytes as u64,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;

        Ok(snapshots)
    }

    async fn restore_snapshot(
        &self,
        instance_id: &InstanceId,
        snapshot_id: &str,
    ) -> StorageResult<()> {
        let row = sqlx::query("SELECT instance_id FROM snapshots WHERE id = $1")
            .bind(snapshot_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let stored = row
            .ok_or_else(|| StorageError::NotFound(format!("Snapshot {} not found", snapshot_id)))?;
        let stored_id: Uuid = stored
            .try_get("instance_id")
            .map_err(|e| StorageError::Query(e.to_string()))?;

        if stored_id != *instance_id.as_uuid() {
            return Err(StorageError::InvalidData(
                "Snapshot does not belong to this instance".to_string(),
            ));
        }

        Ok(())
    }

    async fn delete_snapshot(&self, snapshot_id: &str) -> StorageResult<bool> {
        let result = sqlx::query("DELETE FROM snapshots WHERE id = $1")
            .bind(snapshot_id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }
}

#[async_trait]
impl PlaygroundConfigStorage for PostgresStorage {
    async fn get_playground_config(&self) -> StorageResult<Option<PlaygroundConfig>> {
        let row = sqlx::query("SELECT data FROM playground_state WHERE id = 'singleton'")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        match row {
            Some(record) => {
                let data: Value = record
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Ok(Some(Self::from_json(data)?))
            }
            None => Ok(None),
        }
    }

    async fn upsert_playground_config(&self, config: PlaygroundConfig) -> StorageResult<()> {
        let data = Self::to_json(&config)?;

        sqlx::query(
            r#"
            INSERT INTO playground_state (id, data, updated_at)
            VALUES ('singleton', $1, $2)
            ON CONFLICT (id)
            DO UPDATE SET data = EXCLUDED.data, updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(data)
        .bind(config.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl ResonatorStorage for PostgresStorage {
    async fn get_resonator(&self, id: &str) -> StorageResult<Option<ResonatorStatus>> {
        let row = sqlx::query("SELECT data FROM resonators WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        match row {
            Some(record) => {
                let data: Value = record
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Ok(Some(Self::from_json(data)?))
            }
            None => Ok(None),
        }
    }

    async fn list_resonators(&self) -> StorageResult<Vec<ResonatorStatus>> {
        let rows = sqlx::query("SELECT data FROM resonators ORDER BY updated_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                Self::from_json(data)
            })
            .collect()
    }

    async fn upsert_resonator(&self, resonator: ResonatorStatus) -> StorageResult<()> {
        let data = Self::to_json(&resonator)?;

        sqlx::query(
            r#"
            INSERT INTO resonators (id, status, last_activity, data, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id)
            DO UPDATE SET
                status = EXCLUDED.status,
                last_activity = EXCLUDED.last_activity,
                data = EXCLUDED.data,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(&resonator.id)
        .bind(format!("{:?}", resonator.status))
        .bind(resonator.last_activity)
        .bind(data)
        .bind(resonator.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn delete_resonator(&self, id: &str) -> StorageResult<bool> {
        let result = sqlx::query("DELETE FROM resonators WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(result.rows_affected() > 0)
    }
}

#[async_trait]
impl ActivityStorage for PostgresStorage {
    async fn store_activity(&self, activity: Activity) -> StorageResult<Activity> {
        let details = Self::to_json(&activity.details)?;

        let row = sqlx::query(
            r#"
            INSERT INTO activities (id, timestamp, actor_type, actor_id, kind, summary, data)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING sequence
            "#,
        )
        .bind(activity.id)
        .bind(activity.timestamp)
        .bind(format!("{:?}", activity.actor_type).to_lowercase())
        .bind(&activity.actor_id)
        .bind(&activity.kind)
        .bind(&activity.summary)
        .bind(details)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        let sequence: i64 = row
            .try_get("sequence")
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(Activity {
            sequence: sequence as u64,
            ..activity
        })
    }

    async fn list_activities(
        &self,
        limit: usize,
        after_sequence: Option<u64>,
    ) -> StorageResult<Vec<Activity>> {
        let rows = if let Some(after) = after_sequence {
            sqlx::query(
                r#"
                SELECT sequence, id, timestamp, actor_type, actor_id, kind, summary, data
                FROM activities
                WHERE sequence > $1
                ORDER BY sequence ASC
                LIMIT $2
                "#,
            )
            .bind(after as i64)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?
        } else {
            sqlx::query(
                r#"
                SELECT sequence, id, timestamp, actor_type, actor_id, kind, summary, data
                FROM activities
                ORDER BY sequence DESC
                LIMIT $1
                "#,
            )
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?
        };

        let mut activities: Vec<Activity> = rows
            .into_iter()
            .map(|row| {
                let actor_type: String = row
                    .try_get("actor_type")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                let data: Value = row
                    .try_get("data")
                    .map_err(|e| StorageError::Query(e.to_string()))?;

                Ok(Activity {
                    id: row
                        .try_get("id")
                        .map_err(|e| StorageError::Query(e.to_string()))?,
                    sequence: row
                        .try_get::<i64, _>("sequence")
                        .map_err(|e| StorageError::Query(e.to_string()))?
                        as u64,
                    timestamp: row
                        .try_get("timestamp")
                        .map_err(|e| StorageError::Query(e.to_string()))?,
                    actor_type: match actor_type.to_lowercase().as_str() {
                        "agent" => palm_shared_state::ActivityActor::Agent,
                        "human" => palm_shared_state::ActivityActor::Human,
                        "resonator" => palm_shared_state::ActivityActor::Resonator,
                        _ => palm_shared_state::ActivityActor::System,
                    },
                    actor_id: row
                        .try_get("actor_id")
                        .map_err(|e| StorageError::Query(e.to_string()))?,
                    kind: row
                        .try_get("kind")
                        .map_err(|e| StorageError::Query(e.to_string()))?,
                    summary: row
                        .try_get("summary")
                        .map_err(|e| StorageError::Query(e.to_string()))?,
                    details: data,
                })
            })
            .collect::<Result<_, StorageError>>()?;

        if after_sequence.is_none() {
            activities.sort_by_key(|a| a.sequence);
        }

        Ok(activities)
    }
}

impl Storage for PostgresStorage {}
