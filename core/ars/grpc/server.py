# File: maple/core/ars/grpc/server.py
# Description: gRPC server implementation for the Agent Registry Service.
# Provides high-performance RPC interface for agent management.

from __future__ import annotations
import asyncio
import grpc
from concurrent import futures
from datetime import datetime, timedelta
from typing import AsyncIterator, Optional, Dict, Any, List
import logging
from google.protobuf import timestamp_pb2, empty_pb2, duration_pb2
from grpc_reflection.v1alpha import reflection

from maple.core.ars.registry import RegistryManager, create_registry_manager
from maple.core.ars.models.registry import (
    AgentRegistration, ServiceQuery, Capability,
    AgentStatus, HealthStatus, Endpoint
)
from maple.core.ars.discovery import SearchStrategy
from maple.core.ars.grpc import ars_pb2, ars_pb2_grpc

logger = logging.getLogger(__name__)


class ARSGrpcService(ars_pb2_grpc.AgentRegistryServiceServicer):
    """gRPC service implementation for Agent Registry Service"""

    def __init__(self, registry: RegistryManager):
        self.registry = registry
        self._stream_handlers: Dict[str, asyncio.Queue] = {}

    async def RegisterAgent(
            self,
            request: ars_pb2.RegisterAgentRequest,
            context: grpc.aio.ServicerContext
    ) -> ars_pb2.RegisterAgentResponse:
        """Register a new agent"""
        try:
            # Convert protobuf to internal models
            capabilities = [
                Capability(
                    name=cap.name,
                    version=cap.version,
                    description=cap.description,
                    parameters=self._proto_struct_to_dict(cap.parameters)
                )
                for cap in request.capabilities
            ]

            endpoints = [
                Endpoint(
                    type=ep.type,
                    url=ep.url,
                    protocol=ep.protocol,
                    metadata=self._proto_struct_to_dict(ep.metadata)
                )
                for ep in request.endpoints
            ]

            # Register agent
            registration = await self.registry.register_agent(
                name=request.name,
                version=request.version,
                capabilities=capabilities,
                endpoints=endpoints,
                metadata=self._proto_struct_to_dict(request.metadata),
                agent_id=request.agent_id or None
            )

            # Convert to protobuf response
            response = ars_pb2.RegisterAgentResponse(
                agent=self._registration_to_proto(registration)
            )

            return response

        except ValueError as e:
            await context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(e))
        except Exception as e:
            logger.error(f"Failed to register agent: {e}")
            await context.abort(grpc.StatusCode.INTERNAL, "Internal server error")

    async def DeregisterAgent(
            self,
            request: ars_pb2.DeregisterAgentRequest,
            context: grpc.aio.ServicerContext
    ) -> empty_pb2.Empty:
        """Deregister an agent"""
        success = await self.registry.deregister_agent(request.agent_id)

        if not success:
            await context.abort(grpc.StatusCode.NOT_FOUND, "Agent not found")

        return empty_pb2.Empty()

    async def GetAgent(
            self,
            request: ars_pb2.GetAgentRequest,
            context: grpc.aio.ServicerContext
    ) -> ars_pb2.GetAgentResponse:
        """Get agent by ID"""
        agent = await self.registry.get_agent(request.agent_id)

        if not agent:
            await context.abort(grpc.StatusCode.NOT_FOUND, "Agent not found")

        return ars_pb2.GetAgentResponse(
            agent=self._registration_to_proto(agent)
        )

    async def DiscoverAgents(
            self,
            request: ars_pb2.DiscoverAgentsRequest,
            context: grpc.aio.ServicerContext
    ) -> ars_pb2.DiscoverAgentsResponse:
        """Discover agents matching criteria"""
        try:
            # Convert request to internal query
            agents = await self.registry.discover_agents(
                capabilities=list(request.capabilities),
                tags=list(request.tags),
                status=self._proto_to_status(request.status) if request.status else None,
                health_status=self._proto_to_health(request.health_status) if request.health_status else None,
                metadata_filter=self._proto_struct_to_dict(request.metadata_filter),
                require_all_capabilities=request.require_all_capabilities,
                sort_by=request.sort_by or None,
                limit=request.limit or None,
                offset=request.offset or None
            )

            # Convert to protobuf response
            response = ars_pb2.DiscoverAgentsResponse(
                agents=[self._registration_to_proto(agent) for agent in agents],
                total_count=len(agents)
            )

            return response

        except Exception as e:
            logger.error(f"Failed to discover agents: {e}")
            await context.abort(grpc.StatusCode.INTERNAL, "Internal server error")

    async def UpdateHealth(
            self,
            request: ars_pb2.UpdateHealthRequest,
            context: grpc.aio.ServicerContext
    ) -> empty_pb2.Empty:
        """Update agent health status"""
        health_status = self._proto_to_health(request.health_status)
        metrics = self._proto_struct_to_dict(request.metrics)

        success = await self.registry.update_agent_health(
            request.agent_id,
            health_status,
            metrics
        )

        if not success:
            await context.abort(grpc.StatusCode.NOT_FOUND, "Agent not found")

        return empty_pb2.Empty()

    async def Heartbeat(
            self,
            request: ars_pb2.HeartbeatRequest,
            context: grpc.aio.ServicerContext
    ) -> empty_pb2.Empty:
        """Process agent heartbeat"""
        metrics = self._proto_struct_to_dict(request.metrics)

        success = await self.registry.heartbeat(
            request.agent_id,
            metrics
        )

        if not success:
            await context.abort(grpc.StatusCode.NOT_FOUND, "Agent not found")

        return empty_pb2.Empty()

    async def UpdateCapabilities(
            self,
            request: ars_pb2.UpdateCapabilitiesRequest,
            context: grpc.aio.ServicerContext
    ) -> empty_pb2.Empty:
        """Update agent capabilities"""
        capabilities = [
            Capability(
                name=cap.name,
                version=cap.version,
                description=cap.description,
                parameters=self._proto_struct_to_dict(cap.parameters)
            )
            for cap in request.capabilities
        ]

        success = await self.registry.update_agent_capabilities(
            request.agent_id,
            capabilities
        )

        if not success:
            await context.abort(grpc.StatusCode.NOT_FOUND, "Agent not found")

        return empty_pb2.Empty()

    async def StreamEvents(
            self,
            request: ars_pb2.StreamEventsRequest,
            context: grpc.aio.ServicerContext
    ) -> AsyncIterator[ars_pb2.Event]:
        """Stream registry events"""
        # Create event queue for this stream
        stream_id = context.peer()
        event_queue = asyncio.Queue(maxsize=1000)
        self._stream_handlers[stream_id] = event_queue

        try:
            # Subscribe to events
            def event_handler(event):
                try:
                    event_queue.put_nowait(event)
                except asyncio.QueueFull:
                    logger.warning(f"Event queue full for stream {stream_id}")

            # Subscribe based on filters
            event_types = list(request.event_types) if request.event_types else ["*"]
            for event_type in event_types:
                self.registry.subscribe(event_type, event_handler)

            # Stream events
            while not context.cancelled():
                try:
                    # Wait for event with timeout
                    event = await asyncio.wait_for(
                        event_queue.get(),
                        timeout=30.0  # Send keepalive every 30s
                    )

                    # Convert to protobuf
                    proto_event = ars_pb2.Event(
                        event_id=event.event_id,
                        event_type=event.event_type,
                        timestamp=self._datetime_to_timestamp(event.timestamp),
                        agent_id=event.agent_id or "",
                        data=self._dict_to_proto_struct(event.data)
                    )

                    yield proto_event

                except asyncio.TimeoutError:
                    # Send keepalive event
                    yield ars_pb2.Event(
                        event_type="keepalive",
                        timestamp=self._datetime_to_timestamp(datetime.utcnow())
                    )

        finally:
            # Clean up
            del self._stream_handlers[stream_id]
            for event_type in event_types:
                self.registry.unsubscribe(event_type, event_handler)

    async def GetStatistics(
            self,
            request: empty_pb2.Empty,
            context: grpc.aio.ServicerContext
    ) -> ars_pb2.GetStatisticsResponse:
        """Get registry statistics"""
        stats = await self.registry.get_statistics()

        return ars_pb2.GetStatisticsResponse(
            total_agents=stats.get("total_agents", 0),
            status_counts=stats.get("status_counts", {}),
            health_counts=stats.get("health_counts", {}),
            capability_counts=stats.get("capability_counts", {}),
            total_events=stats.get("total_events", 0),
            metadata=self._dict_to_proto_struct(stats)
        )

    async def BatchDiscover(
            self,
            request: ars_pb2.BatchDiscoverRequest,
            context: grpc.aio.ServicerContext
    ) -> ars_pb2.BatchDiscoverResponse:
        """Batch discovery of agents"""
        results = []

        for discover_request in request.requests:
            try:
                agents = await self.registry.discover_agents(
                    capabilities=list(discover_request.capabilities),
                    tags=list(discover_request.tags),
                    status=self._proto_to_status(discover_request.status) if discover_request.status else None,
                    health_status=self._proto_to_health(
                        discover_request.health_status) if discover_request.health_status else None,
                    metadata_filter=self._proto_struct_to_dict(discover_request.metadata_filter),
                    require_all_capabilities=discover_request.require_all_capabilities,
                    sort_by=discover_request.sort_by or None,
                    limit=discover_request.limit or None,
                    offset=discover_request.offset or None
                )

                result = ars_pb2.BatchDiscoverResponse.BatchResult(
                    success=True,
                    agents=[self._registration_to_proto(agent) for agent in agents],
                    total_count=len(agents)
                )

            except Exception as e:
                result = ars_pb2.BatchDiscoverResponse.BatchResult(
                    success=False,
                    error_message=str(e)
                )

            results.append(result)

        return ars_pb2.BatchDiscoverResponse(results=results)

    # Streaming RPC for bidirectional communication

    async def AgentStream(
            self,
            request_iterator: AsyncIterator[ars_pb2.AgentStreamRequest],
            context: grpc.aio.ServicerContext
    ) -> AsyncIterator[ars_pb2.AgentStreamResponse]:
        """Bidirectional streaming for agent communication"""
        agent_id = None

        try:
            async for request in request_iterator:
                # Handle different request types
                if request.HasField("register"):
                    # Register agent
                    reg_req = request.register
                    capabilities = [
                        Capability(
                            name=cap.name,
                            version=cap.version,
                            description=cap.description,
                            parameters=self._proto_struct_to_dict(cap.parameters)
                        )
                        for cap in reg_req.capabilities
                    ]

                    endpoints = [
                        Endpoint(
                            type=ep.type,
                            url=ep.url,
                            protocol=ep.protocol,
                            metadata=self._proto_struct_to_dict(ep.metadata)
                        )
                        for ep in reg_req.endpoints
                    ]

                    registration = await self.registry.register_agent(
                        name=reg_req.name,
                        version=reg_req.version,
                        capabilities=capabilities,
                        endpoints=endpoints,
                        metadata=self._proto_struct_to_dict(reg_req.metadata),
                        agent_id=reg_req.agent_id or None
                    )

                    agent_id = registration.agent_id

                    yield ars_pb2.AgentStreamResponse(
                        registered=ars_pb2.AgentStreamResponse.Registered(
                            agent_id=agent_id
                        )
                    )

                elif request.HasField("heartbeat"):
                    # Process heartbeat
                    hb_req = request.heartbeat
                    success = await self.registry.heartbeat(
                        hb_req.agent_id,
                        self._proto_struct_to_dict(hb_req.metrics)
                    )

                    yield ars_pb2.AgentStreamResponse(
                        ack=ars_pb2.AgentStreamResponse.Acknowledgment(
                            success=success
                        )
                    )

                elif request.HasField("update_health"):
                    # Update health
                    health_req = request.update_health
                    success = await self.registry.update_agent_health(
                        health_req.agent_id,
                        self._proto_to_health(health_req.health_status),
                        self._proto_struct_to_dict(health_req.metrics)
                    )

                    yield ars_pb2.AgentStreamResponse(
                        ack=ars_pb2.AgentStreamResponse.Acknowledgment(
                            success=success
                        )
                    )

        except Exception as e:
            logger.error(f"Error in agent stream: {e}")
            yield ars_pb2.AgentStreamResponse(
                error=ars_pb2.AgentStreamResponse.Error(
                    code=grpc.StatusCode.INTERNAL.value[0],
                    message=str(e)
                )
            )

    # Helper methods for conversions

    def _registration_to_proto(self, reg: AgentRegistration) -> ars_pb2.Agent:
        """Convert AgentRegistration to protobuf"""
        return ars_pb2.Agent(
            agent_id=reg.agent_id,
            name=reg.name,
            version=reg.version,
            status=self._status_to_proto(reg.status),
            health_status=self._health_to_proto(reg.health_status),
            capabilities=[
                ars_pb2.Capability(
                    name=cap.name,
                    version=cap.version,
                    description=cap.description,
                    parameters=self._dict_to_proto_struct(cap.parameters)
                )
                for cap in reg.capabilities
            ],
            endpoints=[
                ars_pb2.Endpoint(
                    type=ep.type,
                    url=ep.url,
                    protocol=ep.protocol,
                    metadata=self._dict_to_proto_struct(ep.metadata)
                )
                for ep in reg.endpoints
            ],
            metadata=self._dict_to_proto_struct(reg.metadata),
            created_at=self._datetime_to_timestamp(reg.created_at),
            last_heartbeat=self._datetime_to_timestamp(reg.last_heartbeat)
        )

    def _status_to_proto(self, status: AgentStatus) -> ars_pb2.AgentStatus:
        """Convert AgentStatus to protobuf"""
        return {
            AgentStatus.ACTIVE: ars_pb2.AGENT_STATUS_ACTIVE,
            AgentStatus.INACTIVE: ars_pb2.AGENT_STATUS_INACTIVE,
            AgentStatus.MAINTENANCE: ars_pb2.AGENT_STATUS_MAINTENANCE,
            AgentStatus.DEPRECATED: ars_pb2.AGENT_STATUS_DEPRECATED
        }.get(status, ars_pb2.AGENT_STATUS_UNKNOWN)

    def _proto_to_status(self, status: ars_pb2.AgentStatus) -> AgentStatus:
        """Convert protobuf to AgentStatus"""
        return {
            ars_pb2.AGENT_STATUS_ACTIVE: AgentStatus.ACTIVE,
            ars_pb2.AGENT_STATUS_INACTIVE: AgentStatus.INACTIVE,
            ars_pb2.AGENT_STATUS_MAINTENANCE: AgentStatus.MAINTENANCE,
            ars_pb2.AGENT_STATUS_DEPRECATED: AgentStatus.DEPRECATED
        }.get(status, AgentStatus.ACTIVE)

    def _health_to_proto(self, health: HealthStatus) -> ars_pb2.HealthStatus:
        """Convert HealthStatus to protobuf"""
        return {
            HealthStatus.HEALTHY: ars_pb2.HEALTH_STATUS_HEALTHY,
            HealthStatus.DEGRADED: ars_pb2.HEALTH_STATUS_DEGRADED,
            HealthStatus.UNHEALTHY: ars_pb2.HEALTH_STATUS_UNHEALTHY,
            HealthStatus.UNKNOWN: ars_pb2.HEALTH_STATUS_UNKNOWN
        }.get(health, ars_pb2.HEALTH_STATUS_UNKNOWN)

    def _proto_to_health(self, health: ars_pb2.HealthStatus) -> HealthStatus:
        """Convert protobuf to HealthStatus"""
        return {
            ars_pb2.HEALTH_STATUS_HEALTHY: HealthStatus.HEALTHY,
            ars_pb2.HEALTH_STATUS_DEGRADED: HealthStatus.DEGRADED,
            ars_pb2.HEALTH_STATUS_UNHEALTHY: HealthStatus.UNHEALTHY,
            ars_pb2.HEALTH_STATUS_UNKNOWN: HealthStatus.UNKNOWN
        }.get(health, HealthStatus.UNKNOWN)

    def _datetime_to_timestamp(self, dt: datetime) -> timestamp_pb2.Timestamp:
        """Convert datetime to protobuf timestamp"""
        timestamp = timestamp_pb2.Timestamp()
        timestamp.FromDatetime(dt)
        return timestamp

    def _dict_to_proto_struct(self, data: Dict[str, Any]) -> Any:
        """Convert dict to protobuf Struct"""
        # Simplified implementation - would use google.protobuf.struct_pb2
        return data

    def _proto_struct_to_dict(self, struct: Any) -> Dict[str, Any]:
        """Convert protobuf Struct to dict"""
        # Simplified implementation - would use google.protobuf.struct_pb2
        return struct if isinstance(struct, dict) else {}


