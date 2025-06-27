# File: maple/core/ars/storage/postgres.py
# Description: PostgreSQL storage implementation for enterprise deployments.
# Provides ACID compliance, advanced querying, and relational data integrity.

from __future__ import annotations
import asyncio
import json
import uuid
from datetime import datetime, timedelta
from typing import List, Optional, Dict, Any, Set, Union
import logging
from contextlib import asynccontextmanager

import asyncpg
from asyncpg.pool import Pool

from maple.core.ars.models.registry import (
    AgentRegistration, ServiceQuery, Capability,
    AgentStatus, HealthStatus, RegistryEvent, Endpoint
)
from maple.core.ars.storage.interface import RegistryStorage

logger = logging.getLogger(__name__)


class PostgresStorage(RegistryStorage):
    """PostgreSQL implementation of registry storage"""

    def __init__(
            self,
            dsn: Optional[str] = None,
            host: str = "localhost",
            port: int = 5432,
            database: str = "maple_ars",
            user: str = "maple",
            password: Optional[str] = None,
            min_pool_size: int = 10,
            max_pool_size: int = 20,
            schema: str = "ars"
    ):
        self.dsn = dsn
        self.host = host
        self.port = port
        self.database = database
        self.user = user
        self.password = password
        self.min_pool_size = min_pool_size
        self.max_pool_size = max_pool_size
        self.schema = schema
        self._pool: Optional[Pool] = None

    async def connect(self) -> None:
        """Connect to PostgreSQL and initialize schema"""
        try:
            if self.dsn:
                self._pool = await asyncpg.create_pool(
                    self.dsn,
                    min_size=self.min_pool_size,
                    max_size=self.max_pool_size
                )
            else:
                self._pool = await asyncpg.create_pool(
                    host=self.host,
                    port=self.port,
                    database=self.database,
                    user=self.user,
                    password=self.password,
                    min_size=self.min_pool_size,
                    max_size=self.max_pool_size
                )

            # Initialize schema
            await self._initialize_schema()

            logger.info("PostgreSQL storage connected")

        except Exception as e:
            logger.error(f"Failed to connect to PostgreSQL: {e}")
            raise

    async def disconnect(self) -> None:
        """Disconnect from PostgreSQL"""
        if self._pool:
            await self._pool.close()
        logger.info("PostgreSQL storage disconnected")

    async def _initialize_schema(self) -> None:
        """Create database schema if not exists"""
        async with self._pool.acquire() as conn:
            # Create schema
            await conn.execute(f"""
                CREATE SCHEMA IF NOT EXISTS {self.schema}
            """)

            # Create agents table
            await conn.execute(f"""
                CREATE TABLE IF NOT EXISTS {self.schema}.agents (
                    agent_id VARCHAR(255) PRIMARY KEY,
                    name VARCHAR(255) NOT NULL,
                    version VARCHAR(50) NOT NULL,
                    status VARCHAR(50) NOT NULL,
                    health_status VARCHAR(50) NOT NULL,
                    capabilities JSONB NOT NULL,
                    endpoints JSONB NOT NULL,
                    metadata JSONB NOT NULL DEFAULT '{{}}',
                    metrics JSONB NOT NULL DEFAULT '{{}}',
                    created_at TIMESTAMPTZ NOT NULL,
                    last_heartbeat TIMESTAMPTZ NOT NULL,
                    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
                )
            """)

            # Create indices for efficient querying
            await conn.execute(f"""
                CREATE INDEX IF NOT EXISTS idx_agents_status 
                ON {self.schema}.agents(status)
            """)

            await conn.execute(f"""
                CREATE INDEX IF NOT EXISTS idx_agents_health 
                ON {self.schema}.agents(health_status)
            """)

            await conn.execute(f"""
                CREATE INDEX IF NOT EXISTS idx_agents_heartbeat 
                ON {self.schema}.agents(last_heartbeat)
            """)

            await conn.execute(f"""
                CREATE INDEX IF NOT EXISTS idx_agents_capabilities 
                ON {self.schema}.agents USING GIN(capabilities)
            """)

            await conn.execute(f"""
                CREATE INDEX IF NOT EXISTS idx_agents_metadata 
                ON {self.schema}.agents USING GIN(metadata)
            """)

            # Create events table
            await conn.execute(f"""
                CREATE TABLE IF NOT EXISTS {self.schema}.events (
                    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    event_type VARCHAR(100) NOT NULL,
                    timestamp TIMESTAMPTZ NOT NULL,
                    agent_id VARCHAR(255),
                    data JSONB NOT NULL DEFAULT '{{}}',
                    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
                )
            """)

            # Create indices for events
            await conn.execute(f"""
                CREATE INDEX IF NOT EXISTS idx_events_type 
                ON {self.schema}.events(event_type)
            """)

            await conn.execute(f"""
                CREATE INDEX IF NOT EXISTS idx_events_agent 
                ON {self.schema}.events(agent_id)
            """)

            await conn.execute(f"""
                CREATE INDEX IF NOT EXISTS idx_events_timestamp 
                ON {self.schema}.events(timestamp DESC)
            """)

            # Create capability view for fast lookup
            await conn.execute(f"""
                CREATE OR REPLACE VIEW {self.schema}.agent_capabilities AS
                SELECT 
                    a.agent_id,
                    a.name,
                    a.status,
                    a.health_status,
                    jsonb_array_elements(a.capabilities) AS capability
                FROM {self.schema}.agents a
            """)

            # Create trigger for updated_at
            await conn.execute(f"""
                CREATE OR REPLACE FUNCTION {self.schema}.update_modified_column()
                RETURNS TRIGGER AS $$
                BEGIN
                    NEW.updated_at = CURRENT_TIMESTAMP;
                    RETURN NEW;
                END;
                $$ language 'plpgsql'
            """)

            await conn.execute(f"""
                DROP TRIGGER IF EXISTS update_agents_modtime ON {self.schema}.agents
            """)

            await conn.execute(f"""
                CREATE TRIGGER update_agents_modtime
                BEFORE UPDATE ON {self.schema}.agents
                FOR EACH ROW
                EXECUTE FUNCTION {self.schema}.update_modified_column()
            """)

    async def register_agent(self, registration: AgentRegistration) -> str:
        """Register a new agent in PostgreSQL"""
        async with self._pool.acquire() as conn:
            try:
                # Insert agent
                await conn.execute(f"""
                    INSERT INTO {self.schema}.agents (
                        agent_id, name, version, status, health_status,
                        capabilities, endpoints, metadata, metrics,
                        created_at, last_heartbeat
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                """,
                                   registration.agent_id,
                                   registration.name,
                                   registration.version,
                                   registration.status,
                                   registration.health_status,
                                   json.dumps([cap.model_dump() for cap in registration.capabilities]),
                                   json.dumps([ep.model_dump() for ep in registration.endpoints]),
                                   json.dumps(registration.metadata),
                                   json.dumps(registration.metrics),
                                   registration.created_at,
                                   registration.last_heartbeat
                                   )

                # Create registration event
                await conn.execute(f"""
                    INSERT INTO {self.schema}.events (
                        event_type, timestamp, agent_id, data
                    ) VALUES ($1, $2, $3, $4)
                """,
                                   "agent_registered",
                                   datetime.utcnow(),
                                   registration.agent_id,
                                   json.dumps({"registration": registration.model_dump()})
                                   )

                logger.info(f"Agent {registration.agent_id} registered in PostgreSQL")
                return registration.agent_id

            except asyncpg.UniqueViolationError:
                raise ValueError(f"Agent {registration.agent_id} already registered")

    async def deregister_agent(self, agent_id: str) -> bool:
        """Remove agent from PostgreSQL storage"""
        async with self._pool.acquire() as conn:
            async with conn.transaction():
                # Delete agent
                result = await conn.execute(f"""
                    DELETE FROM {self.schema}.agents
                    WHERE agent_id = $1
                """, agent_id)

                if result == "DELETE 0":
                    return False

                # Create deregistration event
                await conn.execute(f"""
                    INSERT INTO {self.schema}.events (
                        event_type, timestamp, agent_id, data
                    ) VALUES ($1, $2, $3, $4)
                """,
                                   "agent_deregistered",
                                   datetime.utcnow(),
                                   agent_id,
                                   json.dumps({})
                                   )

                logger.info(f"Agent {agent_id} deregistered from PostgreSQL")
                return True

    async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
        """Retrieve agent registration from PostgreSQL"""
        async with self._pool.acquire() as conn:
            row = await conn.fetchrow(f"""
                SELECT * FROM {self.schema}.agents
                WHERE agent_id = $1
            """, agent_id)

            if not row:
                return None

            return self._row_to_registration(row)

    async def query_agents(self, query: ServiceQuery) -> List[AgentRegistration]:
        """Query agents based on criteria"""
        async with self._pool.acquire() as conn:
            # Build query
            conditions = []
            params = []
            param_count = 0

            # Base query
            sql = f"""
                SELECT DISTINCT a.* FROM {self.schema}.agents a
            """

            # Join with capability view if needed
            if query.capabilities:
                sql += f"""
                    JOIN {self.schema}.agent_capabilities ac 
                    ON a.agent_id = ac.agent_id
                """

            # Add WHERE clause
            sql += " WHERE 1=1"

            # Filter by capabilities
            if query.capabilities:
                if query.require_all:
                    # Need all capabilities
                    cap_conditions = []
                    for cap in query.capabilities:
                        param_count += 1
                        cap_conditions.append(f"""
                            EXISTS (
                                SELECT 1 FROM jsonb_array_elements(a.capabilities) c
                                WHERE c->>'name' = ${param_count}
                            )
                        """)
                        params.append(cap)
                    conditions.append(f"({' AND '.join(cap_conditions)})")
                else:
                    # Need any capability
                    param_count += 1
                    conditions.append(f"ac.capability->>'name' = ANY(${param_count})")
                    params.append(query.capabilities)

            # Filter by status
            if query.status:
                param_count += 1
                conditions.append(f"a.status = ${param_count}")
                params.append(query.status)

            # Filter by health
            if query.health_status:
                param_count += 1
                conditions.append(f"a.health_status = ${param_count}")
                params.append(query.health_status)

            # Filter by tags
            if query.tags:
                param_count += 1
                conditions.append(f"a.metadata->'tags' ?| ${param_count}")
                params.append(query.tags)

            # Filter by metadata
            if query.metadata_filter:
                for key, value in query.metadata_filter.items():
                    param_count += 1
                    conditions.append(f"a.metadata->>'{key}' = ${param_count}")
                    params.append(str(value))

            # Add conditions to query
            if conditions:
                sql += " AND " + " AND ".join(conditions)

            # Add sorting
            if query.sort_by:
                reverse = query.sort_by.startswith("-")
                field = query.sort_by.lstrip("-")

                order_map = {
                    "created_at": "a.created_at",
                    "last_heartbeat": "a.last_heartbeat",
                    "agent_id": "a.agent_id",
                    "name": "a.name"
                }

                if field in order_map:
                    sql += f" ORDER BY {order_map[field]}"
                    sql += " DESC" if reverse else " ASC"

            # Add pagination
            if query.limit:
                param_count += 1
                sql += f" LIMIT ${param_count}"
                params.append(query.limit)

                if query.offset:
                    param_count += 1
                    sql += f" OFFSET ${param_count}"
                    params.append(query.offset)

            # Execute query
            rows = await conn.fetch(sql, *params)

            return [self._row_to_registration(row) for row in rows]

    async def update_health(
            self,
            agent_id: str,
            health: HealthStatus,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Update agent health status in PostgreSQL"""
        async with self._pool.acquire() as conn:
            async with conn.transaction():
                # Update agent
                result = await conn.execute(f"""
                    UPDATE {self.schema}.agents
                    SET health_status = $1,
                        last_heartbeat = $2,
                        metrics = COALESCE($3, metrics)
                    WHERE agent_id = $4
                """,
                                            health,
                                            datetime.utcnow(),
                                            json.dumps(metrics) if metrics else None,
                                            agent_id
                                            )

                if result == "UPDATE 0":
                    return False

                # Create health update event
                await conn.execute(f"""
                    INSERT INTO {self.schema}.events (
                        event_type, timestamp, agent_id, data
                    ) VALUES ($1, $2, $3, $4)
                """,
                                   "health_updated",
                                   datetime.utcnow(),
                                   agent_id,
                                   json.dumps({"health": health, "metrics": metrics})
                                   )

                return True

    async def update_capabilities(
            self,
            agent_id: str,
            capabilities: List[Capability]
    ) -> bool:
        """Update agent capabilities in PostgreSQL"""
        async with self._pool.acquire() as conn:
            async with conn.transaction():
                # Update capabilities
                result = await conn.execute(f"""
                    UPDATE {self.schema}.agents
                    SET capabilities = $1
                    WHERE agent_id = $2
                """,
                                            json.dumps([cap.model_dump() for cap in capabilities]),
                                            agent_id
                                            )

                if result == "UPDATE 0":
                    return False

                # Create capability update event
                await conn.execute(f"""
                    INSERT INTO {self.schema}.events (
                        event_type, timestamp, agent_id, data
                    ) VALUES ($1, $2, $3, $4)
                """,
                                   "capabilities_updated",
                                   datetime.utcnow(),
                                   agent_id,
                                   json.dumps({
                                       "capabilities": [cap.model_dump() for cap in capabilities]
                                   })
                                   )

                return True

    async def get_events(
            self,
            agent_id: Optional[str] = None,
            event_type: Optional[str] = None,
            since: Optional[datetime] = None,
            limit: int = 100
    ) -> List[RegistryEvent]:
        """Retrieve registry events from PostgreSQL"""
        async with self._pool.acquire() as conn:
            # Build query
            conditions = []
            params = []
            param_count = 0

            sql = f"""
                SELECT * FROM {self.schema}.events
                WHERE 1=1
            """

            # Filter by agent_id
            if agent_id:
                param_count += 1
                conditions.append(f"agent_id = ${param_count}")
                params.append(agent_id)

            # Filter by event type
            if event_type:
                param_count += 1
                conditions.append(f"event_type = ${param_count}")
                params.append(event_type)

            # Filter by timestamp
            if since:
                param_count += 1
                conditions.append(f"timestamp >= ${param_count}")
                params.append(since)

            # Add conditions
            if conditions:
                sql += " AND " + " AND ".join(conditions)

            # Order by timestamp desc
            sql += " ORDER BY timestamp DESC"

            # Add limit
            param_count += 1
            sql += f" LIMIT ${param_count}"
            params.append(limit)

            # Execute query
            rows = await conn.fetch(sql, *params)

            return [self._row_to_event(row) for row in rows]

    async def clean_expired(self, ttl: timedelta) -> int:
        """Remove expired agents from PostgreSQL"""
        async with self._pool.acquire() as conn:
            cutoff_time = datetime.utcnow() - ttl

            async with conn.transaction():
                # Get expired agents
                expired_rows = await conn.fetch(f"""
                    SELECT agent_id, last_heartbeat 
                    FROM {self.schema}.agents
                    WHERE last_heartbeat < $1
                """, cutoff_time)

                if not expired_rows:
                    return 0

                # Delete expired agents
                expired_ids = [row['agent_id'] for row in expired_rows]
                await conn.execute(f"""
                    DELETE FROM {self.schema}.agents
                    WHERE agent_id = ANY($1)
                """, expired_ids)

                # Create expiration events
                for row in expired_rows:
                    await conn.execute(f"""
                        INSERT INTO {self.schema}.events (
                            event_type, timestamp, agent_id, data
                        ) VALUES ($1, $2, $3, $4)
                    """,
                                       "agent_expired",
                                       datetime.utcnow(),
                                       row['agent_id'],
                                       json.dumps({
                                           "last_heartbeat": row['last_heartbeat'].isoformat()
                                       })
                                       )

                logger.info(f"Cleaned {len(expired_ids)} expired agents from PostgreSQL")
                return len(expired_ids)

    async def get_statistics(self) -> Dict[str, Any]:
        """Get registry statistics from PostgreSQL"""
        async with self._pool.acquire() as conn:
            # Get total agents
            total_agents = await conn.fetchval(f"""
                SELECT COUNT(*) FROM {self.schema}.agents
            """)

            # Get status counts
            status_rows = await conn.fetch(f"""
                SELECT status, COUNT(*) as count
                FROM {self.schema}.agents
                GROUP BY status
            """)
            status_counts = {row['status']: row['count'] for row in status_rows}

            # Get health counts
            health_rows = await conn.fetch(f"""
                SELECT health_status, COUNT(*) as count
                FROM {self.schema}.agents
                GROUP BY health_status
            """)
            health_counts = {row['health_status']: row['count'] for row in health_rows}

            # Get capability counts
            cap_rows = await conn.fetch(f"""
                SELECT capability->>'name' as name, COUNT(DISTINCT agent_id) as count
                FROM {self.schema}.agent_capabilities
                GROUP BY capability->>'name'
            """)
            capability_counts = {row['name']: row['count'] for row in cap_rows}

            # Get event count
            total_events = await conn.fetchval(f"""
                SELECT COUNT(*) FROM {self.schema}.events
            """)

            # Get database size
            db_size = await conn.fetchval(f"""
                SELECT pg_database_size($1)
            """, self.database)

            # Get table sizes
            table_sizes = {}
            for table in ['agents', 'events']:
                size = await conn.fetchval(f"""
                    SELECT pg_total_relation_size('{self.schema}.{table}')
                """)
                table_sizes[table] = size

            return {
                "total_agents": total_agents,
                "status_counts": status_counts,
                "health_counts": health_counts,
                "capability_counts": capability_counts,
                "total_events": total_events,
                "storage_type": "postgresql",
                "database_size": db_size,
                "table_sizes": table_sizes
            }

    # Private helper methods

    def _row_to_registration(self, row: asyncpg.Record) -> AgentRegistration:
        """Convert database row to AgentRegistration"""
        return AgentRegistration(
            agent_id=row['agent_id'],
            name=row['name'],
            version=row['version'],
            status=row['status'],
            health_status=row['health_status'],
            capabilities=[
                Capability(**cap)
                for cap in json.loads(row['capabilities'])
            ],
            endpoints=[
                Endpoint(**ep)
                for ep in json.loads(row['endpoints'])
            ],
            metadata=json.loads(row['metadata']),
            metrics=json.loads(row['metrics']),
            created_at=row['created_at'],
            last_heartbeat=row['last_heartbeat']
        )

    def _row_to_event(self, row: asyncpg.Record) -> RegistryEvent:
        """Convert database row to RegistryEvent"""
        return RegistryEvent(
            event_id=str(row['event_id']),
            event_type=row['event_type'],
            timestamp=row['timestamp'],
            agent_id=row['agent_id'],
            data=json.loads(row['data'])
        )

    # Context manager support

    async def __aenter__(self):
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.disconnect()


# Advanced PostgreSQL storage with partitioning

class PartitionedPostgresStorage(PostgresStorage):
    """PostgreSQL storage with time-based partitioning for events"""

    async def _initialize_schema(self) -> None:
        """Create schema with partitioned tables"""
        await super()._initialize_schema()

        async with self._pool.acquire() as conn:
            # Create partitioned events table
            await conn.execute(f"""
                CREATE TABLE IF NOT EXISTS {self.schema}.events_partitioned (
                    event_id UUID DEFAULT gen_random_uuid(),
                    event_type VARCHAR(100) NOT NULL,
                    timestamp TIMESTAMPTZ NOT NULL,
                    agent_id VARCHAR(255),
                    data JSONB NOT NULL DEFAULT '{{}}',
                    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                    PRIMARY KEY (event_id, timestamp)
                ) PARTITION BY RANGE (timestamp)
            """)

            # Create function to automatically create monthly partitions
            await conn.execute(f"""
                CREATE OR REPLACE FUNCTION {self.schema}.create_monthly_partition()
                RETURNS void AS $$
                DECLARE
                    start_date date;
                    end_date date;
                    partition_name text;
                BEGIN
                    start_date := date_trunc('month', CURRENT_DATE);
                    end_date := start_date + interval '1 month';
                    partition_name := '{self.schema}.events_' || 
                                    to_char(start_date, 'YYYY_MM');

                    EXECUTE format(
                        'CREATE TABLE IF NOT EXISTS %I PARTITION OF %I.events_partitioned 
                        FOR VALUES FROM (%L) TO (%L)',
                        partition_name,
                        '{self.schema}',
                        start_date,
                        end_date
                    );
                END;
                $$ LANGUAGE plpgsql
            """)

            # Create current month partition
            await conn.execute(f"SELECT {self.schema}.create_monthly_partition()")

            # Create scheduled job to create future partitions
            await conn.execute(f"""
                CREATE OR REPLACE FUNCTION {self.schema}.maintain_partitions()
                RETURNS void AS $$
                BEGIN
                    -- Create next month's partition
                    PERFORM {self.schema}.create_monthly_partition();

                    -- Drop old partitions (older than 6 months)
                    EXECUTE format(
                        'DROP TABLE IF EXISTS {self.schema}.events_%s',
                        to_char(CURRENT_DATE - interval '6 months', 'YYYY_MM')
                    );
                END;
                $$ LANGUAGE plpgsql
            """)


# Export storage implementations
__all__ = [
    "PostgresStorage",
    "PartitionedPostgresStorage"
]