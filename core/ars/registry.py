# File: maple/core/ars/registry.py
# Description: Core registry manager that orchestrates storage backends and provides
# the main API for agent registration, discovery, and lifecycle management.

from __future__ import annotations
import asyncio
import uuid
from datetime import datetime, timedelta
from typing import List, Optional, Dict, Any, Type, Union
import logging
from enum import Enum
from contextlib import asynccontextmanager

from maple.core.ars.models.registry import (
    AgentRegistration, ServiceQuery, Capability,
    AgentStatus, HealthStatus, RegistryEvent, Endpoint
)
from maple.core.ars.storage.interface import RegistryStorage
from maple.core.ars.storage.memory import InMemoryStorage
from maple.core.ars.storage.redis import RedisStorage
from maple.core.ars.storage.postgres import PostgresStorage
from maple.core.ars.discovery import DiscoveryEngine
from maple.core.ars.health import HealthMonitor
from maple.core.ars.events import EventBus, EventHandler

logger = logging.getLogger(__name__)


class StorageBackend(str, Enum):
    """Available storage backends"""
    MEMORY = "memory"
    REDIS = "redis"
    POSTGRES = "postgres"
    HYBRID = "hybrid"


class RegistryConfig:
    """Configuration for the registry"""

    def __init__(
            self,
            storage_backend: StorageBackend = StorageBackend.MEMORY,
            storage_config: Optional[Dict[str, Any]] = None,
            health_check_interval: int = 30,
            cleanup_interval: int = 300,
            agent_ttl: int = 3600,
            enable_clustering: bool = False,
            cluster_config: Optional[Dict[str, Any]] = None,
            enable_caching: bool = True,
            cache_ttl: int = 60,
            max_agents: int = 1000000,
            enable_events: bool = True,
            event_retention: int = 86400  # 24 hours
    ):
        self.storage_backend = storage_backend
        self.storage_config = storage_config or {}
        self.health_check_interval = health_check_interval
        self.cleanup_interval = cleanup_interval
        self.agent_ttl = agent_ttl
        self.enable_clustering = enable_clustering
        self.cluster_config = cluster_config or {}
        self.enable_caching = enable_caching
        self.cache_ttl = cache_ttl
        self.max_agents = max_agents
        self.enable_events = enable_events
        self.event_retention = event_retention


