# File: maple/core/map/orchestration/persistence.py
# Description: Persistence layer for workflow state management, supporting
# multiple backends including in-memory, Redis, and database storage.

from __future__ import annotations
import asyncio
import json
import logging
import pickle
from abc import ABC, abstractmethod
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Any, Set
from uuid import UUID
import aioredis
import asyncpg
from dataclasses import dataclass, field

from maple.core.map.orchestration.models import WorkflowContext, WorkflowState

logger = logging.getLogger(__name__)


class WorkflowPersistence(ABC):
    """Abstract base class for workflow persistence"""

    @abstractmethod
    async def save_instance(self, instance: 'WorkflowInstance') -> None:
        """Save workflow instance state"""
        pass

    @abstractmethod
    async def load_instance(self, instance_id: UUID) -> Optional[Dict[str, Any]]:
        """Load workflow instance from PostgreSQL"""
        await self.connect()

        async with self.pool.acquire() as conn:
            row = await conn.fetchrow(f"""
                SELECT * FROM {self.schema}.workflow_instances
                WHERE instance_id = $1
            """, instance_id)

            if row:
                return {
                    "instance_id": str(row["instance_id"]),
                    "workflow_id": row["workflow_id"],
                    "state": row["state"],
                    "execution_mode": row["execution_mode"],
                    "context": json.loads(row["context"]),
                    "metrics": json.loads(row["metrics"]) if row["metrics"] else {},
                    "step_results": json.loads(row["step_results"]) if row["step_results"] else {},
                    "current_step_index": 0  # Would need to store this separately
                }

            return None

    async def delete_instance(self, instance_id: UUID) -> None:
        """Delete workflow instance from PostgreSQL"""
        await self.connect()

        async with self.pool.acquire() as conn:
            await conn.execute(f"""
                DELETE FROM {self.schema}.workflow_instances
                WHERE instance_id = $1
            """, instance_id)

    async def list_instances(self,
                             workflow_id: Optional[str] = None,
                             state: Optional[WorkflowState] = None,
                             limit: int = 100) -> List[Dict[str, Any]]:
        """List workflow instances from PostgreSQL"""
        await self.connect()

        query = f"""
            SELECT instance_id, workflow_id, state, 
                   context->>'created_at' as created_at,
                   context->>'updated_at' as updated_at
            FROM {self.schema}.workflow_instances
            WHERE 1=1
        """

        params = []
        param_count = 0

        if workflow_id:
            param_count += 1
            query += f" AND workflow_id = ${param_count}"
            params.append(workflow_id)

        if state:
            param_count += 1
            query += f" AND state = ${param_count}"
            params.append(state.value)

        query += f" ORDER BY created_at DESC LIMIT {limit}"

        async with self.pool.acquire() as conn:
            rows = await conn.fetch(query, *params)

            return [
                {
                    "instance_id": str(row["instance_id"]),
                    "workflow_id": row["workflow_id"],
                    "state": row["state"],
                    "created_at": row["created_at"],
                    "updated_at": row["updated_at"]
                }
                for row in rows
            ]

    async def archive_instance(self, instance_id: UUID) -> None:
        """Archive workflow instance in PostgreSQL"""
        await self.connect()

        async with self.pool.acquire() as conn:
            async with conn.transaction():
                # Get instance data
                row = await conn.fetchrow(f"""
                    SELECT * FROM {self.schema}.workflow_instances
                    WHERE instance_id = $1
                """, instance_id)

                if row:
                    # Insert into archives
                    await conn.execute(f"""
                        INSERT INTO {self.schema}.workflow_archives
                        (instance_id, workflow_id, data)
                        VALUES ($1, $2, $3)
                    """,
                                       row["instance_id"],
                                       row["workflow_id"],
                                       json.dumps({
                                           "instance_id": str(row["instance_id"]),
                                           "workflow_id": row["workflow_id"],
                                           "state": row["state"],
                                           "execution_mode": row["execution_mode"],
                                           "context": json.loads(row["context"]),
                                           "metrics": json.loads(row["metrics"]) if row["metrics"] else {},
                                           "step_results": json.loads(row["step_results"]) if row[
                                               "step_results"] else {}
                                       })
                                       )

                    # Delete from active instances
                    await conn.execute(f"""
                        DELETE FROM {self.schema}.workflow_instances
                        WHERE instance_id = $1
                    """, instance_id)

    async def save_checkpoint(self, instance_id: UUID, checkpoint_id: str, data: Dict[str, Any]) -> None:
        """Save checkpoint to PostgreSQL"""
        await self.connect()

        async with self.pool.acquire() as conn:
            await conn.execute(f"""
                INSERT INTO {self.schema}.workflow_checkpoints
                (instance_id, checkpoint_id, data)
                VALUES ($1, $2, $3)
                ON CONFLICT (instance_id, checkpoint_id) DO UPDATE SET
                    data = EXCLUDED.data,
                    created_at = NOW()
            """,
                               instance_id,
                               checkpoint_id,
                               json.dumps(data)
                               )

    async def load_checkpoint(self, instance_id: UUID, checkpoint_id: str) -> Optional[Dict[str, Any]]:
        """Load checkpoint from PostgreSQL"""
        await self.connect()

        async with self.pool.acquire() as conn:
            row = await conn.fetchrow(f"""
                SELECT data FROM {self.schema}.workflow_checkpoints
                WHERE instance_id = $1 AND checkpoint_id = $2
            """, instance_id, checkpoint_id)

            if row:
                return json.loads(row["data"])

            return None


