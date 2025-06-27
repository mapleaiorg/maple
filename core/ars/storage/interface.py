# File: core/ars/storage/interface.py
# Description: Abstract storage interface for the Agent Registry Service
# supporting multiple backend implementations.

from __future__ import annotations
from abc import ABC, abstractmethod
from typing import List, Optional, Dict, Any, Set, AsyncIterator
from datetime import datetime, timedelta
import asyncio

from core.ars.models.registry import (
    AgentRegistration, ServiceQuery, Capability,
    AgentStatus, HealthStatus, RegistryEvent
)


class RegistryStorage(ABC):
    """Abstract interface for registry storage backends"""

    @abstractmethod
    async def connect(self) -> None:
        """Connect to storage backend"""
        pass

    @abstractmethod
    async def disconnect(self) -> None:
        """Disconnect from storage backend"""
        pass

    @abstractmethod
    async def register_agent(self, registration: AgentRegistration) -> None:
        """Register or update an agent"""
        pass

    @abstractmethod
    async def deregister_agent(self, agent_id: str) -> bool:
        """Remove an agent from registry"""
        pass

    @abstractmethod
    async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
        """Get agent by ID"""
        pass

    @abstractmethod
    async def update_agent_status(self,
                                  agent_id: str,
                                  status: AgentStatus,
                                  health: Optional[HealthStatus] = None) -> bool:
        """Update agent status and optionally health"""
        pass

    @abstractmethod
    async def update_heartbeat(self, agent_id: str) -> bool:
        """Update agent heartbeat timestamp"""
        pass

    @abstractmethod
    async def query_agents(self, query: ServiceQuery,
                           limit: Optional[int] = None) -> List[AgentRegistration]:
        """Query agents matching criteria"""
        pass

    @abstractmethod
    async def get_all_agents(self,
                             include_offline: bool = False,
                             limit: Optional[int] = None) -> List[AgentRegistration]:
        """Get all agents in registry"""
        pass

    @abstractmethod
    async def get_agents_by_capability(self,
                                       capability_name: str,
                                       min_version: Optional[str] = None) -> List[AgentRegistration]:
        """Get all agents with specific capability"""
        pass

    @abstractmethod
    async def update_capability_metrics(self,
                                        agent_id: str,
                                        capability_name: str,
                                        response_time_ms: float,
                                        success: bool) -> bool:
        """Update capability performance metrics"""
        pass

    @abstractmethod
    async def get_stale_agents(self,
                               max_age: timedelta) -> List[AgentRegistration]:
        """Get agents that haven't sent heartbeat within max_age"""
        pass

    @abstractmethod
    async def record_event(self, event: RegistryEvent) -> None:
        """Record a registry event"""
        pass

    @abstractmethod
    async def get_events(self,
                         agent_id: Optional[str] = None,
                         event_type: Optional[str] = None,
                         since: Optional[datetime] = None,
                         limit: int = 100) -> List[RegistryEvent]:
        """Retrieve registry events"""
        pass

    @abstractmethod
    async def cleanup_stale_data(self, max_age: timedelta) -> int:
        """Clean up old data and return number of records removed"""
        pass


class TransactionalStorage(RegistryStorage):
    """Storage interface with transaction support"""

    @abstractmethod
    async def begin_transaction(self) -> Any:
        """Begin a transaction"""
        pass

    @abstractmethod
    async def commit_transaction(self, transaction: Any) -> None:
        """Commit a transaction"""
        pass

    @abstractmethod
    async def rollback_transaction(self, transaction: Any) -> None:
        """Rollback a transaction"""
        pass


class DistributedStorage(RegistryStorage):
    """Storage interface for distributed implementations"""

    @abstractmethod
    async def acquire_lock(self, key: str, timeout: float = 5.0) -> bool:
        """Acquire distributed lock"""
        pass

    @abstractmethod
    async def release_lock(self, key: str) -> None:
        """Release distributed lock"""
        pass

    @abstractmethod
    async def watch_changes(self,
                            callback: callable,
                            agent_id: Optional[str] = None) -> AsyncIterator[RegistryEvent]:
        """Watch for registry changes"""
        pass

    @abstractmethod
    async def replicate_to_peer(self, peer_id: str, data: Any) -> bool:
        """Replicate data to peer node"""
        pass


