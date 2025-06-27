# File: core/ars/client.py
# Description: Client SDK for interacting with the Agent Registry Service.
# Provides both REST and gRPC clients with a unified interface.

from __future__ import annotations
import asyncio
import json
from datetime import datetime, timedelta
from typing import List, Optional, Dict, Any, Union, AsyncIterator, Callable
import logging
from dataclasses import dataclass, field
from contextlib import asynccontextmanager
from enum import Enum

import aiohttp
import grpc

from core.ars.models.registry import (
    AgentRegistration, ServiceQuery, Capability,
    AgentStatus, HealthStatus, RegistryEvent, Endpoint
)
from core.ars.discovery import SearchStrategy

logger = logging.getLogger(__name__)


class ClientProtocol(str, Enum):
    """Client protocol options"""
    REST = "rest"
    GRPC = "grpc"
    AUTO = "auto"


@dataclass
class ClientConfig:
    """Configuration for ARS client"""
    protocol: ClientProtocol = ClientProtocol.AUTO
    base_url: Optional[str] = None  # For REST
    grpc_host: Optional[str] = None  # For gRPC
    grpc_port: int = 50051
    timeout: int = 30
    retry_count: int = 3
    retry_delay: float = 1.0
    auth_token: Optional[str] = None
    enable_caching: bool = True
    cache_ttl: int = 300
    max_connections: int = 100
    enable_compression: bool = True

    def __post_init__(self):
        # Set defaults based on protocol
        if self.protocol == ClientProtocol.REST and not self.base_url:
            self.base_url = "http://localhost:8080"
        elif self.protocol == ClientProtocol.GRPC and not self.grpc_host:
            self.grpc_host = "localhost"