class CompositePersistence(WorkflowPersistence):
    """Composite persistence using multiple backends"""

    def __init__(self,
                 primary: WorkflowPersistence,
                 cache: Optional[WorkflowPersistence] = None,
                 backup: Optional[WorkflowPersistence] = None):
        self.primary = primary
        self.cache = cache
        self.backup = backup

    async def save_instance(self, instance: 'WorkflowInstance') -> None:
        """Save to all backends"""
        # Save to primary
        await self.primary.save_instance(instance)

        # Save to cache for fast access
        if self.cache:
            try:
                await self.cache.save_instance(instance)
            except Exception as e:
                logger.warning(f"Failed to save to cache: {str(e)}")

        # Save to backup
        if self.backup:
            try:
                await self.backup.save_instance(instance)
            except Exception as e:
                logger.error(f"Failed to save to backup: {str(e)}")

    async def load_instance(self, instance_id: UUID) -> Optional[Dict[str, Any]]:
        """Load from cache first, then primary"""
        # Try cache first
        if self.cache:
            try:
                data = await self.cache.load_instance(instance_id)
                if data:
                    return data
            except Exception as e:
                logger.warning(f"Failed to load from cache: {str(e)}")

        # Load from primary
        data = await self.primary.load_instance(instance_id)

        # Update cache
        if data and self.cache:
            try:
                # Create a minimal instance object for caching
                class MinimalInstance:
                    def __init__(self, data):
                        self.instance_id = UUID(data["instance_id"])
                        self.to_dict = lambda: data

                await self.cache.save_instance(MinimalInstance(data))
            except Exception as e:
                logger.warning(f"Failed to update cache: {str(e)}")

        return data

    async def delete_instance(self, instance_id: UUID) -> None:
        """Delete from all backends"""
        # Delete from primary
        await self.primary.delete_instance(instance_id)

        # Delete from cache
        if self.cache:
            try:
                await self.cache.delete_instance(instance_id)
            except Exception as e:
                logger.warning(f"Failed to delete from cache: {str(e)}")

        # Delete from backup
        if self.backup:
            try:
                await self.backup.delete_instance(instance_id)
            except Exception as e:
                logger.error(f"Failed to delete from backup: {str(e)}")

    async def list_instances(self,
                             workflow_id: Optional[str] = None,
                             state: Optional[WorkflowState] = None,
                             limit: int = 100) -> List[Dict[str, Any]]:
        """List from primary backend"""
        return await self.primary.list_instances(workflow_id, state, limit)

    async def archive_instance(self, instance_id: UUID) -> None:
        """Archive in all backends"""
        await self.primary.archive_instance(instance_id)

        if self.backup:
            try:
                await self.backup.archive_instance(instance_id)
            except Exception as e:
                logger.error(f"Failed to archive in backup: {str(e)}")

    async def save_checkpoint(self, instance_id: UUID, checkpoint_id: str, data: Dict[str, Any]) -> None:
        """Save checkpoint to all backends"""
        await self.primary.save_checkpoint(instance_id, checkpoint_id, data)

        if self.cache:
            try:
                await self.cache.save_checkpoint(instance_id, checkpoint_id, data)
            except Exception as e:
                logger.warning(f"Failed to save checkpoint to cache: {str(e)}")

    async def load_checkpoint(self, instance_id: UUID, checkpoint_id: str) -> Optional[Dict[str, Any]]:
        """Load checkpoint from cache first, then primary"""
        if self.cache:
            try:
                data = await self.cache.load_checkpoint(instance_id, checkpoint_id)
                if data:
                    return data
            except Exception as e:
                logger.warning(f"Failed to load checkpoint from cache: {str(e)}")

        return await self.primary.load_checkpoint(instance_id, checkpoint_id)
        d
        workflow
        instance
        state
        """
        pass

    @abstractmethod
    async def delete_instance(self, instance_id: UUID) -> None:
        """
        Delete
        workflow
        instance
        """
        pass

    @abstractmethod
    async def list_instances(self, 
                           workflow_id: Optional[str] = None,
                           state: Optional[WorkflowState] = None,
                           limit: int = 100) -> List[Dict[str, Any]]:
        """
        List
        workflow
        instances
        """
        pass

    @abstractmethod
    async def archive_instance(self, instance_id: UUID) -> None:
        """
        Archive
        completed
        workflow
        instance
        """
        pass

    @abstractmethod
    async def save_checkpoint(self, instance_id: UUID, checkpoint_id: str, data: Dict[str, Any]) -> None:
        """
        Save
        workflow
        checkpoint
        """
        pass

    @abstractmethod
    async def load_checkpoint(self, instance_id: UUID, checkpoint_id: str) -> Optional[Dict[str, Any]]:
        """
        Load
        workflow
        checkpoint
        """
        pass


class InMemoryPersistence(WorkflowPersistence):
    """
        In - memory
        persistence
        for development / testing"""

    def __init__(self):
        self.instances: Dict[UUID, Dict[str, Any]] = {}
        self.checkpoints: Dict[Tuple[UUID, str], Dict[str, Any]] = {}
        self.archived: Dict[UUID, Dict[str, Any]] = {}

    async def save_instance(self, instance: 'WorkflowInstance') -> None:
        """Save workflow instance in memory"""
        self.instances[instance.instance_id] = instance.to_dict()

    async def load_instance(self, instance_id: UUID) -> Optional[Dict[str, Any]]:
        """Load workflow instance from memory"""
        return self.instances.get(instance_id)

    async def delete_instance(self, instance_id: UUID) -> None:
        """Delete workflow instance from memory"""
        if instance_id in self.instances:
            del self.instances[instance_id]

        # Delete associated checkpoints
        checkpoint_keys = [
            key for key in self.checkpoints.keys()
            if key[0] == instance_id
        ]
        for key in checkpoint_keys:
            del self.checkpoints[key]

    async def list_instances(self,
                           workflow_id: Optional[str] = None,
                           state: Optional[WorkflowState] = None,
                           limit: int = 100) -> List[Dict[str, Any]]:
        """List workflow instances from memory"""
        results = []

        for instance_data in self.instances.values():
            # Apply filters
            if workflow_id and instance_data.get("workflow_id") != workflow_id:
                continue

            if state and instance_data.get("state") != state.value:
                continue

            results.append({
                "instance_id": instance_data["instance_id"],
                "workflow_id": instance_data["workflow_id"],
                "state": instance_data["state"],
                "created_at": instance_data["context"]["created_at"],
                "updated_at": instance_data["context"]["updated_at"]
            })

            if len(results) >= limit:
                break

        return results

    async def archive_instance(self, instance_id: UUID) -> None:
        """Archive workflow instance"""
        if instance_id in self.instances:
            self.archived[instance_id] = self.instances[instance_id]
            del self.instances[instance_id]

    async def save_checkpoint(self, instance_id: UUID, checkpoint_id: str, data: Dict[str, Any]) -> None:
        """Save checkpoint in memory"""
        self.checkpoints[(instance_id, checkpoint_id)] = data

    async def load_checkpoint(self, instance_id: UUID, checkpoint_id: str) -> Optional[Dict[str, Any]]:
        """Load checkpoint from memory"""
        return self.checkpoints.get((instance_id, checkpoint_id))


class RedisPersistence(WorkflowPersistence):
    """Redis-based persistence for distributed deployments"""

    def __init__(self,
                 redis_url: str = "redis://localhost:6379",
                 key_prefix: str = "maple:workflow:",
                 ttl: Optional[timedelta] = None):
        self.redis_url = redis_url
        self.key_prefix = key_prefix
        self.ttl = ttl
        self.redis: Optional[aioredis.Redis] = None

    async def connect(self) -> None:
        """Connect to Redis"""
        if not self.redis:
            self.redis = await aioredis.from_url(self.redis_url)

    async def disconnect(self) -> None:
        """Disconnect from Redis"""
        if self.redis:
            await self.redis.close()
            self.redis = None

    def _make_key(self, *parts: str) -> str:
        """Create Redis key"""
        return self.key_prefix + ":".join(str(p) for p in parts)

    async def save_instance(self, instance: 'WorkflowInstance') -> None:
        """Save workflow instance to Redis"""
        await self.connect()

        key = self._make_key("instance", str(instance.instance_id))
        data = json.dumps(instance.to_dict())

        if self.ttl:
            await self.redis.setex(key, self.ttl, data)
        else:
            await self.redis.set(key, data)

        # Add to workflow index
        workflow_key = self._make_key("workflow", instance.definition.workflow_id, "instances")
        await self.redis.sadd(workflow_key, str(instance.instance_id))

        # Add to state index
        state_key = self._make_key("state", instance.state.value)
        await self.redis.sadd(state_key, str(instance.instance_id))

    async def load_instance(self, instance_id: UUID) -> Optional[Dict[str, Any]]:
        """Load workflow instance from Redis"""
        await self.connect()

        key = self._make_key("instance", str(instance_id))
        data = await self.redis.get(key)

        if data:
            return json.loads(data)
        return None

    async def delete_instance(self, instance_id: UUID) -> None:
        """Delete workflow instance from Redis"""
        await self.connect()

        # Get instance data first
        instance_data = await self.load_instance(instance_id)
        if not instance_data:
            return

        # Delete instance data
        key = self._make_key("instance", str(instance_id))
        await self.redis.delete(key)

        # Remove from indices
        workflow_key = self._make_key("workflow", instance_data["workflow_id"], "instances")
        await self.redis.srem(workflow_key, str(instance_id))

        state_key = self._make_key("state", instance_data["state"])
        await self.redis.srem(state_key, str(instance_id))

        # Delete checkpoints
        checkpoint_pattern = self._make_key("checkpoint", str(instance_id), "*")
        async for key in self.redis.scan_iter(match=checkpoint_pattern):
            await self.redis.delete(key)

    async def list_instances(self,
                           workflow_id: Optional[str] = None,
                           state: Optional[WorkflowState] = None,
                           limit: int = 100) -> List[Dict[str, Any]]:
        """List workflow instances from Redis"""
        await self.connect()

        instance_ids: Set[str] = set()

        if workflow_id:
            # Get instances for specific workflow
            workflow_key = self._make_key("workflow", workflow_id, "instances")
            instance_ids = set(await self.redis.smembers(workflow_key))
        elif state:
            # Get instances in specific state
            state_key = self._make_key("state", state.value)
            instance_ids = set(await self.redis.smembers(state_key))
        else:
            # Get all instances
            pattern = self._make_key("instance", "*")
            async for key in self.redis.scan_iter(match=pattern):
                instance_id = key.decode().split(":")[-1]
                instance_ids.add(instance_id)

        # Load instance summaries
        results = []
        for instance_id in list(instance_ids)[:limit]:
            instance_data = await self.load_instance(UUID(instance_id))
            if instance_data:
                results.append({
                    "instance_id": instance_data["instance_id"],
                    "workflow_id": instance_data["workflow_id"],
                    "state": instance_data["state"],
                    "created_at": instance_data["context"]["created_at"],
                    "updated_at": instance_data["context"]["updated_at"]
                })

        return results

    async def archive_instance(self, instance_id: UUID) -> None:
        """Archive workflow instance in Redis"""
        await self.connect()

        # Move to archive key with longer TTL
        instance_data = await self.load_instance(instance_id)
        if instance_data:
            archive_key = self._make_key("archive", str(instance_id))
            archive_ttl = timedelta(days=90)  # Keep archives for 90 days

            await self.redis.setex(
                archive_key,
                archive_ttl,
                json.dumps(instance_data)
            )

            # Delete active instance
            await self.delete_instance(instance_id)

    async def save_checkpoint(self, instance_id: UUID, checkpoint_id: str, data: Dict[str, Any]) -> None:
        """Save checkpoint to Redis"""
        await self.connect()

        key = self._make_key("checkpoint", str(instance_id), checkpoint_id)

        if self.ttl:
            await self.redis.setex(key, self.ttl, json.dumps(data))
        else:
            await self.redis.set(key, json.dumps(data))

    async def load_checkpoint(self, instance_id: UUID, checkpoint_id: str) -> Optional[Dict[str, Any]]:
        """Load checkpoint from Redis"""
        await self.connect()

        key = self._make_key("checkpoint", str(instance_id), checkpoint_id)
        data = await self.redis.get(key)

        if data:
            return json.loads(data)
        return None


class PostgresPersistence(WorkflowPersistence):
    """PostgreSQL-based persistence for production deployments"""

    def __init__(self,
                 dsn: str,
                 schema: str = "maple_workflows",
                 pool_size: int = 10):
        self.dsn = dsn
        self.schema = schema
        self.pool_size = pool_size
        self.pool: Optional[asyncpg.Pool] = None

    async def connect(self) -> None:
        """Connect to PostgreSQL and create schema"""
        if not self.pool:
            self.pool = await asyncpg.create_pool(
                self.dsn,
                min_size=1,
                max_size=self.pool_size
            )

            # Create schema and tables
            async with self.pool.acquire() as conn:
                await self._create_schema(conn)

    async def disconnect(self) -> None:
        """Disconnect from PostgreSQL"""
        if self.pool:
            await self.pool.close()
            self.pool = None

    async def _create_schema(self, conn: asyncpg.Connection) -> None:
        """Create database schema"""
        await conn.execute(f"""
        CREATE
        SCHEMA
        IF
        NOT
        EXISTS
        {self.schema}

    """)

    await conn.execute(f"""
    CREATE
    TABLE
    IF
    NOT
    EXISTS
    {self.schema}.workflow_instances(
        instance_id
    UUID
    PRIMARY
    KEY,
    workflow_id
    VARCHAR(255)
    NOT
    NULL,
    state
    VARCHAR(50)
    NOT
    NULL,
    execution_mode
    VARCHAR(50),
    context
    JSONB
    NOT
    NULL,
    metrics
    JSONB,
    step_results
    JSONB,
    created_at
    TIMESTAMP
    NOT
    NULL,
    updated_at
    TIMESTAMP
    NOT
    NULL,
    completed_at
    TIMESTAMP,

    INDEX
    idx_workflow_id(workflow_id),
    INDEX
    idx_state(state),
    INDEX
    idx_created_at(created_at)
    )
    """)

    await conn.execute(f"""
    CREATE
    TABLE
    IF
    NOT
    EXISTS
    {self.schema}.workflow_checkpoints(
        instance_id
    UUID
    NOT
    NULL,
    checkpoint_id
    VARCHAR(255)
    NOT
    NULL,
    data
    JSONB
    NOT
    NULL,
    created_at
    TIMESTAMP
    NOT
    NULL
    DEFAULT
    NOW(),

    PRIMARY
    KEY(instance_id, checkpoint_id),
    FOREIGN
    KEY(instance_id)
    REFERENCES
    {self.schema}.workflow_instances(instance_id)
    ON
    DELETE
    CASCADE
    )
    """)

    await conn.execute(f"""
    CREATE
    TABLE
    IF
    NOT
    EXISTS
    {self.schema}.workflow_archives(
        instance_id
    UUID
    PRIMARY
    KEY,
    workflow_id
    VARCHAR(255)
    NOT
    NULL,
    data
    JSONB
    NOT
    NULL,
    archived_at
    TIMESTAMP
    NOT
    NULL
    DEFAULT
    NOW(),

    INDEX
    idx_archived_at(archived_at)
    )
    """)

async def save_instance(self, instance: 'WorkflowInstance') -> None:
    """
    Save
    workflow
    instance
    to
    PostgreSQL
    """
        await self.connect()

        data = instance.to_dict()

        async with self.pool.acquire() as conn:
            await conn.execute(f"""
    INSERT
    INTO
    {self.schema}.workflow_instances(
        instance_id, workflow_id, state, execution_mode,
        context, metrics, step_results,
        created_at, updated_at, completed_at
    )
    VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
    ON
    CONFLICT(instance_id)
    DO
    UPDATE
    SET
    state = EXCLUDED.state,
    context = EXCLUDED.context,
    metrics = EXCLUDED.metrics,
    step_results = EXCLUDED.step_results,
    updated_at = EXCLUDED.updated_at,
    completed_at = EXCLUDED.completed_at


""",
    UUID(data["instance_id"]),
    data["workflow_id"],
    data["state"],
    data.get("execution_mode"),
    json.dumps(data["context"]),
    json.dumps(data.get("metrics", {})),
    json.dumps(data.get("step_results", {})),
    datetime.fromisoformat(data["context"]["created_at"]),
    datetime.fromisoformat(data["context"]["updated_at"]),
    datetime.fromisoformat(data["metrics"]["end_time"]) if data.get("metrics", {}).get("end_time") else None
)

async def load_instance(self, instance_id: UUID) -> Optional[Dict[str, Any]]:
"""
Loa