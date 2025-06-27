# File: core/ars/storage/memory.py
# Description: In-memory storage implementation for development and testing.
# Provides fast, thread-safe storage with full query capabilities.

from __future__ import annotations
import asyncio
from collections import defaultdict
from datetime import datetime, timedelta
from typing import List, Optional, Dict, Any, Set
import logging

from core.ars.models.registry import (
    AgentRegistration, ServiceQuery, Capability,
    AgentStatus, HealthStatus, RegistryEvent
)
from core.ars.storage.interface import RegistryStorage

logger = logging.getLogger(__name__)


class InMemoryStorage(RegistryStorage):
    """In-memory implementation of registry storage"""

    def __init__(self):
        self.agents: Dict[str, AgentRegistration] = {}
        self.events: List[RegistryEvent] = []
        self.capability_index: Dict[str, Set[str]] = defaultdict(set)  # capability -> agent_ids
        self.tag_index: Dict[str, Set[str]] = defaultdict(set)  # tag -> agent_ids
        self._lock = asyncio.Lock()
        self._connected = False

    async def connect(self) -> None:
        """Connect to storage (no-op for in-memory)"""
        self._connected = True
        logger.info("In-memory storage connected")

    async def disconnect(self) -> None:
        """Disconnect from storage (no-op for in-memory)"""
        self._connected = False
        logger.info("In-memory storage disconnected")


    async def register_agent(self, registration: AgentRegistration) -> str:
        """Register a new agent in memory"""
        async with self._lock:
            if registration.agent_id in self.agents:
                raise ValueError(f"Agent {registration.agent_id} already registered")

            # Store agent
            self.agents[registration.agent_id] = registration

            # Update indices
            self._update_indices(registration)

            # Create registration event
            event = RegistryEvent(
                event_type="agent_registered",
                timestamp=datetime.utcnow(),
                agent_id=registration.agent_id,
                data={"registration": registration.model_dump()}
            )
            self.events.append(event)

            logger.info(f"Agent {registration.agent_id} registered in memory")
            return registration.agent_id


    async def deregister_agent(self, agent_id: str) -> bool:
        """Remove agent from memory storage"""
        async with self._lock:
            if agent_id not in self.agents:
                return False

            registration = self.agents[agent_id]

            # Remove from indices
            self._remove_from_indices(registration)

            # Remove agent
            del self.agents[agent_id]

            # Create deregistration event
            event = RegistryEvent(
                event_type="agent_deregistered",
                timestamp=datetime.utcnow(),
                agent_id=agent_id,
                data={}
            )
            self.events.append(event)

            logger.info(f"Agent {agent_id} deregistered from memory")
            return True


    async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
        """Retrieve agent registration from memory"""
        async with self._lock:
            return self.agents.get(agent_id)


    async def query_agents(self, query: ServiceQuery) -> List[AgentRegistration]:
        """Query agents based on criteria"""
        async with self._lock:
            results = []

            for agent in self.agents.values():
                if self._matches_query(agent, query):
                    results.append(agent)

            # Apply sorting
            if query.sort_by:
                results = self._sort_results(results, query.sort_by)

            # Apply pagination
            if query.limit:
                start = query.offset or 0
                results = results[start:start + query.limit]

            return results


    async def update_health(
            self,
            agent_id: str,
            health: HealthStatus,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Update agent health status in memory"""
        async with self._lock:
            if agent_id not in self.agents:
                return False

            agent = self.agents[agent_id]
            agent.health_status = health
            agent.last_heartbeat = datetime.utcnow()

            if metrics:
                agent.metrics.update(metrics)

            # Create health update event
            event = RegistryEvent(
                event_type="health_updated",
                timestamp=datetime.utcnow(),
                agent_id=agent_id,
                data={"health": health, "metrics": metrics}
            )
            self.events.append(event)

            return True


    async def update_capabilities(
            self,
            agent_id: str,
            capabilities: List[Capability]
    ) -> bool:
        """Update agent capabilities in memory"""
        async with self._lock:
            if agent_id not in self.agents:
                return False

            agent = self.agents[agent_id]

            # Remove old capability mappings
            for cap in agent.capabilities:
                self.capability_index[cap.name].discard(agent_id)

            # Update capabilities
            agent.capabilities = capabilities

            # Add new capability mappings
            for cap in capabilities:
                self.capability_index[cap.name].add(agent_id)

            # Create capability update event
            event = RegistryEvent(
                event_type="capabilities_updated",
                timestamp=datetime.utcnow(),
                agent_id=agent_id,
                data={"capabilities": [cap.model_dump() for cap in capabilities]}
            )
            self.events.append(event)

            return True


    async def get_events(
            self,
            agent_id: Optional[str] = None,
            event_type: Optional[str] = None,
            since: Optional[datetime] = None,
            limit: int = 100
    ) -> List[RegistryEvent]:
        """Retrieve registry events from memory"""
        async with self._lock:
            results = []

            for event in reversed(self.events):  # Most recent first
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
        """Remove expired agents from memory"""
        async with self._lock:
            expired_agents = []
            cutoff_time = datetime.utcnow() - ttl

            for agent_id, agent in self.agents.items():
                if agent.last_heartbeat < cutoff_time:
                    expired_agents.append(agent_id)

            # Remove expired agents
            for agent_id in expired_agents:
                registration = self.agents[agent_id]
                self._remove_from_indices(registration)
                del self.agents[agent_id]

                # Create expiration event
                event = RegistryEvent(
                    event_type="agent_expired",
                    timestamp=datetime.utcnow(),
                    agent_id=agent_id,
                    data={"last_heartbeat": registration.last_heartbeat.isoformat()}
                )
                self.events.append(event)

            if expired_agents:
                logger.info(f"Cleaned {len(expired_agents)} expired agents")

            return len(expired_agents)


    async def get_statistics(self) -> Dict[str, Any]:
        """Get registry statistics from memory"""
        async with self._lock:
            total_agents = len(self.agents)

            # Count by status
            status_counts = defaultdict(int)
            for agent in self.agents.values():
                status_counts[agent.status] += 1

            # Count by health
            health_counts = defaultdict(int)
            for agent in self.agents.values():
                health_counts[agent.health_status] += 1

            # Capability statistics
            capability_counts = {
                cap: len(agents)
                for cap, agents in self.capability_index.items()
            }

            return {
                "total_agents": total_agents,
                "status_counts": dict(status_counts),
                "health_counts": dict(health_counts),
                "capability_counts": capability_counts,
                "total_events": len(self.events),
                "storage_type": "memory"
            }


    # Private helper methods

    def _update_indices(self, registration: AgentRegistration) -> None:
        """Update internal indices when agent is added"""
        # Update capability index
        for cap in registration.capabilities:
            self.capability_index[cap.name].add(registration.agent_id)

        # Update tag index
        for tag in registration.metadata.get("tags", []):
            self.tag_index[tag].add(registration.agent_id)


    def _remove_from_indices(self, registration: AgentRegistration) -> None:
        """Remove agent from internal indices"""
        # Remove from capability index
        for cap in registration.capabilities:
            self.capability_index[cap.name].discard(registration.agent_id)
            if not self.capability_index[cap.name]:
                del self.capability_index[cap.name]

        # Remove from tag index
        for tag in registration.metadata.get("tags", []):
            self.tag_index[tag].discard(registration.agent_id)
            if not self.tag_index[tag]:
                del self.tag_index[tag]


    def _matches_query(self, agent: AgentRegistration, query: ServiceQuery) -> bool:
        """Check if agent matches query criteria"""
        # Filter by capabilities
        if query.capabilities:
            agent_caps = {cap.name for cap in agent.capabilities}
            if query.require_all:
                if not all(cap in agent_caps for cap in query.capabilities):
                    return False
            else:
                if not any(cap in agent_caps for cap in query.capabilities):
                    return False

        # Filter by status
        if query.status and agent.status != query.status:
            return False

        # Filter by health
        if query.health_status and agent.health_status != query.health_status:
            return False

        # Filter by tags
        if query.tags:
            agent_tags = set(agent.metadata.get("tags", []))
            if not any(tag in agent_tags for tag in query.tags):
                return False

        # Filter by metadata
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

        return results


    # Specialized in-memory storage with additional features

    class CachedInMemoryStorage(InMemoryStorage):
        """In-memory storage with caching optimizations"""

        def __init__(self, cache_ttl: int = 60):
            super().__init__()
            self.cache_ttl = cache_ttl
            self.query_cache: Dict[str, tuple[List[AgentRegistration], datetime]] = {}
            self._cache_lock = asyncio.Lock()

        async def query_agents(self, query: ServiceQuery) -> List[AgentRegistration]:
            """Query agents with caching"""
            # Generate cache key
            cache_key = self._generate_cache_key(query)

            # Check cache
            async with self._cache_lock:
                if cache_key in self.query_cache:
                    results, timestamp = self.query_cache[cache_key]
                    if datetime.utcnow() - timestamp < timedelta(seconds=self.cache_ttl):
                        logger.debug(f"Cache hit for query: {cache_key}")
                        return results

            # Execute query
            results = await super().query_agents(query)

            # Update cache
            async with self._cache_lock:
                self.query_cache[cache_key] = (results, datetime.utcnow())

                # Clean old cache entries
                self._clean_cache()

            return results

        async def register_agent(self, registration: AgentRegistration) -> str:
            """Register agent and invalidate cache"""
            result = await super().register_agent(registration)
            await self._invalidate_cache()
            return result

        async def deregister_agent(self, agent_id: str) -> bool:
            """Deregister agent and invalidate cache"""
            result = await super().deregister_agent(agent_id)
            if result:
                await self._invalidate_cache()
            return result

        async def update_capabilities(
                self,
                agent_id: str,
                capabilities: List[Capability]
        ) -> bool:
            """Update capabilities and invalidate cache"""
            result = await super().update_capabilities(agent_id, capabilities)
            if result:
                await self._invalidate_cache()
            return result

        async def _invalidate_cache(self) -> None:
            """Clear query cache"""
            async with self._cache_lock:
                self.query_cache.clear()
                logger.debug("Query cache invalidated")

        def _generate_cache_key(self, query: ServiceQuery) -> str:
            """Generate cache key from query"""
            parts = []

            if query.capabilities:
                parts.append(f"caps:{','.join(sorted(query.capabilities))}")
            if query.status:
                parts.append(f"status:{query.status}")
            if query.health_status:
                parts.append(f"health:{query.health_status}")
            if query.tags:
                parts.append(f"tags:{','.join(sorted(query.tags))}")
            if query.metadata_filter:
                meta_str = ",".join(f"{k}:{v}" for k, v in sorted(query.metadata_filter.items()))
                parts.append(f"meta:{meta_str}")

            parts.append(f"all:{query.require_all}")
            parts.append(f"sort:{query.sort_by or 'none'}")
            parts.append(f"limit:{query.limit or 'none'}")
            parts.append(f"offset:{query.offset or 0}")

            return "|".join(parts)

        def _clean_cache(self) -> None:
            """Remove expired cache entries"""
            cutoff = datetime.utcnow() - timedelta(seconds=self.cache_ttl)
            expired_keys = [
                key for key, (_, timestamp) in self.query_cache.items()
                if timestamp < cutoff
            ]
            for key in expired_keys:
                del self.query_cache[key]


# Export storage implementations
__all__ = [
    "InMemoryStorage",
    "CachedInMemoryStorage"
]