class ARSGrpcServer:
    """gRPC server for Agent Registry Service"""

    def __init__(
            self,
            registry: Optional[RegistryManager] = None,
            config: Optional[Dict[str, Any]] = None
    ):
        self.registry = registry
        self.config = config or {}
        self.server: Optional[grpc.aio.Server] = None
        self.port = self.config.get('port', 50051)
        self.max_workers = self.config.get('max_workers', 10)
        self.enable_reflection = self.config.get('enable_reflection', True)

    async def start(self):
        """Start the gRPC server"""
        # Initialize registry if not provided
        if not self.registry:
            self.registry = create_registry_manager(
                backend=self.config.get('storage_backend', 'memory'),
                **self.config.get('registry_config', {})
            )
            await self.registry.start()

        # Create gRPC server
        self.server = grpc.aio.server(
            futures.ThreadPoolExecutor(max_workers=self.max_workers),
            options=[
                ('grpc.max_send_message_length', 50 * 1024 * 1024),  # 50MB
                ('grpc.max_receive_message_length', 50 * 1024 * 1024),  # 50MB
                ('grpc.keepalive_time_ms', 10000),
                ('grpc.keepalive_timeout_ms', 5000),
                ('grpc.keepalive_permit_without_calls', True),
                ('grpc.http2.max_pings_without_data', 0),
                ('grpc.http2.min_time_between_pings_ms', 10000),
            ]
        )

        # Add service
        service = ARSGrpcService(self.registry)
        ars_pb2_grpc.add_AgentRegistryServiceServicer_to_server(service, self.server)

        # Enable reflection for debugging
        if self.enable_reflection:
            SERVICE_NAMES = (
                ars_pb2.DESCRIPTOR.services_by_name['AgentRegistryService'].full_name,
                reflection.SERVICE_NAME,
            )
            reflection.enable_server_reflection(SERVICE_NAMES, self.server)

        # Start server
        listen_addr = f'[::]:{self.port}'
        self.server.add_insecure_port(listen_addr)

        await self.server.start()
        logger.info(f"gRPC server started on {listen_addr}")

        # Wait for termination
        await self.server.wait_for_termination()

    async def stop(self):
        """Stop the gRPC server"""
        if self.server:
            await self.server.stop(grace=5.0)

        if self.registry:
            await self.registry.stop()

        logger.info("gRPC server stopped")