class CachedStorage(RegistryStorage):
    """Storage wrapper with caching support"""

    def __init__(self, backend: RegistryStorage, cache_ttl: int = 300):
        self.backend = backend
        self.cache_ttl = cache_ttl
        self.cache: Dict[str, Tuple[Any, datetime]] = {}
        self._lock = asyncio.Lock()

    async def connect(self) -> None:
        await self.backend.connect()

    async def disconnect(self) -> None:
        await self.backend.disconnect()

    async def register_agent(self, registration: AgentRegistration) -> None:
        await self.backend.register_agent(registration)
        # Invalidate cache
        async with self._lock:
            self._invalidate_agent_cache(registration.agent_id)

    async def deregister_agent(self, agent_id: str) -> bool:
        result = await self.backend.deregister_agent(agent_id)
        # Invalidate cache
        async with self._lock:
            self._invalidate_agent_cache(agent_id)
        return result

    async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
        # Check cache first
        async with self._lock:
            cached = self._get_from_cache(f"agent:{agent_id}")
            if cached:
                return cached

        # Get from backend
        agent = await self.backend.get_agent(agent_id)

        # Cache result
        if agent:
            async with self._lock:
                self._add_to_cache(f"agent:{agent_id}", agent)

        return agent

    async def update_agent_status(self,
                                  agent_id: str,
                                  status: AgentStatus,
                                  health: Optional[HealthStatus] = None) -> bool:
        result = await self.backend.update_agent_status(agent_id, status, health)

        # Invalidate cache
        async with self._lock:
            self._invalidate_agent_cache(agent_id)

        return result

    async def update_heartbeat(self, agent_id: str) -> bool:
        # Don't cache heartbeat updates
        return await self.backend.update_heartbeat(agent_id)

    async def query_agents(self, query: ServiceQuery,
                           limit: Optional[int] = None) -> List[AgentRegistration]:
        # Complex queries bypass cache for now
        return await self.backend.query_agents(query, limit)

    async def get_all_agents(self,
                             include_offline: bool = False,
                             limit: Optional[int] = None) -> List[AgentRegistration]:
        cache_key = f"all_agents:{include_offline}:{limit}"

        # Check cache
        async with self._lock:
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached

        # Get from backend
        agents = await self.backend.get_all_agents(include_offline, limit)

        # Cache result
        async with self._lock:
            self._add_to_cache(cache_key, agents, ttl=60)  # Shorter TTL for lists

        return agents

    async def get_agents_by_capability(self,
                                       capability_name: str,
                                       min_version: Optional[str] = None) -> List[AgentRegistration]:
        cache_key = f"capability:{capability_name}:{min_version}"

        # Check cache
        async with self._lock:
            cached = self._get_from_cache(cache_key)
            if cached:
                return cached

        # Get from backend
        agents = await self.backend.get_agents_by_capability(capability_name, min_version)

        # Cache result
        async with self._lock:
            self._add_to_cache(cache_key, agents, ttl=120)

        return agents

    async def update_capability_metrics(self,
                                        agent_id: str,
                                        capability_name: str,
                                        response_time_ms: float,
                                        success: bool) -> bool:
        result = await self.backend.update_capability_metrics(
            agent_id, capability_name, response_time_ms, success
        )

        # Invalidate related caches
        async with self._lock:
            self._invalidate_agent_cache(agent_id)
            self._invalidate_pattern(f"capability:{capability_name}:*")

        return result

    async def get_stale_agents(self, max_age: timedelta) -> List[AgentRegistration]:
        # Don't cache stale agent queries
        return await self.backend.get_stale_agents(max_age)

    async def record_event(self, event: RegistryEvent) -> None:
        await self.backend.record_event(event)

    async def get_events(self,
                         agent_id: Optional[str] = None,
                         event_type: Optional[str] = None,
                         since: Optional[datetime] = None,
                         limit: int = 100) -> List[RegistryEvent]:
        # Don't cache event queries
        return await self.backend.get_events(agent_id, event_type, since, limit)

    async def cleanup_stale_data(self, max_age: timedelta) -> int:
        result = await self.backend.cleanup_stale_data(max_age)

        # Clear cache after cleanup
        async with self._lock:
            self.cache.clear()

        return result

    def _get_from_cache(self, key: str) -> Optional[Any]:
        """Get item from cache if not expired"""
        if key in self.cache:
            value, timestamp = self.cache[key]
            if (datetime.utcnow() - timestamp).total_seconds() < self.cache_ttl:
                return value
            else:
                del self.cache[key]
        return None

    def _add_to_cache(self, key: str, value: Any, ttl: Optional[int] = None) -> None:
        """Add item to cache"""
        self.cache[key] = (value, datetime.utcnow())

    def _invalidate_agent_cache(self, agent_id: str) -> None:
        """Invalidate all cache entries for an agent"""
        self._invalidate_pattern(f"agent:{agent_id}")
        self._invalidate_pattern("all_agents:*")

    def _invalidate_pattern(self, pattern: str) -> None:
        """Invalidate cache entries matching pattern"""
        if pattern.endswith("*"):
            prefix = pattern[:-1]
            keys_to_remove = [k for k in self.cache.keys() if k.startswith(prefix)]
        else:
            keys_to_remove = [pattern] if pattern in self.cache else []

        for key in keys_to_remove:
            del self.cache[key]