class ARSClient:
    """
    Unified client for Agent Registry Service.
    Supports both REST and gRPC protocols with automatic failover.
    """

    def __init__(self, config: Optional[ClientConfig] = None):
        self.config = config or ClientConfig()
        self._rest_client: Optional[ARSRestClient] = None
        self._grpc_client: Optional[ARSGrpcClient] = None
        self._active_client: Optional[Union[ARSRestClient, ARSGrpcClient]] = None
        self._cache: Dict[str, tuple[Any, datetime]] = {}
        self._cache_lock = asyncio.Lock()

    async def __aenter__(self):
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.close()

    async def connect(self) -> None:
        """Connect to ARS service"""
        if self.config.protocol in [ClientProtocol.REST, ClientProtocol.AUTO]:
            self._rest_client = ARSRestClient(self.config)
            await self._rest_client.connect()

        if self.config.protocol in [ClientProtocol.GRPC, ClientProtocol.AUTO]:
            self._grpc_client = ARSGrpcClient(self.config)
            await self._grpc_client.connect()

        # Determine active client
        if self.config.protocol == ClientProtocol.AUTO:
            # Try gRPC first for better performance
            if self._grpc_client and await self._grpc_client.health_check():
                self._active_client = self._grpc_client
                logger.info("Using gRPC client")
            elif self._rest_client and await self._rest_client.health_check():
                self._active_client = self._rest_client
                logger.info("Using REST client")
            else:
                raise ConnectionError("Failed to connect to ARS service")
        else:
            self._active_client = self._grpc_client if self.config.protocol == ClientProtocol.GRPC else self._rest_client

    async def close(self) -> None:
        """Close client connections"""
        if self._rest_client:
            await self._rest_client.close()
        if self._grpc_client:
            await self._grpc_client.close()

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
        return await self._active_client.register_agent(
            name=name,
            version=version,
            capabilities=capabilities,
            endpoints=endpoints,
            metadata=metadata,
            agent_id=agent_id
        )

    async def deregister_agent(self, agent_id: str) -> bool:
        """Deregister an agent"""
        # Invalidate cache
        await self._invalidate_cache(f"agent:{agent_id}")
        return await self._active_client.deregister_agent(agent_id)

    async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
        """Get agent by ID"""
        # Check cache
        cache_key = f"agent:{agent_id}"
        cached = await self._get_cached(cache_key)
        if cached is not None:
            return cached

        # Fetch from service
        agent = await self._active_client.get_agent(agent_id)

        # Cache result
        if agent and self.config.enable_caching:
            await self._set_cached(cache_key, agent)

        return agent

    async def discover_agents(
            self,
            capabilities: Optional[List[str]] = None,
            tags: Optional[List[str]] = None,
            status: Optional[AgentStatus] = None,
            health_status: Optional[HealthStatus] = None,
            metadata_filter: Optional[Dict[str, Any]] = None,
            require_all_capabilities: bool = True,
            search_strategy: SearchStrategy = SearchStrategy.HYBRID,
            sort_by: Optional[str] = None,
            limit: Optional[int] = None,
            offset: Optional[int] = None
    ) -> List[AgentRegistration]:
        """Discover agents matching criteria"""
        return await self._active_client.discover_agents(
            capabilities=capabilities,
            tags=tags,
            status=status,
            health_status=health_status,
            metadata_filter=metadata_filter,
            require_all_capabilities=require_all_capabilities,
            search_strategy=search_strategy,
            sort_by=sort_by,
            limit=limit,
            offset=offset
        )

    async def update_health(
            self,
            agent_id: str,
            health_status: HealthStatus,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Update agent health status"""
        # Invalidate cache
        await self._invalidate_cache(f"agent:{agent_id}")
        return await self._active_client.update_health(agent_id, health_status, metrics)

    async def heartbeat(
            self,
            agent_id: str,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Send agent heartbeat"""
        return await self._active_client.heartbeat(agent_id, metrics)

    async def update_capabilities(
            self,
            agent_id: str,
            capabilities: List[Capability]
    ) -> bool:
        """Update agent capabilities"""
        # Invalidate cache
        await self._invalidate_cache(f"agent:{agent_id}")
        return await self._active_client.update_capabilities(agent_id, capabilities)

    async def get_events(
            self,
            agent_id: Optional[str] = None,
            event_type: Optional[str] = None,
            since: Optional[datetime] = None,
            limit: int = 100
    ) -> List[RegistryEvent]:
        """Get registry events"""
        return await self._active_client.get_events(
            agent_id=agent_id,
            event_type=event_type,
            since=since,
            limit=limit
        )

    async def stream_events(
            self,
            event_types: Optional[List[str]] = None,
            agent_id: Optional[str] = None
    ) -> AsyncIterator[RegistryEvent]:
        """Stream registry events"""
        async for event in self._active_client.stream_events(event_types, agent_id):
            yield event

    async def get_statistics(self) -> Dict[str, Any]:
        """Get registry statistics"""
        return await self._active_client.get_statistics()

    # Caching methods

    async def _get_cached(self, key: str) -> Optional[Any]:
        """Get cached value"""
        if not self.config.enable_caching:
            return None

        async with self._cache_lock:
            if key in self._cache:
                value, timestamp = self._cache[key]
                if datetime.utcnow() - timestamp < timedelta(seconds=self.config.cache_ttl):
                    return value
                else:
                    del self._cache[key]

        return None

    async def _set_cached(self, key: str, value: Any) -> None:
        """Set cached value"""
        if not self.config.enable_caching:
            return

        async with self._cache_lock:
            self._cache[key] = (value, datetime.utcnow())

            # Limit cache size
            if len(self._cache) > 1000:
                # Remove oldest entries
                sorted_items = sorted(
                    self._cache.items(),
                    key=lambda x: x[1][1]
                )
                self._cache = dict(sorted_items[-800:])

    async def _invalidate_cache(self, pattern: str) -> None:
        """Invalidate cache entries matching pattern"""
        if not self.config.enable_caching:
            return

        async with self._cache_lock:
            keys_to_remove = [
                key for key in self._cache.keys()
                if key.startswith(pattern)
            ]
            for key in keys_to_remove:
                del self._cache[key]


class ARSRestClient:
    """REST client for Agent Registry Service"""

    def __init__(self, config: ClientConfig):
        self.config = config
        self.base_url = config.base_url.rstrip('/')
        self._session: Optional[aiohttp.ClientSession] = None

    async def connect(self) -> None:
        """Initialize HTTP session"""
        connector = aiohttp.TCPConnector(
            limit=self.config.max_connections,
            force_close=True
        )

        timeout = aiohttp.ClientTimeout(total=self.config.timeout)

        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json"
        }

        if self.config.auth_token:
            headers["Authorization"] = f"Bearer {self.config.auth_token}"

        self._session = aiohttp.ClientSession(
            connector=connector,
            timeout=timeout,
            headers=headers,
            compress=self.config.enable_compression
        )

    async def close(self) -> None:
        """Close HTTP session"""
        if self._session:
            await self._session.close()

    async def health_check(self) -> bool:
        """Check if service is healthy"""
        try:
            async with self._session.get(f"{self.base_url}/health") as resp:
                return resp.status == 200
        except:
            return False

    async def register_agent(
            self,
            name: str,
            version: str,
            capabilities: List[Capability],
            endpoints: List[Endpoint],
            metadata: Optional[Dict[str, Any]] = None,
            agent_id: Optional[str] = None
    ) -> AgentRegistration:
        """Register a new agent via REST"""
        data = {
            "name": name,
            "version": version,
            "capabilities": [cap.model_dump() for cap in capabilities],
            "endpoints": [ep.model_dump() for ep in endpoints],
            "metadata": metadata or {}
        }

        if agent_id:
            data["agent_id"] = agent_id

        async with self._session.post(
                f"{self.base_url}/v1/agents",
                json=data
        ) as resp:
            if resp.status == 201:
                result = await resp.json()
                return self._parse_agent(result)
            else:
                error = await resp.text()
                raise Exception(f"Failed to register agent: {error}")

    async def deregister_agent(self, agent_id: str) -> bool:
        """Deregister an agent via REST"""
        async with self._session.delete(
                f"{self.base_url}/v1/agents/{agent_id}"
        ) as resp:
            return resp.status == 204

    async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
        """Get agent by ID via REST"""
        async with self._session.get(
                f"{self.base_url}/v1/agents/{agent_id}"
        ) as resp:
            if resp.status == 200:
                result = await resp.json()
                return self._parse_agent(result)
            elif resp.status == 404:
                return None
            else:
                error = await resp.text()
                raise Exception(f"Failed to get agent: {error}")

    async def discover_agents(
            self,
            capabilities: Optional[List[str]] = None,
            tags: Optional[List[str]] = None,
            status: Optional[AgentStatus] = None,
            health_status: Optional[HealthStatus] = None,
            metadata_filter: Optional[Dict[str, Any]] = None,
            require_all_capabilities: bool = True,
            search_strategy: SearchStrategy = SearchStrategy.HYBRID,
            sort_by: Optional[str] = None,
            limit: Optional[int] = None,
            offset: Optional[int] = None
    ) -> List[AgentRegistration]:
        """Discover agents via REST"""
        data = {
            "require_all_capabilities": require_all_capabilities,
            "search_strategy": search_strategy.value
        }

        if capabilities:
            data["capabilities"] = capabilities
        if tags:
            data["tags"] = tags
        if status:
            data["status"] = status.value
        if health_status:
            data["health_status"] = health_status.value
        if metadata_filter:
            data["metadata_filter"] = metadata_filter
        if sort_by:
            data["sort_by"] = sort_by
        if limit is not None:
            data["limit"] = limit
        if offset is not None:
            data["offset"] = offset

        async with self._session.post(
                f"{self.base_url}/v1/agents/discover",
                json=data
        ) as resp:
            if resp.status == 200:
                results = await resp.json()
                return [self._parse_agent(agent) for agent in results]
            else:
                error = await resp.text()
                raise Exception(f"Failed to discover agents: {error}")

    async def update_health(
            self,
            agent_id: str,
            health_status: HealthStatus,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Update agent health via REST"""
        data = {
            "health_status": health_status.value,
            "metrics": metrics
        }

        async with self._session.put(
                f"{self.base_url}/v1/agents/{agent_id}/health",
                json=data
        ) as resp:
            return resp.status == 204

    async def heartbeat(
            self,
            agent_id: str,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Send heartbeat via REST"""
        data = {"metrics": metrics} if metrics else {}

        async with self._session.post(
                f"{self.base_url}/v1/agents/{agent_id}/heartbeat",
                json=data
        ) as resp:
            return resp.status == 204

    async def update_capabilities(
            self,
            agent_id: str,
            capabilities: List[Capability]
    ) -> bool:
        """Update capabilities via REST"""
        data = {
            "capabilities": [cap.model_dump() for cap in capabilities]
        }

        async with self._session.put(
                f"{self.base_url}/v1/agents/{agent_id}/capabilities",
                json=data
        ) as resp:
            return resp.status == 204

    async def get_events(
            self,
            agent_id: Optional[str] = None,
            event_type: Optional[str] = None,
            since: Optional[datetime] = None,
            limit: int = 100
    ) -> List[RegistryEvent]:
        """Get events via REST"""
        params = {"limit": limit}

        if agent_id:
            params["agent_id"] = agent_id
        if event_type:
            params["event_type"] = event_type
        if since:
            params["since"] = since.isoformat()

        async with self._session.get(
                f"{self.base_url}/v1/events",
                params=params
        ) as resp:
            if resp.status == 200:
                results = await resp.json()
                return [self._parse_event(event) for event in results]
            else:
                error = await resp.text()
                raise Exception(f"Failed to get events: {error}")

    async def stream_events(
            self,
            event_types: Optional[List[str]] = None,
            agent_id: Optional[str] = None
    ) -> AsyncIterator[RegistryEvent]:
        """Stream events via WebSocket"""
        import aiohttp

        ws_url = self.base_url.replace("http://", "ws://").replace("https://", "wss://")
        ws_url = f"{ws_url}/v1/events/stream"

        async with self._session.ws_connect(ws_url) as ws:
            # Send subscription
            await ws.send_json({
                "event_types": event_types or [],
                "agent_id": agent_id
            })

            async for msg in ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    event_data = json.loads(msg.data)
                    yield self._parse_event(event_data)
                elif msg.type == aiohttp.WSMsgType.ERROR:
                    logger.error(f"WebSocket error: {ws.exception()}")
                    break

    async def get_statistics(self) -> Dict[str, Any]:
        """Get statistics via REST"""
        async with self._session.get(
                f"{self.base_url}/v1/statistics"
        ) as resp:
            if resp.status == 200:
                return await resp.json()
            else:
                error = await resp.text()
                raise Exception(f"Failed to get statistics: {error}")

    def _parse_agent(self, data: Dict[str, Any]) -> AgentRegistration:
        """Parse agent data from REST response"""
        return AgentRegistration(
            agent_id=data["agent_id"],
            name=data["name"],
            version=data["version"],
            status=AgentStatus(data["status"]),
            health_status=HealthStatus(data["health_status"]),
            capabilities=[
                Capability(**cap) for cap in data["capabilities"]
            ],
            endpoints=[
                Endpoint(**ep) for ep in data["endpoints"]
            ],
            metadata=data["metadata"],
            metrics=data.get("metrics", {}),
            created_at=datetime.fromisoformat(data["created_at"]),
            last_heartbeat=datetime.fromisoformat(data["last_heartbeat"])
        )

    def _parse_event(self, data: Dict[str, Any]) -> RegistryEvent:
        """Parse event data from REST response"""
        return RegistryEvent(
            event_id=data["event_id"],
            event_type=data["event_type"],
            timestamp=datetime.fromisoformat(data["timestamp"]),
            agent_id=data.get("agent_id"),
            data=data["data"]
        )


class ARSGrpcClient:
    """gRPC client for Agent Registry Service"""

    def __init__(self, config: ClientConfig):
        self.config = config
        self._channel: Optional[grpc.aio.Channel] = None
        self._stub = None

    async def connect(self) -> None:
        """Initialize gRPC channel"""
        try:
            from core.ars.grpc import ars_pb2_grpc

            # Create channel
            target = f"{self.config.grpc_host}:{self.config.grpc_port}"

            options = [
                ('grpc.max_send_message_length', 50 * 1024 * 1024),
                ('grpc.max_receive_message_length', 50 * 1024 * 1024),
                ('grpc.keepalive_time_ms', 10000),
                ('grpc.keepalive_timeout_ms', 5000),
            ]

            if self.config.enable_compression:
                options.append(('grpc.default_compression_algorithm', 1))  # GZIP

            if self.config.auth_token:
                # Create secure channel with auth
                credentials = grpc.ssl_channel_credentials()
                call_credentials = grpc.access_token_call_credentials(
                    self.config.auth_token
                )
                composite_credentials = grpc.composite_channel_credentials(
                    credentials,
                    call_credentials
                )
                self._channel = grpc.aio.secure_channel(
                    target,
                    composite_credentials,
                    options=options
                )
            else:
                # Create insecure channel
                self._channel = grpc.aio.insecure_channel(target, options=options)

            self._stub = ars_pb2_grpc.AgentRegistryServiceStub(self._channel)

        except ImportError:
            raise ImportError(
                "gRPC support not available. "
                "Run 'python -m core.ars.grpc.generate_grpc' to generate required files."
            )

    async def close(self) -> None:
        """Close gRPC channel"""
        if self._channel:
            await self._channel.close()

    async def health_check(self) -> bool:
        """Check if service is healthy via gRPC"""
        try:
            from google.protobuf import empty_pb2

            # Try to get statistics as health check
            await self._stub.GetStatistics(
                empty_pb2.Empty(),
                timeout=5
            )
            return True
        except:
            return False

    async def register_agent(
            self,
            name: str,
            version: str,
            capabilities: List[Capability],
            endpoints: List[Endpoint],
            metadata: Optional[Dict[str, Any]] = None,
            agent_id: Optional[str] = None
    ) -> AgentRegistration:
        """Register agent via gRPC"""
        from core.ars.grpc import ars_pb2

        request = ars_pb2.RegisterAgentRequest(
            agent_id=agent_id or "",
            name=name,
            version=version,
            capabilities=[
                ars_pb2.Capability(
                    name=cap.name,
                    version=cap.version,
                    description=cap.description,
                    parameters=self._dict_to_struct(cap.parameters)
                )
                for cap in capabilities
            ],
            endpoints=[
                ars_pb2.Endpoint(
                    type=ep.type,
                    url=ep.url,
                    protocol=ep.protocol,
                    metadata=self._dict_to_struct(ep.metadata)
                )
                for ep in endpoints
            ],
            metadata=self._dict_to_struct(metadata or {})
        )

        response = await self._stub.RegisterAgent(request)
        return self._proto_to_registration(response.agent)

    async def deregister_agent(self, agent_id: str) -> bool:
        """Deregister agent via gRPC"""
        from core.ars.grpc import ars_pb2

        request = ars_pb2.DeregisterAgentRequest(agent_id=agent_id)

        try:
            await self._stub.DeregisterAgent(request)
            return True
        except grpc.RpcError as e:
            if e.code() == grpc.StatusCode.NOT_FOUND:
                return False
            raise

    async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
        """Get agent via gRPC"""
        from core.ars.grpc import ars_pb2

        request = ars_pb2.GetAgentRequest(agent_id=agent_id)

        try:
            response = await self._stub.GetAgent(request)
            return self._proto_to_registration(response.agent)
        except grpc.RpcError as e:
            if e.code() == grpc.StatusCode.NOT_FOUND:
                return None
            raise

    async def discover_agents(
            self,
            capabilities: Optional[List[str]] = None,
            tags: Optional[List[str]] = None,
            status: Optional[AgentStatus] = None,
            health_status: Optional[HealthStatus] = None,
            metadata_filter: Optional[Dict[str, Any]] = None,
            require_all_capabilities: bool = True,
            search_strategy: SearchStrategy = SearchStrategy.HYBRID,
            sort_by: Optional[str] = None,
            limit: Optional[int] = None,
            offset: Optional[int] = None
    ) -> List[AgentRegistration]:
        """Discover agents via gRPC"""
        from core.ars.grpc import ars_pb2

        request = ars_pb2.DiscoverAgentsRequest(
            capabilities=capabilities or [],
            tags=tags or [],
            require_all_capabilities=require_all_capabilities,
            search_strategy=self._search_strategy_to_proto(search_strategy),
            sort_by=sort_by or "",
            limit=limit or 0,
            offset=offset or 0
        )

        if status:
            request.status = self._status_to_proto(status)
        if health_status:
            request.health_status = self._health_to_proto(health_status)
        if metadata_filter:
            request.metadata_filter.CopyFrom(self._dict_to_struct(metadata_filter))

        response = await self._stub.DiscoverAgents(request)
        return [self._proto_to_registration(agent) for agent in response.agents]

    async def update_health(
            self,
            agent_id: str,
            health_status: HealthStatus,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Update health via gRPC"""
        from core.ars.grpc import ars_pb2

        request = ars_pb2.UpdateHealthRequest(
            agent_id=agent_id,
            health_status=self._health_to_proto(health_status),
            metrics=self._dict_to_struct(metrics or {})
        )

        try:
            await self._stub.UpdateHealth(request)
            return True
        except grpc.RpcError as e:
            if e.code() == grpc.StatusCode.NOT_FOUND:
                return False
            raise

    async def heartbeat(
            self,
            agent_id: str,
            metrics: Optional[Dict[str, Any]] = None
    ) -> bool:
        """Send heartbeat via gRPC"""
        from core.ars.grpc import ars_pb2

        request = ars_pb2.HeartbeatRequest(
            agent_id=agent_id,
            metrics=self._dict_to_struct(metrics or {})
        )

        try:
            await self._stub.Heartbeat(request)
            return True
        except grpc.RpcError as e:
            if e.code() == grpc.StatusCode.NOT_FOUND:
                return False
            raise

    async def update_capabilities(
            self,
            agent_id: str,
            capabilities: List[Capability]
    ) -> bool:
        """Update capabilities via gRPC"""
        from core.ars.grpc import ars_pb2

        request = ars_pb2.UpdateCapabilitiesRequest(
            agent_id=agent_id,
            capabilities=[
                ars_pb2.Capability(
                    name=cap.name,
                    version=cap.version,
                    description=cap.description,
                    parameters=self._dict_to_struct(cap.parameters)
                )
                for cap in capabilities
            ]
        )

        try:
            await self._stub.UpdateCapabilities(request)
            return True
        except grpc.RpcError as e:
            if e.code() == grpc.StatusCode.NOT_FOUND:
                return False
            raise

    async def get_events(
            self,
            agent_id: Optional[str] = None,
            event_type: Optional[str] = None,
            since: Optional[datetime] = None,
            limit: int = 100
    ) -> List[RegistryEvent]:
        """Get events via gRPC (not streaming)"""
        # This would require a non-streaming RPC method
        # For now, collect from stream
        events = []
        async for event in self.stream_events([event_type] if event_type else None, agent_id):
            events.append(event)
            if len(events) >= limit:
                break
        return events

    async def stream_events(
            self,
            event_types: Optional[List[str]] = None,
            agent_id: Optional[str] = None
    ) -> AsyncIterator[RegistryEvent]:
        """Stream events via gRPC"""
        from core.ars.grpc import ars_pb2

        request = ars_pb2.StreamEventsRequest(
            event_types=event_types or [],
            agent_id=agent_id or ""
        )

        stream = self._stub.StreamEvents(request)

        async for event in stream:
            if event.event_type != "keepalive":
                yield self._proto_to_event(event)

    async def get_statistics(self) -> Dict[str, Any]:
        """Get statistics via gRPC"""
        from google.protobuf import empty_pb2

        response = await self._stub.GetStatistics(empty_pb2.Empty())

        return {
            "total_agents": response.total_agents,
            "status_counts": dict(response.status_counts),
            "health_counts": dict(response.health_counts),
            "capability_counts": dict(response.capability_counts),
            "total_events": response.total_events,
            **self._struct_to_dict(response.metadata)
        }

    # Conversion methods

    def _proto_to_registration(self, agent) -> AgentRegistration:
        """Convert protobuf Agent to AgentRegistration"""
        return AgentRegistration(
            agent_id=agent.agent_id,
            name=agent.name,
            version=agent.version,
            status=self._proto_to_status(agent.status),
            health_status=self._proto_to_health(agent.health_status),
            capabilities=[
                Capability(
                    name=cap.name,
                    version=cap.version,
                    description=cap.description,
                    parameters=self._struct_to_dict(cap.parameters)
                )
                for cap in agent.capabilities
            ],
            endpoints=[
                Endpoint(
                    type=ep.type,
                    url=ep.url,
                    protocol=ep.protocol,
                    metadata=self._struct_to_dict(ep.metadata)
                )
                for ep in agent.endpoints
            ],
            metadata=self._struct_to_dict(agent.metadata),
            metrics=self._struct_to_dict(agent.metrics),
            created_at=agent.created_at.ToDatetime(),
            last_heartbeat=agent.last_heartbeat.ToDatetime()
        )

    def _proto_to_event(self, event) -> RegistryEvent:
        """Convert protobuf Event to RegistryEvent"""
        return RegistryEvent(
            event_id=event.event_id,
            event_type=event.event_type,
            timestamp=event.timestamp.ToDatetime(),
            agent_id=event.agent_id if event.agent_id else None,
            data=self._struct_to_dict(event.data)
        )

    def _status_to_proto(self, status: AgentStatus):
        """Convert AgentStatus to protobuf"""
        from core.ars.grpc import ars_pb2

        mapping = {
            AgentStatus.ACTIVE: ars_pb2.AGENT_STATUS_ACTIVE,
            AgentStatus.INACTIVE: ars_pb2.AGENT_STATUS_INACTIVE,
            AgentStatus.MAINTENANCE: ars_pb2.AGENT_STATUS_MAINTENANCE,
            AgentStatus.DEPRECATED: ars_pb2.AGENT_STATUS_DEPRECATED
        }
        return mapping.get(status, ars_pb2.AGENT_STATUS_UNKNOWN)

    def _proto_to_status(self, status) -> AgentStatus:
        """Convert protobuf to AgentStatus"""
        from core.ars.grpc import ars_pb2

        mapping = {
            ars_pb2.AGENT_STATUS_ACTIVE: AgentStatus.ACTIVE,
            ars_pb2.AGENT_STATUS_INACTIVE: AgentStatus.INACTIVE,
            ars_pb2.AGENT_STATUS_MAINTENANCE: AgentStatus.MAINTENANCE,
            ars_pb2.AGENT_STATUS_DEPRECATED: AgentStatus.DEPRECATED
        }
        return mapping.get(status, AgentStatus.ACTIVE)

    def _health_to_proto(self, health: HealthStatus):
        """Convert HealthStatus to protobuf"""
        from core.ars.grpc import ars_pb2

        mapping = {
            HealthStatus.HEALTHY: ars_pb2.HEALTH_STATUS_HEALTHY,
            HealthStatus.DEGRADED: ars_pb2.HEALTH_STATUS_DEGRADED,
            HealthStatus.UNHEALTHY: ars_pb2.HEALTH_STATUS_UNHEALTHY,
            HealthStatus.UNKNOWN: ars_pb2.HEALTH_STATUS_UNKNOWN
        }
        return mapping.get(health, ars_pb2.HEALTH_STATUS_UNKNOWN)

    def _proto_to_health(self, health) -> HealthStatus:
        """Convert protobuf to HealthStatus"""
        from core.ars.grpc import ars_pb2

        mapping = {
            ars_pb2.HEALTH_STATUS_HEALTHY: HealthStatus.HEALTHY,
            ars_pb2.HEALTH_STATUS_DEGRADED: HealthStatus.DEGRADED,
            ars_pb2.HEALTH_STATUS_UNHEALTHY: HealthStatus.UNHEALTHY,
            ars_pb2.HEALTH_STATUS_UNKNOWN: HealthStatus.UNKNOWN
        }
        return mapping.get(health, HealthStatus.UNKNOWN)

    def _search_strategy_to_proto(self, strategy: SearchStrategy):
        """Convert SearchStrategy to protobuf"""
        from core.ars.grpc import ars_pb2

        mapping = {
            SearchStrategy.EXACT: ars_pb2.SEARCH_STRATEGY_EXACT,
            SearchStrategy.FUZZY: ars_pb2.SEARCH_STRATEGY_FUZZY,
            SearchStrategy.SEMANTIC: ars_pb2.SEARCH_STRATEGY_SEMANTIC,
            SearchStrategy.HYBRID: ars_pb2.SEARCH_STRATEGY_HYBRID
        }
        return mapping.get(strategy, ars_pb2.SEARCH_STRATEGY_HYBRID)

    def _dict_to_struct(self, data: Dict[str, Any]):
        """Convert dict to protobuf Struct"""
        from google.protobuf import struct_pb2

        struct = struct_pb2.Struct()
        if data:
            struct.update(data)
        return struct

    def _struct_to_dict(self, struct) -> Dict[str, Any]:
        """Convert protobuf Struct to dict"""
        from google.protobuf.json_format import MessageToDict

        if struct:
            return MessageToDict(struct)
        return {}

    # Convenience functions and utilities

    class AgentManager:
        """
        High-level agent management class.
        Provides simplified interface for common agent operations.
        """

        def __init__(self, client: ARSClient):
            self.client = client
            self._heartbeat_tasks: Dict[str, asyncio.Task] = {}
            self._event_handlers: Dict[str, List[Callable]] = defaultdict(list)
            self._running = False

        async def start(self) -> None:
            """Start agent manager"""
            self._running = True

            # Start event monitoring
            asyncio.create_task(self._monitor_events())

        async def stop(self) -> None:
            """Stop agent manager"""
            self._running = False

            # Cancel heartbeat tasks
            for task in self._heartbeat_tasks.values():
                task.cancel()

            # Wait for tasks to complete
            if self._heartbeat_tasks:
                await asyncio.gather(
                    *self._heartbeat_tasks.values(),
                    return_exceptions=True
                )

        async def register_and_maintain(
                self,
                name: str,
                version: str,
                capabilities: List[Capability],
                endpoints: List[Endpoint],
                metadata: Optional[Dict[str, Any]] = None,
                heartbeat_interval: int = 30,
                heartbeat_callback: Optional[Callable] = None
        ) -> AgentRegistration:
            """
            Register an agent and automatically maintain its heartbeat.

            Args:
                name: Agent name
                version: Agent version
                capabilities: Agent capabilities
                endpoints: Agent endpoints
                metadata: Agent metadata
                heartbeat_interval: Heartbeat interval in seconds
                heartbeat_callback: Optional callback to get metrics

            Returns:
                Agent registration
            """
            # Register agent
            registration = await self.client.register_agent(
                name=name,
                version=version,
                capabilities=capabilities,
                endpoints=endpoints,
                metadata=metadata
            )

            # Start heartbeat
            self._heartbeat_tasks[registration.agent_id] = asyncio.create_task(
                self._heartbeat_loop(
                    registration.agent_id,
                    heartbeat_interval,
                    heartbeat_callback
                )
            )

            return registration

        async def deregister_and_cleanup(self, agent_id: str) -> bool:
            """Deregister agent and cleanup resources"""
            # Stop heartbeat
            if agent_id in self._heartbeat_tasks:
                self._heartbeat_tasks[agent_id].cancel()
                await asyncio.gather(
                    self._heartbeat_tasks[agent_id],
                    return_exceptions=True
                )
                del self._heartbeat_tasks[agent_id]

            # Deregister agent
            return await self.client.deregister_agent(agent_id)

        def on_event(self, event_type: str, handler: Callable) -> None:
            """Register event handler"""
            self._event_handlers[event_type].append(handler)

        def off_event(self, event_type: str, handler: Callable) -> None:
            """Unregister event handler"""
            if handler in self._event_handlers[event_type]:
                self._event_handlers[event_type].remove(handler)

        async def find_best_agent(
                self,
                capabilities: List[str],
                prefer_healthy: bool = True,
                max_results: int = 5
        ) -> List[AgentRegistration]:
            """
            Find the best agents matching capabilities.

            Args:
                capabilities: Required capabilities
                prefer_healthy: Prefer healthy agents
                max_results: Maximum results to return

            Returns:
                List of best matching agents
            """
            # Discover agents
            agents = await self.client.discover_agents(
                capabilities=capabilities,
                require_all_capabilities=True,
                search_strategy=SearchStrategy.HYBRID,
                limit=max_results * 3  # Get more to filter
            )

            if not agents:
                return []

            # Score agents
            scored_agents = []
            for agent in agents:
                score = 0.0

                # Health score
                if agent.health_status == HealthStatus.HEALTHY:
                    score += 1.0 if prefer_healthy else 0.5
                elif agent.health_status == HealthStatus.DEGRADED:
                    score += 0.5 if prefer_healthy else 0.3

                # Capability match score
                agent_caps = {cap.name for cap in agent.capabilities}
                matched = len(set(capabilities) & agent_caps)
                score += matched / len(capabilities)

                # Recency score (prefer recently updated)
                time_since_heartbeat = datetime.utcnow() - agent.last_heartbeat
                if time_since_heartbeat < timedelta(minutes=1):
                    score += 0.5
                elif time_since_heartbeat < timedelta(minutes=5):
                    score += 0.3

                scored_agents.append((agent, score))

            # Sort by score
            scored_agents.sort(key=lambda x: x[1], reverse=True)

            return [agent for agent, _ in scored_agents[:max_results]]

        async def _heartbeat_loop(
                self,
                agent_id: str,
                interval: int,
                callback: Optional[Callable] = None
        ) -> None:
            """Background heartbeat loop"""
            while self._running:
                try:
                    # Get metrics if callback provided
                    metrics = None
                    if callback:
                        if asyncio.iscoroutinefunction(callback):
                            metrics = await callback()
                        else:
                            metrics = callback()

                    # Send heartbeat
                    success = await self.client.heartbeat(agent_id, metrics)

                    if not success:
                        logger.warning(f"Heartbeat failed for agent {agent_id}")

                    # Wait for next interval
                    await asyncio.sleep(interval)

                except asyncio.CancelledError:
                    break
                except Exception as e:
                    logger.error(f"Error in heartbeat loop for {agent_id}: {e}")
                    await asyncio.sleep(interval)

        async def _monitor_events(self) -> None:
            """Monitor and dispatch events"""
            while self._running:
                try:
                    async for event in self.client.stream_events():
                        # Dispatch to handlers
                        handlers = self._event_handlers.get(event.event_type, [])
                        handlers.extend(self._event_handlers.get("*", []))  # Wildcard

                        for handler in handlers:
                            try:
                                if asyncio.iscoroutinefunction(handler):
                                    await handler(event)
                                else:
                                    handler(event)
                            except Exception as e:
                                logger.error(f"Error in event handler: {e}")

                except Exception as e:
                    logger.error(f"Error in event monitor: {e}")
                    await asyncio.sleep(5)  # Retry after delay

    # Batch operations helper

    class BatchOperations:
        """Helper for batch operations on ARS"""

        def __init__(self, client: ARSClient):
            self.client = client

        async def register_multiple(
                self,
                agents: List[Dict[str, Any]],
                parallel: int = 10
        ) -> List[Union[AgentRegistration, Exception]]:
            """Register multiple agents in parallel"""
            semaphore = asyncio.Semaphore(parallel)

            async def register_with_limit(agent_data):
                async with semaphore:
                    try:
                        return await self.client.register_agent(**agent_data)
                    except Exception as e:
                        return e

            tasks = [register_with_limit(agent) for agent in agents]
            return await asyncio.gather(*tasks)

        async def update_health_multiple(
                self,
                updates: List[Dict[str, Any]],
                parallel: int = 20
        ) -> List[bool]:
            """Update health for multiple agents"""
            semaphore = asyncio.Semaphore(parallel)

            async def update_with_limit(update):
                async with semaphore:
                    try:
                        return await self.client.update_health(**update)
                    except:
                        return False

            tasks = [update_with_limit(update) for update in updates]
            return await asyncio.gather(*tasks)

        async def discover_by_patterns(
                self,
                patterns: List[Dict[str, Any]]
        ) -> Dict[str, List[AgentRegistration]]:
            """Discover agents using multiple patterns"""
            results = {}

            for idx, pattern in enumerate(patterns):
                key = pattern.get("name", f"pattern_{idx}")
                agents = await self.client.discover_agents(**pattern)
                results[key] = agents

            return results

    # Testing utilities

    class MockARSClient(ARSClient):
        """Mock ARS client for testing"""

        def __init__(self):
            super().__init__(ClientConfig(protocol=ClientProtocol.REST))
            self._agents: Dict[str, AgentRegistration] = {}
            self._events: List[RegistryEvent] = []

        async def connect(self) -> None:
            pass

        async def close(self) -> None:
            pass

        async def register_agent(
                self,
                name: str,
                version: str,
                capabilities: List[Capability],
                endpoints: List[Endpoint],
                metadata: Optional[Dict[str, Any]] = None,
                agent_id: Optional[str] = None
        ) -> AgentRegistration:
            agent_id = agent_id or f"{name}-{len(self._agents)}"

            registration = AgentRegistration(
                agent_id=agent_id,
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

            self._agents[agent_id] = registration
            return registration

        async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
            return self._agents.get(agent_id)

        async def discover_agents(self, **kwargs) -> List[AgentRegistration]:
            # Simple filtering
            results = list(self._agents.values())

            if kwargs.get("capabilities"):
                required_caps = set(kwargs["capabilities"])
                results = [
                    agent for agent in results
                    if required_caps.issubset({cap.name for cap in agent.capabilities})
                ]

            return results


# Export public API
__all__ = [
    "ARSClient",
    "ARSRestClient",
    "ARSGrpcClient",
    "ClientConfig",
    "ClientProtocol",
    "AgentManager",
    "BatchOperations",
    "MockARSClient"
]