# Interceptors for authentication, logging, etc.

class AuthenticationInterceptor(grpc.aio.ServerInterceptor):
    """gRPC interceptor for authentication"""

    def __init__(self, auth_token: str):
        self.auth_token = auth_token

    async def intercept_service(
            self,
            continuation,
            handler_call_details
    ):
        # Extract metadata
        metadata = dict(handler_call_details.invocation_metadata)

        # Check authentication
        if metadata.get('authorization') != f'Bearer {self.auth_token}':
            async def abort(request, context):
                await context.abort(
                    grpc.StatusCode.UNAUTHENTICATED,
                    'Invalid authentication token'
                )

            return grpc.unary_unary_rpc_method_handler(abort)

        return await continuation(handler_call_details)


class LoggingInterceptor(grpc.aio.ServerInterceptor):
    """gRPC interceptor for request logging"""

    async def intercept_service(
            self,
            continuation,
            handler_call_details
    ):
        start_time = datetime.utcnow()
        method = handler_call_details.method

        # Log request
        logger.info(f"gRPC request: {method}")

        # Continue with the handler
        handler = await continuation(handler_call_details)

        # Wrap handler to log response
        if handler and handler.unary_unary:
            original_handler = handler.unary_unary

            async def logging_wrapper(request, context):
                try:
                    response = await original_handler(request, context)
                    duration = (datetime.utcnow() - start_time).total_seconds()
                    logger.info(
                        f"gRPC response: {method} "
                        f"status=OK duration={duration:.3f}s"
                    )
                    return response
                except Exception as e:
                    duration = (datetime.utcnow() - start_time).total_seconds()
                    logger.error(
                        f"gRPC error: {method} "
                        f"error={e} duration={duration:.3f}s"
                    )
                    raise

            return grpc.unary_unary_rpc_method_handler(
                logging_wrapper,
                request_deserializer=handler.request_deserializer,
                response_serializer=handler.response_serializer
            )

        return handler


# CLI entry point

def main():
    """Main entry point for running the gRPC server"""
    import argparse

    parser = argparse.ArgumentParser(description="MAPLE Agent Registry Service gRPC Server")
    parser.add_argument("--port", type=int, default=50051, help="Port to bind to")
    parser.add_argument("--backend", default="memory", help="Storage backend")
    parser.add_argument("--workers", type=int, default=10, help="Max worker threads")
    parser.add_argument("--log-level", default="info", help="Log level")

    args = parser.parse_args()

    # Configure logging
    logging.basicConfig(
        level=getattr(logging, args.log_level.upper()),
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    )

    # Create and run server
    config = {
        'port': args.port,
        'storage_backend': args.backend,
        'max_workers': args.workers,
        'log_level': args.log_level
    }

    server = ARSGrpcServer(config=config)

    # Run server
    asyncio.run(server.start())


if __name__ == "__main__":
    main()

# Export public API
__all__ = [
    "ARSGrpcService",
    "ARSGrpcServer",
    "AuthenticationInterceptor",
    "LoggingInterceptor"
]