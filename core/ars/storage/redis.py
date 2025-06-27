# File: core/ars/storage/redis.py
# Description: Redis-based storage implementation for production deployments.
# Provides distributed, persistent storage with high performance and reliability.

from __future__ import annotations
import asyncio
import json
import uuid
from datetime import datetime, timedelta
from typing import List, Optional, Dict, Any, Set
import logging
from contextlib import asynccontextmanager

import redis.asyncio as redis
from redis.asyncio import ConnectionPool
from redis.asyncio.sentinel import Sentinel

from core.ars.models.registry import (
    AgentRegistration, ServiceQuery, Capability,
    AgentStatus, HealthStatus, RegistryEvent
)
from core.ars.storage.interface import RegistryStorage

logger = logging.getLogger(__name__)


class RedisStorage(RegistryStorage):
    """Redis implementation of registry storage"""

    def __init__(
            self,
            host: str = "localhost",
            port: int = 6379,
            db: int = 0,
            password: Optional[str] = None,
            sentinel_hosts: Optional[List[tuple[str, int]]] = None,
            master_name: Optional[str] = None,
            connection_pool: Optional[ConnectionPool] = None,
            key_prefix: str = "maple:ars:"
    ):
        self.host = host
        self.port = port
        self.db = db
        self.password = password
        self.sentinel_hosts = sentinel_hosts
        self.master_name = master_name
        self.connection_pool = connection_pool
        self.key_prefix = key_prefix
        self._client: Optional[redis.Redis] = None
        self._sentinel: Optional[Sentinel] = None
        self._pubsub: Optional[redis.client.PubSub] = None

    async def connect(self) -> None:
        """Connect to Redis"""
        try:
            if self.sentinel_hosts and self.master_name:
                # Use Redis Sentinel for HA
                self._sentinel = Sentinel(
                    self.sentinel_hosts,
                    password=self.password,
                    db=self.db
                )
                self._client = self._sentinel.master_for(
                    self.master_name,
                    redis_class=redis.Redis
                )
            elif self.connection_pool:
                # Use provided connection pool
                self._client = redis.Redis(connection_pool=self.connection_pool)
            else:
                # Direct connection
                self._client = redis.Redis(
                    host=self.host,
                    port=self.port,
                    db=self.db,
                    password=self.password,
                    decode_responses=True
                )

            # Test connection
            await self._client.ping()

            # Setup pub/sub for events
            self._pubsub = self._client.pubsub()

            logger.info("Redis storage connected")

        except Exception as e:
            logger.error(f"Failed to connect to Redis: {e}")
            raise

    async def disconnect(self) -> None:
        """Disconnect from Redis"""
        if self._pubsub:
            await self._pubsub.close()
        if self._client:
            await self._client.close()
        logger.info("Redis storage disconnected")

    async def register_agent(self, registration: AgentRegistration) -> str:
        """Register a new agent in Redis"""
        agent_key = self._get_agent_key(registration.agent_id)

        # Check if already exists
        exists = await self._client.exists(agent_key)
        if exists:
            raise ValueError(f"Agent {registration.agent_id} already registered")

        # Prepare agent data
        agent_data = self._serialize_registration(registration)

        # Transaction to ensure atomicity
        async with self._client.pipeline(transaction=True) as pipe:
            # Store agent data
            await pipe.hset(agent_key, mapping=agent_data)

            # Add to indices
            await self._add_to_indices(pipe, registration)

            # Add to all agents set
            await pipe.sadd(self._get_all_agents_key(), registration.agent_id)

            # Set TTL if needed
            if hasattr(registration, 'ttl') and registration.ttl:
                await pipe.expire(agent_key, registration.ttl)

            # Create registration event
            event = RegistryEvent(
                event_type="agent_registered",
                timestamp=datetime.utcnow(),
                agent_id=registration.agent_id,
                data={"registration": registration.model_dump()}
            )
            await self._store_event(pipe, event)

            # Execute transaction
            await pipe.execute()

        # Publish event
        await self._publish_event(event)

        logger.info(f"Agent {registration.agent_id} registered in Redis")
        return registration.agent_id

    async def deregister_agent(self, agent_id: str) -> bool:
        """Remove agent from Redis storage"""
        agent_key = self._get_agent_key(agent_id)

        # Get agent data first
        agent_data = await self._client.hgetall(agent_key)
        if not agent_data:
            return False

        registration = self._deserialize_registration(agent_data)

        # Transaction to ensure atomicity
        async with self._client.pipeline(transaction=True) as pipe:
            # Remove from indices
            await self._remove_from_indices(pipe, registration)

            # Remove from all agents set
            await pipe.srem(self._get_all_agents_key(), agent_id)

            # Delete agent data
            await pipe.delete(agent_key)

            # Create deregistration event
            event = RegistryEvent(
                event_type="agent_deregistered",
                timestamp=datetime.utcnow(),
                agent_id=agent_id,
                data={}
            )
            await self._store_event(pipe, event)

            # Execute transaction
            await pipe.execute()

        # Publish event
        await self._publish_event(event)

        logger.info(f"Agent {agent_id} deregistered from Redis")
        return True

    async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
        """Retrieve agent registration from Redis"""
        agent_key = self._get_agent_key(agent_id)
        agent_data = await self._client.hgetall(agent_key)

        if not agent_data:
            return None

        return self._deserialize_registration(agent_data)

    async def query_agents(self, query: ServiceQuery) -> List[AgentRegistration]:
        """Query agents based on criteria"""
        # Start with all agents
        agent_ids = await self._get_filtered_agent_ids(query)

        # Fetch agent data
        agents = []
        if agent_ids:
            # Use pipeline for efficiency
            async with self._client.pipeline() as pipe:
                for agent_id in agent_ids:
                    pipe.hgetall(self._get_agent_key(agent_id))

                results = await pipe.execute()

                for agent_data in results:
                    if agent_data:
                        agent = self._deserialize_registration(agent_data)
                        if self._matches_query(agent, query):
                            agents.append(agent)

        # Apply sorting
        if query.sort_by:
            agents = self._sort_results(agents, query.sort_by)

        # Apply pagination
        if query.limit:
            start = query.offset or 0
            agents = agents[start:start + query.limit]

        return agents

    async def update_health(
            self,
            agent_id: str,
            health: HealthStatus,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Update agent health status in Redis"""
        agent_key = self._get_agent_key(agent_id)

        # Check if agent exists
        exists = await self._client.exists(agent_key)
        if not exists:
            return False

        # Update health and heartbeat
        updates = {
            "health_status": health,
            "last_heartbeat": datetime.utcnow().isoformat()
        }

        if metrics:
            updates["metrics"] = json.dumps(metrics)

        await self._client.hset(agent_key, mapping=updates)

        # Create health update event
        event = RegistryEvent(
            event_type="health_updated",
            timestamp=datetime.utcnow(),
            agent_id=agent_id,
            data={"health": health, "metrics": metrics}
        )
        await self._store_event(None, event)
        await self._publish_event(event)

        return True

    async def update_capabilities(
            self,
            agent_id: str,
            capabilities: List[Capability]
    ) -> bool:
        """Update agent capabilities in Redis"""
        agent_key = self._get_agent_key(agent_id)

        # Get current agent data
        agent_data = await self._client.hgetall(agent_key)
        if not agent_data:
            return False

        registration = self._deserialize_registration(agent_data)
        old_capabilities = registration.capabilities

        # Transaction to update capabilities and indices
        async with self._client.pipeline(transaction=True) as pipe:
            # Remove from old capability indices
            for cap in old_capabilities:
                await pipe.srem(
                    self._get_capability_key(cap.name),
                    agent_id
                )

            # Add to new capability indices
            for cap in capabilities:
                await pipe.sadd(
                    self._get_capability_key(cap.name),
                    agent_id
                )

            # Update agent data
            cap_data = json.dumps([cap.model_dump() for cap in capabilities])
            await pipe.hset(agent_key, "capabilities", cap_data)

            # Create capability update event
            event = RegistryEvent(
                event_type="capabilities_updated",
                timestamp=datetime.utcnow(),
                agent_id=agent_id,
                data={"capabilities": [cap.model_dump() for cap in capabilities]}
            )
            await self._store_event(pipe, event)

            # Execute transaction
            await pipe.execute()

        # Publish event
        await self._publish_event(event)

        return True

    async def get_events(
            self,
            agent_id: Optional[str] = None,
            event_type: Optional[str] = None,
            since: Optional[datetime] = None,
            limit: int = 100
    ) -> List[RegistryEvent]:
        """Retrieve registry events from Redis"""
        events_key = self._get_events_key()

        # Get all events (newest first)
        event_data = await self._client.lrange(events_key, 0, -1)

        results = []
        for data in event_data:
            event = self._deserialize_event(data)

            # Filter by agent_id
            if agent_id and event.agent_id != agent_id:
                continue

            # Filter by event type
            if event_type and event.event_type != event_type:
                continue

            # Filter by timestamp
            if since and event.timestamp < since:
                continue

            results.append(event)

            if len(results) >= limit:
                break

        return results

    async def clean_expired(self, ttl: timedelta) -> int:
        """Remove expired agents from Redis"""
        cutoff_time = datetime.utcnow() - ttl
        expired_count = 0

        # Get all agents
        agent_ids = await self._client.smembers(self._get_all_agents_key())

        for agent_id in agent_ids:
            agent_key = self._get_agent_key(agent_id)

            # Check last heartbeat
            heartbeat_str = await self._client.hget(agent_key, "last_heartbeat")
            if heartbeat_str:
                last_heartbeat = datetime.fromisoformat(heartbeat_str)

                if last_heartbeat < cutoff_time:
                    # Deregister expired agent
                    if await self.deregister_agent(agent_id):
                        expired_count += 1

                        # Create expiration event
                        event = RegistryEvent(
                            event_type="agent_expired",
                            timestamp=datetime.utcnow(),
                            agent_id=agent_id,
                            data={"last_heartbeat": heartbeat_str}
                        )
                        await self._store_event(None, event)
                        await self._publish_event(event)

        if expired_count > 0:
            logger.info(f"Cleaned {expired_count} expired agents from Redis")

        return expired_count

    async def get_statistics(self) -> Dict[str, Any]:
        """Get registry statistics from Redis"""
        # Get total agents
        total_agents = await self._client.scard(self._get_all_agents_key())

        # Get all agents for detailed stats
        agent_ids = await self._client.smembers(self._get_all_agents_key())

        status_counts = {}
        health_counts = {}

        if agent_ids:
            # Use pipeline for efficiency
            async with self._client.pipeline() as pipe:
                for agent_id in agent_ids:
                    pipe.hmget(
                        self._get_agent_key(agent_id),
                        ["status", "health_status"]
                    )

                results = await pipe.execute()

                for status, health in results:
                    if status:
                        status_counts[status] = status_counts.get(status, 0) + 1
                    if health:
                        health_counts[health] = health_counts.get(health, 0) + 1

        # Get capability statistics
        capability_counts = {}
        cap_pattern = f"{self.key_prefix}capability:*"

        cursor = 0
        while True:
            cursor, keys = await self._client.scan(
                cursor,
                match=cap_pattern,
                count=100
            )

            if keys:
                async with self._client.pipeline() as pipe:
                    for key in keys:
                        pipe.scard(key)

                    counts = await pipe.execute()

                    for key, count in zip(keys, counts):
                        cap_name = key.split(":")[-1]
                        capability_counts[cap_name] = count

            if cursor == 0:
                break

        # Get event count
        total_events = await self._client.llen(self._get_events_key())

        return {
            "total_agents": total_agents,
            "status_counts": status_counts,
            "health_counts": health_counts,
            "capability_counts": capability_counts,
            "total_events": total_events,
            "storage_type": "redis",
            "redis_info": await self._client.info()
        }

    # Private helper methods

    def _get_agent_key(self, agent_id: str) -> str:
        """Get Redis key for agent data"""
        return f"{self.key_prefix}agent:{agent_id}"

    def _get_all_agents_key(self) -> str:
        """Get Redis key for all agents set"""
        return f"{self.key_prefix}agents:all"

    def _get_capability_key(self, capability: str) -> str:
        """Get Redis key for capability index"""
        return f"{self.key_prefix}capability:{capability}"

    def _get_tag_key(self, tag: str) -> str:
        """Get Redis key for tag index"""
        return f"{self.key_prefix}tag:{tag}"

    def _get_status_key(self, status: str) -> str:
        """Get Redis key for status index"""
        return f"{self.key_prefix}status:{status}"

    def _get_health_key(self, health: str) -> str:
        """Get Redis key for health index"""
        return f"{self.key_prefix}health:{health}"

    def _get_events_key(self) -> str:
        """Get Redis key for events list"""
        return f"{self.key_prefix}events"

    def _get_event_channel(self) -> str:
        """Get Redis pubsub channel for events"""
        return f"{self.key_prefix}events:channel"

    def _serialize_registration(self, registration: AgentRegistration) -> Dict[str, str]:
        """Serialize agent registration for Redis storage"""
        return {
            "agent_id": registration.agent_id,
            "name": registration.name,
            "version": registration.version,
            "status": registration.status,
            "health_status": registration.health_status,
            "capabilities": json.dumps([cap.model_dump() for cap in registration.capabilities]),
            "endpoints": json.dumps([ep.model_dump() for ep in registration.endpoints]),
            "metadata": json.dumps(registration.metadata),
            "metrics": json.dumps(registration.metrics),
            "created_at": registration.created_at.isoformat(),
            "last_heartbeat": registration.last_heartbeat.isoformat()
        }

    def _deserialize_registration(self, data: Dict[str, str]) -> AgentRegistration:
        """Deserialize agent registration from Redis data"""
        return AgentRegistration(
            agent_id=data["agent_id"],
            name=data["name"],
            version=data["version"],
            status=data["status"],
            health_status=data["health_status"],
            capabilities=[
                Capability(**cap)
                for cap in json.loads(data["capabilities"])
            ],
            endpoints=[
                Endpoint(**ep)
                for ep in json.loads(data["endpoints"])
            ],
            metadata=json.loads(data["metadata"]),
            metrics=json.loads(data["metrics"]),
            created_at=datetime.fromisoformat(data["created_at"]),
            last_heartbeat=datetime.fromisoformat(data["last_heartbeat"])
        )

    def _serialize_event(self, event: RegistryEvent) -> str:
        """Serialize event for Redis storage"""
        return json.dumps({
            "event_id": event.event_id,
            "event_type": event.event_type,
            "timestamp": event.timestamp.isoformat(),
            "agent_id": event.agent_id,
            "data": event.data
        })

    def _deserialize_event(self, data: str) -> RegistryEvent:
        """Deserialize event from Redis data"""
        event_data = json.loads(data)
        return RegistryEvent(
            event_id=event_data["event_id"],
            event_type=event_data["event_type"],
            timestamp=datetime.fromisoformat(event_data["timestamp"]),
            agent_id=event_data.get("agent_id"),
            data=event_data["data"]
        )

    async def _add_to_indices(
            self,
            pipe: redis.client.Pipeline,
            registration: AgentRegistration
    ) -> None:
        """Add agent to various indices"""
        # Capability index
        for cap in registration.capabilities:
            await pipe.sadd(
                self._get_capability_key(cap.name),
                registration.agent_id
            )

        # Tag index
        for tag in registration.metadata.get("tags", []):
            await pipe.sadd(
                self._get_tag_key(tag),
                registration.agent_id
            )

        # Status index
        await pipe.sadd(
            self._get_status_key(registration.status),
            registration.agent_id
        )

        # Health index
        await pipe.sadd(
            self._get_health_key(registration.health_status),
            registration.agent_id
        )

    async def _remove_from_indices(
            self,
            pipe: redis.client.Pipeline,
            registration: AgentRegistration
    ) -> None:
        """Remove agent from various indices"""
        # Capability index
        for cap in registration.capabilities:
            await pipe.srem(
                self._get_capability_key(cap.name),
                registration.agent_id
            )

        # Tag index
        for tag in registration.metadata.get("tags", []):
            await pipe.srem(
                self._get_tag_key(tag),
                registration.agent_id
            )

        # Status index
        await pipe.srem(
            self._get_status_key(registration.status),
            registration.agent_id
        )

        # Health index
        await pipe.srem(
            self._get_health_key(registration.health_status),
            registration.agent_id
        )

    async def _get_filtered_agent_ids(self, query: ServiceQuery) -> Set[str]:
        """Get agent IDs matching query filters"""
        sets_to_intersect = []

        # Start with all agents
        base_set = self._get_all_agents_key()

        # Filter by capabilities
        if query.capabilities:
            if query.require_all:
                # Need all capabilities - intersect all sets
                for cap in query.capabilities:
                    sets_to_intersect.append(self._get_capability_key(cap))
            else:
                # Need any capability - union then intersect
                cap_keys = [
                    self._get_capability_key(cap)
                    for cap in query.capabilities
                ]
                # Create temporary union set
                temp_key = f"{self.key_prefix}temp:{uuid.uuid4()}"
                await self._client.sunionstore(temp_key, *cap_keys)
                sets_to_intersect.append(temp_key)

        # Filter by status
        if query.status:
            sets_to_intersect.append(self._get_status_key(query.status))

        # Filter by health
        if query.health_status:
            sets_to_intersect.append(self._get_health_key(query.health_status))

        # Filter by tags
        if query.tags:
            tag_keys = [self._get_tag_key(tag) for tag in query.tags]
            # Create temporary union of tags
            temp_key = f"{self.key_prefix}temp:{uuid.uuid4()}"
            await self._client.sunionstore(temp_key, *tag_keys)
            sets_to_intersect.append(temp_key)

        # Perform intersection
        if sets_to_intersect:
            result_key = f"{self.key_prefix}temp:{uuid.uuid4()}"
            await self._client.sinterstore(result_key, base_set, *sets_to_intersect)
            agent_ids = await self._client.smembers(result_key)

            # Clean up temporary keys
            temp_keys = [
                key for key in sets_to_intersect
                if key.startswith(f"{self.key_prefix}temp:")
            ]
            temp_keys.append(result_key)
            if temp_keys:
                await self._client.delete(*temp_keys)

            return agent_ids
        else:
            return await self._client.smembers(base_set)

    def _matches_query(self, agent: AgentRegistration, query: ServiceQuery) -> bool:
        """Check if agent matches additional query criteria"""
        # Check metadata filters
        if query.metadata_filter:
            for key, value in query.metadata_filter.items():
                if agent.metadata.get(key) != value:
                    return False

        return True

    def _sort_results(
            self,
            results: List[AgentRegistration],
            sort_by: str
    ) -> List[AgentRegistration]:
        """Sort query results by specified field"""
        reverse = sort_by.startswith("-")
        field = sort_by.lstrip("-")

        if field == "created_at":
            results.sort(key=lambda x: x.created_at, reverse=reverse)
        elif field == "last_heartbeat":
            results.sort(key=lambda x: x.last_heartbeat, reverse=reverse)
        elif field == "agent_id":
            results.sort(key=lambda x: x.agent_id, reverse=reverse)
        elif field == "name":
            results.sort(key=lambda x: x.name, reverse=reverse)

        return results

    async def _store_event(
            self,
            pipe: Optional[redis.client.Pipeline],
            event: RegistryEvent
    ) -> None:
        """Store event in Redis"""
        event_data = self._serialize_event(event)
        events_key = self._get_events_key()

        if pipe:
            # Add to pipeline
            await pipe.lpush(events_key, event_data)
            await pipe.ltrim(events_key, 0, 9999)  # Keep last 10k events
        else:
            # Direct execution
            await self._client.lpush(events_key, event_data)
            await self._client.ltrim(events_key, 0, 9999)

    async def _publish_event(self, event: RegistryEvent) -> None:
        """Publish event to Redis pubsub"""
        channel = self._get_event_channel()
        event_data = self._serialize_event(event)
        await self._client.publish(channel, event_data)

    # Context manager support

    async def __aenter__(self):
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.disconnect()


# Cluster-aware Redis storage

class RedisClusterStorage(RedisStorage):
    """Redis Cluster implementation for horizontal scaling"""

    def __init__(
            self,
            startup_nodes: List[Dict[str, Any]],
            password: Optional[str] = None,
            key_prefix: str = "maple:ars:"
    ):
        from redis.asyncio.cluster import RedisCluster

        self.startup_nodes = startup_nodes
        self.password = password
        self.key_prefix = key_prefix
        self._client: Optional[RedisCluster] = None

    async def connect(self) -> None:
        """Connect to Redis Cluster"""
        from redis.asyncio.cluster import RedisCluster

        try:
            self._client = RedisCluster(
                startup_nodes=self.startup_nodes,
                password=self.password,
                decode_responses=True
            )

            # Test connection
            await self._client.ping()

            logger.info("Redis Cluster storage connected")

        except Exception as e:
            logger.error(f"Failed to connect to Redis Cluster: {e}")
            raise


# Export storage implementations
__all__ = [
    "RedisStorage",
    "RedisClusterStorage"
]