class RegistryManager:
    """
    Core registry manager for the Agent Registry Service.
    Manages agent lifecycle, discovery, and coordination.
    """

    def __init__(self, config: Optional[RegistryConfig] = None):
        self.config = config or RegistryConfig()
        self._storage: Optional[RegistryStorage] = None
        self._discovery: Optional[DiscoveryEngine] = None
        self._health_monitor: Optional[HealthMonitor] = None
        self._event_bus: Optional[EventBus] = None
        self._running = False
        self._tasks: List[asyncio.Task] = []
        self._lock = asyncio.Lock()
        self._agent_locks: Dict[str, asyncio.Lock] = {}
        self._stats = RegistryStats()

    async def start(self) -> None:
        """Start the registry manager"""
        if self._running:
            return

        logger.info("Starting Registry Manager...")

        # Initialize storage
        self._storage = await self._create_storage()
        await self._storage.connect()

        # Initialize components
        self._discovery = DiscoveryEngine(self._storage, self.config)
        self._health_monitor = HealthMonitor(self._storage, self.config)

        if self.config.enable_events:
            self._event_bus = EventBus()
            await self._event_bus.start()

        # Start background tasks
        self._tasks.append(
            asyncio.create_task(self._health_check_loop())
        )
        self._tasks.append(
            asyncio.create_task(self._cleanup_loop())
        )

        self._running = True
        logger.info("Registry Manager started successfully")

    async def stop(self) -> None:
        """Stop the registry manager"""
        if not self._running:
            return

        logger.info("Stopping Registry Manager...")
        self._running = False

        # Cancel background tasks
        for task in self._tasks:
            task.cancel()

        # Wait for tasks to complete
        await asyncio.gather(*self._tasks, return_exceptions=True)
        self._tasks.clear()

        # Stop components
        if self._event_bus:
            await self._event_bus.stop()

        # Disconnect storage
        if self._storage:
            await self._storage.disconnect()

        logger.info("Registry Manager stopped")

    async def register_agent(
            self,
            name: str,
            version: str,
            capabilities: List[Capability],
            endpoints: List[Endpoint],
            metadata: Optional[Dict[str, Any]] = None,
            agent_id: Optional[str] = None
    ) -> AgentRegistration:
        """Register a new agent"""
        # Validate registration
        if not name or not version:
            raise ValueError("Agent name and version are required")

        if not capabilities:
            raise ValueError("At least one capability is required")

        if not endpoints:
            raise ValueError("At least one endpoint is required")

        # Check agent limit
        stats = await self._storage.get_statistics()
        if stats["total_agents"] >= self.config.max_agents:
            raise ValueError(f"Maximum agent limit ({self.config.max_agents}) reached")

        # Create registration
        registration = AgentRegistration(
            agent_id=agent_id or self._generate_agent_id(name),
            name=name,
            version=version,
            status=AgentStatus.ACTIVE,
            health_status=HealthStatus.HEALTHY,
            capabilities=capabilities,
            endpoints=endpoints,
            metadata=metadata or {},
            metrics={},
            created_at=datetime.utcnow(),
            last_heartbeat=datetime.utcnow()
        )

        # Acquire agent lock
        async with self._get_agent_lock(registration.agent_id):
            # Store registration
            await self._storage.register_agent(registration)

            # Update statistics
            self._stats.agents_registered += 1

            # Emit event
            if self._event_bus:
                await self._event_bus.emit(
                    "agent.registered",
                    {
                        "agent_id": registration.agent_id,
                        "name": name,
                        "capabilities": [cap.name for cap in capabilities]
                    }
                )

            logger.info(f"Agent {registration.agent_id} registered successfully")
            return registration

    async def deregister_agent(self, agent_id: str) -> bool:
        """Deregister an agent"""
        async with self._get_agent_lock(agent_id):
            # Check if agent exists
            agent = await self._storage.get_agent(agent_id)
            if not agent:
                return False

            # Deregister from storage
            success = await self._storage.deregister_agent(agent_id)

            if success:
                # Update statistics
                self._stats.agents_deregistered += 1

                # Emit event
                if self._event_bus:
                    await self._event_bus.emit(
                        "agent.deregistered",
                        {"agent_id": agent_id}
                    )

                # Remove agent lock
                self._agent_locks.pop(agent_id, None)

                logger.info(f"Agent {agent_id} deregistered successfully")

            return success

    async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
        """Get agent by ID"""
        return await self._storage.get_agent(agent_id)

    async def discover_agents(
            self,
            capabilities: Optional[List[str]] = None,
            tags: Optional[List[str]] = None,
            status: Optional[AgentStatus] = None,
            health_status: Optional[HealthStatus] = None,
            metadata_filter: Optional[Dict[str, Any]] = None,
            require_all_capabilities: bool = True,
            sort_by: Optional[str] = None,
            limit: Optional[int] = None,
            offset: Optional[int] = None
    ) -> List[AgentRegistration]:
        """Discover agents matching criteria"""
        query = ServiceQuery(
            capabilities=capabilities,
            tags=tags,
            status=status,
            health_status=health_status,
            metadata_filter=metadata_filter,
            require_all=require_all_capabilities,
            sort_by=sort_by,
            limit=limit,
            offset=offset
        )

        # Use discovery engine for advanced matching
        if self._discovery:
            return await self._discovery.search(query)
        else:
            return await self._storage.query_agents(query)

    async def update_agent_health(
            self,
            agent_id: str,
            health: HealthStatus,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Update agent health status"""
        async with self._get_agent_lock(agent_id):
            success = await self._storage.update_health(agent_id, health, metrics)

            if success:
                # Update statistics
                self._stats.health_updates += 1

                # Emit event
                if self._event_bus:
                    await self._event_bus.emit(
                        "agent.health_updated",
                        {
                            "agent_id": agent_id,
                            "health": health,
                            "metrics": metrics
                        }
                    )

                # Check if agent needs attention
                if health in [HealthStatus.DEGRADED, HealthStatus.UNHEALTHY]:
                    logger.warning(f"Agent {agent_id} health degraded: {health}")

            return success

    async def heartbeat(
            self,
            agent_id: str,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Process agent heartbeat"""
        return await self.update_agent_health(
            agent_id,
            HealthStatus.HEALTHY,
            metrics
        )

    async def update_agent_capabilities(
            self,
            agent_id: str,
            capabilities: List[Capability]
    ) -> bool:
        """Update agent capabilities"""
        async with self._get_agent_lock(agent_id):
            success = await self._storage.update_capabilities(agent_id, capabilities)

            if success:
                # Emit event
                if self._event_bus:
                    await self._event_bus.emit(
                        "agent.capabilities_updated",
                        {
                            "agent_id": agent_id,
                            "capabilities": [cap.name for cap in capabilities]
                        }
                    )

            return success

    async def get_agent_events(
            self,
            agent_id: Optional[str] = None,
            event_type: Optional[str] = None,
            since: Optional[datetime] = None,
            limit: int = 100
    ) -> List[RegistryEvent]:
        """Get registry events"""
        return await self._storage.get_events(
            agent_id=agent_id,
            event_type=event_type,
            since=since,
            limit=limit
        )

    async def get_statistics(self) -> Dict[str, Any]:
        """Get registry statistics"""
        storage_stats = await self._storage.get_statistics()

        return {
            **storage_stats,
            "registry_stats": {
                "agents_registered": self._stats.agents_registered,
                "agents_deregistered": self._stats.agents_deregistered,
                "health_updates": self._stats.health_updates,
                "uptime_seconds": (datetime.utcnow() - self._stats.start_time).total_seconds()
            }
        }

    def subscribe(self, event_type: str, handler: EventHandler) -> None:
        """Subscribe to registry events"""
        if self._event_bus:
            self._event_bus.subscribe(event_type, handler)

    def unsubscribe(self, event_type: str, handler: EventHandler) -> None:
        """Unsubscribe from registry events"""
        if self._event_bus:
            self._event_bus.unsubscribe(event_type, handler)

    # Private methods

    async def _create_storage(self) -> RegistryStorage:
        """Create storage backend based on configuration"""
        backend = self.config.storage_backend
        config = self.config.storage_config

        if backend == StorageBackend.MEMORY:
            from maple.core.ars.storage.memory import CachedInMemoryStorage
            if self.config.enable_caching:
                return CachedInMemoryStorage(cache_ttl=self.config.cache_ttl)
            else:
                return InMemoryStorage()

        elif backend == StorageBackend.REDIS:
            return RedisStorage(**config)

        elif backend == StorageBackend.POSTGRES:
            return PostgresStorage(**config)

        elif backend == StorageBackend.HYBRID:
            # Hybrid storage with caching layer
            return await self._create_hybrid_storage(config)

        else:
            raise ValueError(f"Unknown storage backend: {backend}")

    async def _create_hybrid_storage(self, config: Dict[str, Any]) -> RegistryStorage:
        """Create hybrid storage with multiple backends"""
        # Implementation for hybrid storage with caching and persistence
        # This would combine memory cache with persistent backend
        raise NotImplementedError("Hybrid storage not yet implemented")

    def _generate_agent_id(self, name: str) -> str:
        """Generate unique agent ID"""
        return f"{name}-{uuid.uuid4().hex[:8]}"

    def _get_agent_lock(self, agent_id: str) -> asyncio.Lock:
        """Get or create lock for agent"""
        if agent_id not in self._agent_locks:
            self._agent_locks[agent_id] = asyncio.Lock()
        return self._agent_locks[agent_id]

    async def _health_check_loop(self) -> None:
        """Background task for health monitoring"""
        while self._running:
            try:
                await asyncio.sleep(self.config.health_check_interval)

                if self._health_monitor:
                    await self._health_monitor.check_all_agents()

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Error in health check loop: {e}")

    async def _cleanup_loop(self) -> None:
        """Background task for cleanup"""
        while self._running:
            try:
                await asyncio.sleep(self.config.cleanup_interval)

                # Clean expired agents
                ttl = timedelta(seconds=self.config.agent_ttl)
                expired_count = await self._storage.clean_expired(ttl)

                if expired_count > 0:
                    logger.info(f"Cleaned {expired_count} expired agents")

                # Clean old events if enabled
                if self.config.enable_events and self.config.event_retention > 0:
                    # Event cleanup would be implemented here
                    pass

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Error in cleanup loop: {e}")

    # Context manager support

    async def __aenter__(self):
        await self.start()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.stop()


class RegistryStats:
    """Registry statistics tracker"""

    def __init__(self):
        self.start_time = datetime.utcnow()
        self.agents_registered = 0
        self.agents_deregistered = 0
        self.health_updates = 0


# Clustered Registry Manager for distributed deployments

class ClusteredRegistryManager(RegistryManager):
    """
    Registry manager with clustering support for distributed deployments.
    Provides coordination across multiple registry instances.
    """

    def __init__(self, config: Optional[RegistryConfig] = None):
        super().__init__(config)
        self._node_id = uuid.uuid4().hex[:8]
        self._cluster_coordinator = None
        self._peer_nodes: Dict[str, PeerNode] = {}

    async def start(self) -> None:
        """Start clustered registry manager"""
        await super().start()

        if self.config.enable_clustering:
            # Initialize cluster coordinator
            self._cluster_coordinator = ClusterCoordinator(
                node_id=self._node_id,
                config=self.config.cluster_config
            )
            await self._cluster_coordinator.start()

            # Start peer discovery
            self._tasks.append(
                asyncio.create_task(self._peer_discovery_loop())
            )

            logger.info(f"Clustered Registry Manager started (node: {self._node_id})")

    async def stop(self) -> None:
        """Stop clustered registry manager"""
        if self._cluster_coordinator:
            await self._cluster_coordinator.stop()

        await super().stop()

    async def _peer_discovery_loop(self) -> None:
        """Discover and sync with peer nodes"""
        while self._running:
            try:
                await asyncio.sleep(30)  # Discovery interval

                if self._cluster_coordinator:
                    # Discover peers
                    peers = await self._cluster_coordinator.discover_peers()

                    # Update peer list
                    for peer in peers:
                        if peer.node_id not in self._peer_nodes:
                            self._peer_nodes[peer.node_id] = peer
                            logger.info(f"Discovered peer node: {peer.node_id}")

                    # Sync with peers
                    await self._sync_with_peers()

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Error in peer discovery: {e}")

    async def _sync_with_peers(self) -> None:
        """Synchronize state with peer nodes"""
        # Implementation for state synchronization
        # This would handle distributed consensus and data replication
        pass


class ClusterCoordinator:
    """Coordinator for cluster operations"""

    def __init__(self, node_id: str, config: Dict[str, Any]):
        self.node_id = node_id
        self.config = config

    async def start(self) -> None:
        """Start cluster coordinator"""
        # Implementation for cluster coordination
        # Could use Raft, etcd, or similar for consensus
        pass

    async def stop(self) -> None:
        """Stop cluster coordinator"""
        pass

    async def discover_peers(self) -> List['PeerNode']:
        """Discover peer nodes in cluster"""
        # Implementation for peer discovery
        # Could use multicast, DNS, or service discovery
        return []


class PeerNode:
    """Represents a peer node in the cluster"""

    def __init__(self, node_id: str, address: str, metadata: Dict[str, Any]):
        self.node_id = node_id
        self.address = address
        self.metadata = metadata


# Factory function for creating registry managers

def create_registry_manager(
        backend: Union[str, StorageBackend] = StorageBackend.MEMORY,
        **kwargs
) -> RegistryManager:
    """
    Factory function to create a registry manager.

    Args:
        backend: Storage backend to use
        **kwargs: Additional configuration options

    Returns:
        Configured RegistryManager instance
    """
    if isinstance(backend, str):
        backend = StorageBackend(backend)

    config = RegistryConfig(
        storage_backend=backend,
        **kwargs
    )

    if config.enable_clustering:
        return ClusteredRegistryManager(config)
    else:
        return RegistryManager(config)


# Export public API
__all__ = [
    "RegistryManager",
    "ClusteredRegistryManager",
    "RegistryConfig",
    "StorageBackend",
    "create_registry_manager"
]