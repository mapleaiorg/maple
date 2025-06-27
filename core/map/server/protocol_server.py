# File: maple/core/map/server/protocol_server.py
# Description: Main MAP Protocol Server that integrates all components and provides
# the primary service interface for the Multi-Agent Protocol. This server handles
# incoming messages, manages agent connections, and orchestrates all MAP operations.

from __future__ import annotations
import asyncio
import logging
import signal
import sys
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Set, Any, Callable
from uuid import UUID
import json
from aiohttp import web
import aiohttp_cors
from prometheus_client import Counter, Histogram, Gauge, generate_latest
import aiokafka
from dataclasses import dataclass, field

from maple.core.map.models.message import (
    MAPMessage, MessageType, MessagePriority,
    DeliveryMode, AgentIdentifier, MessageDestination
)
from maple.core.map.routing.engine import RoutingEngine, AgentRoute
from maple.core.map.transport.base import TransportManager, DeliveryReceipt
from maple.core.map.orchestration.workflow import WorkflowEngine

logger = logging.getLogger(__name__)

# Prometheus metrics
message_counter = Counter(
    'map_messages_total',
    'Total number of messages processed',
    ['message_type', 'priority', 'status']
)

message_duration = Histogram(
    'map_message_duration_seconds',
    'Time spent processing messages',
    ['message_type', 'priority']
)

active_agents = Gauge(
    'map_active_agents',
    'Number of active agents',
    ['service_type']
)

active_connections = Gauge(
    'map_active_connections',
    'Number of active connections',
    ['transport_type']
)


@dataclass
class ServerConfig:
    """MAP Protocol Server configuration"""
    host: str = "0.0.0.0"
    port: int = 8080
    kafka_brokers: List[str] = field(default_factory=lambda: ["localhost:9092"])
    kafka_topic_prefix: str = "maple.map"
    enable_metrics: bool = True
    metrics_port: int = 9090
    enable_auth: bool = True
    auth_secret: str = "change-me-in-production"
    max_message_size: int = 10 * 1024 * 1024  # 10MB
    request_timeout: int = 300  # 5 minutes
    cleanup_interval: int = 3600  # 1 hour
    enable_clustering: bool = False
    cluster_nodes: List[str] = field(default_factory=list)
    node_id: str = "map-node-1"
    ssl_cert: Optional[str] = None
    ssl_key: Optional[str] = None


class MAPProtocolServer:
    """Main MAP Protocol Server implementation"""

    def __init__(self, config: ServerConfig):
        self.config = config
        self.routing_engine = RoutingEngine()
        self.transport_manager = TransportManager()
        self.workflow_engine = WorkflowEngine(self.routing_engine, self.transport_manager)

        # HTTP app
        self.app = web.Application(
            client_max_size=config.max_message_size
        )
        self._setup_routes()
        self._setup_cors()

        # Kafka components
        self.kafka_producer: Optional[aiokafka.AIOKafkaProducer] = None
        self.kafka_consumer: Optional[aiokafka.AIOKafkaConsumer] = None

        # WebSocket connections
        self.ws_connections: Dict[str, web.WebSocketResponse] = {}

        # Message handlers
        self.message_handlers: Dict[str, List[Callable]] = {}

        # Background tasks
        self.background_tasks: List[asyncio.Task] = []

        # Server state
        self.is_running = False
        self.start_time = datetime.utcnow()

    def _setup_routes(self):
        """Configure HTTP routes"""
        # Message endpoints
        self.app.router.add_post('/api/v1/messages', self.handle_message)
        self.app.router.add_post('/api/v1/messages/batch', self.handle_batch_messages)
        self.app.router.add_get('/api/v1/messages/{message_id}', self.get_message_status)

        # Agent management
        self.app.router.add_post('/api/v1/agents/register', self.register_agent)
        self.app.router.add_delete('/api/v1/agents/{agent_id}', self.unregister_agent)
        self.app.router.add_post('/api/v1/agents/{agent_id}/heartbeat', self.agent_heartbeat)
        self.app.router.add_get('/api/v1/agents', self.list_agents)
        self.app.router.add_get('/api/v1/agents/{agent_id}', self.get_agent_info)

        # Workflow management
        self.app.router.add_post('/api/v1/workflows', self.create_workflow)
        self.app.router.add_post('/api/v1/workflows/{workflow_id}/start', self.start_workflow)
        self.app.router.add_get('/api/v1/workflows/{instance_id}/status', self.get_workflow_status)
        self.app.router.add_post('/api/v1/workflows/{instance_id}/cancel', self.cancel_workflow)

        # WebSocket endpoint
        self.app.router.add_get('/api/v1/ws', self.websocket_handler)

        # Health and metrics
        self.app.router.add_get('/health', self.health_check)
        self.app.router.add_get('/ready', self.readiness_check)
        if self.config.enable_metrics:
            self.app.router.add_get('/metrics', self.metrics_handler)

        # Admin endpoints
        self.app.router.add_get('/api/v1/admin/stats', self.get_server_stats)
        self.app.router.add_post('/api/v1/admin/broadcast', self.broadcast_message)

    def _setup_cors(self):
        """Configure CORS for the application"""
        cors = aiohttp_cors.setup(self.app, defaults={
            "*": aiohttp_cors.ResourceOptions(
                allow_credentials=True,
                expose_headers="*",
                allow_headers="*",
                allow_methods="*"
            )
        })

        for route in list(self.app.router.routes()):
            cors.add(route)

    async def start(self):
        """Start the MAP Protocol Server"""
        logger.info(f"Starting MAP Protocol Server on {self.config.host}:{self.config.port}")

        self.is_running = True

        # Initialize Kafka
        await self._init_kafka()

        # Start background tasks
        self._start_background_tasks()

        # Setup signal handlers
        for sig in (signal.SIGTERM, signal.SIGINT):
            signal.signal(sig, lambda s, f: asyncio.create_task(self.shutdown()))

        # Start metrics server if enabled
        if self.config.enable_metrics:
            asyncio.create_task(self._start_metrics_server())

        # Start main HTTP server
        runner = web.AppRunner(self.app)
        await runner.setup()

        if self.config.ssl_cert and self.config.ssl_key:
            import ssl
            ssl_context = ssl.create_default_context(ssl.Purpose.CLIENT_AUTH)
            ssl_context.load_cert_chain(self.config.ssl_cert, self.config.ssl_key)
            site = web.TCPSite(runner, self.config.host, self.config.port, ssl_context=ssl_context)
        else:
            site = web.TCPSite(runner, self.config.host, self.config.port)

        await site.start()

        logger.info(f"MAP Protocol Server started successfully")

        # Keep server running
        while self.is_running:
            await asyncio.sleep(1)

    async def shutdown(self):
        """Gracefully shutdown the server"""
        logger.info("Shutting down MAP Protocol Server...")

        self.is_running = False

        # Cancel background tasks
        for task in self.background_tasks:
            task.cancel()

        # Close WebSocket connections
        for ws in self.ws_connections.values():
            await ws.close()

        # Shutdown Kafka
        if self.kafka_producer:
            await self.kafka_producer.stop()
        if self.kafka_consumer:
            await self.kafka_consumer.stop()

        # Shutdown transport manager
        await self.transport_manager.close_all()

        # Shutdown workflow engine
        await self.workflow_engine.shutdown()

        logger.info("MAP Protocol Server shutdown complete")

    async def _init_kafka(self):
        """Initialize Kafka producer and consumer"""
        try:
            # Create producer
            self.kafka_producer = aiokafka.AIOKafkaProducer(
                bootstrap_servers=self.config.kafka_brokers,
                value_serializer=lambda v: json.dumps(v).encode('utf-8'),
                compression_type='gzip'
            )
            await self.kafka_producer.start()

            # Create consumer
            self.kafka_consumer = aiokafka.AIOKafkaConsumer(
                f"{self.config.kafka_topic_prefix}.messages",
                f"{self.config.kafka_topic_prefix}.events",
                bootstrap_servers=self.config.kafka_brokers,
                group_id=f"map-server-{self.config.node_id}",
                value_deserializer=lambda v: json.loads(v.decode('utf-8')),
                auto_offset_reset='latest'
            )
            await self.kafka_consumer.start()

            # Start consumer loop
            self.background_tasks.append(
                asyncio.create_task(self._kafka_consumer_loop())
            )

            logger.info("Kafka initialization complete")

        except Exception as e:
            logger.error(f"Failed to initialize Kafka: {str(e)}")
            # Continue without Kafka in development mode

    async def _kafka_consumer_loop(self):
        """Process messages from Kafka"""
        async for msg in self.kafka_consumer:
            try:
                if msg.topic.endswith('.messages'):
                    await self._process_kafka_message(msg.value)
                elif msg.topic.endswith('.events'):
                    await self._process_kafka_event(msg.value)
            except Exception as e:
                logger.error(f"Error processing Kafka message: {str(e)}")

    def _start_background_tasks(self):
        """Start background maintenance tasks"""
        # Cleanup task
        self.background_tasks.append(
            asyncio.create_task(self._cleanup_loop())
        )

        # Health check task
        self.background_tasks.append(
            asyncio.create_task(self._health_monitor_loop())
        )

        # Metrics collection
        if self.config.enable_metrics:
            self.background_tasks.append(
                asyncio.create_task(self._metrics_collection_loop())
            )

    async def _cleanup_loop(self):
        """Periodic cleanup of expired data"""
        while self.is_running:
            try:
                await asyncio.sleep(self.config.cleanup_interval)

                # Cleanup completed workflows
                await self.workflow_engine.cleanup_completed_instances(
                    timedelta(days=7)
                )

                # Cleanup stale WebSocket connections
                stale_connections = []
                for agent_id, ws in self.ws_connections.items():
                    if ws.closed:
                        stale_connections.append(agent_id)

                for agent_id in stale_connections:
                    del self.ws_connections[agent_id]

            except Exception as e:
                logger.error(f"Error in cleanup loop: {str(e)}")

    async def _health_monitor_loop(self):
        """Monitor health of connected agents"""
        while self.is_running:
            try:
                await asyncio.sleep(30)  # Check every 30 seconds

                # Get all registered agents
                stats = await self.routing_engine.get_routing_stats()

                # Update metrics
                for service, count in stats['service_distribution'].items():
                    active_agents.labels(service_type=service).set(count)

            except Exception as e:
                logger.error(f"Error in health monitor loop: {str(e)}")

    async def _metrics_collection_loop(self):
        """Collect and update metrics"""
        while self.is_running:
            try:
                await asyncio.sleep(60)  # Update every minute

                # Update connection metrics
                active_connections.labels(transport_type='websocket').set(
                    len(self.ws_connections)
                )

            except Exception as e:
                logger.error(f"Error in metrics collection: {str(e)}")

    # HTTP Handlers

    async def handle_message(self, request: web.Request) -> web.Response:
        """Handle incoming message"""
        start_time = datetime.utcnow()

        try:
            # Parse message
            data = await request.json()
            message = MAPMessage.from_json(json.dumps(data))

            # Validate message
            if message.is_expired():
                return web.json_response(
                    {"error": "Message expired"},
                    status=400
                )

            # Route message
            route = await self.routing_engine.route_message(message)
            if not route:
                return web.json_response(
                    {"error": "No route found"},
                    status=404
                )

            # Send message
            receipt = await self.transport_manager.send_message(message, route)

            # Update metrics
            duration = (datetime.utcnow() - start_time).total_seconds()
            message_counter.labels(
                message_type=message.payload.type.value,
                priority=message.header.priority.value,
                status='success'
            ).inc()
            message_duration.labels(
                message_type=message.payload.type.value,
                priority=message.header.priority.value
            ).observe(duration)

            # Publish to Kafka for clustering
            if self.config.enable_clustering and self.kafka_producer:
                await self.kafka_producer.send(
                    f"{self.config.kafka_topic_prefix}.messages",
                    {
                        "message": data,
                        "route": route.agent.agent_id,
                        "node_id": self.config.node_id,
                        "timestamp": datetime.utcnow().isoformat()
                    }
                )

            return web.json_response({
                "message_id": str(receipt.message_id),
                "status": receipt.status,
                "delivered_at": receipt.delivered_at.isoformat(),
                "recipient": receipt.recipient
            })

        except Exception as e:
            logger.error(f"Error handling message: {str(e)}")
            message_counter.labels(
                message_type='unknown',
                priority='medium',
                status='error'
            ).inc()
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def handle_batch_messages(self, request: web.Request) -> web.Response:
        """Handle batch of messages"""
        try:
            data = await request.json()
            messages = [MAPMessage.from_json(json.dumps(msg)) for msg in data]

            results = []
            for message in messages:
                try:
                    route = await self.routing_engine.route_message(message)
                    if route:
                        receipt = await self.transport_manager.send_message(message, route)
                        results.append({
                            "message_id": str(message.header.message_id),
                            "status": "delivered",
                            "recipient": route.agent.agent_id
                        })
                    else:
                        results.append({
                            "message_id": str(message.header.message_id),
                            "status": "no_route",
                            "error": "No route found"
                        })
                except Exception as e:
                    results.append({
                        "message_id": str(message.header.message_id),
                        "status": "error",
                        "error": str(e)
                    })

            return web.json_response({"results": results})

        except Exception as e:
            logger.error(f"Error handling batch messages: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def get_message_status(self, request: web.Request) -> web.Response:
        """Get status of a specific message"""
        message_id = request.match_info['message_id']

        # This would query message status from persistence
        # For now, return mock status
        return web.json_response({
            "message_id": message_id,
            "status": "delivered",
            "delivered_at": datetime.utcnow().isoformat()
        })

    async def register_agent(self, request: web.Request) -> web.Response:
        """Register a new agent"""
        try:
            data = await request.json()

            # Validate required fields
            required_fields = ['agent_id', 'service', 'instance', 'endpoint', 'capabilities']
            for field in required_fields:
                if field not in data:
                    return web.json_response(
                        {"error": f"Missing required field: {field}"},
                        status=400
                    )

            # Create agent identifier
            agent = AgentIdentifier(
                agent_id=data['agent_id'],
                service=data['service'],
                instance=data['instance'],
                version=data.get('version')
            )

            # Register with routing engine
            await self.routing_engine.register_agent(
                agent=agent,
                endpoint=data['endpoint'],
                capabilities=data['capabilities'],
                max_concurrent=data.get('max_concurrent', 100)
            )

            # Update metrics
            active_agents.labels(service_type=agent.service).inc()

            # Publish registration event
            if self.kafka_producer:
                await self.kafka_producer.send(
                    f"{self.config.kafka_topic_prefix}.events",
                    {
                        "event": "agent_registered",
                        "agent_id": agent.agent_id,
                        "service": agent.service,
                        "capabilities": data['capabilities'],
                        "timestamp": datetime.utcnow().isoformat()
                    }
                )

            logger.info(f"Agent registered: {agent.agent_id}")

            return web.json_response({
                "status": "registered",
                "agent_id": agent.agent_id,
                "message": "Agent successfully registered"
            })

        except Exception as e:
            logger.error(f"Error registering agent: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def unregister_agent(self, request: web.Request) -> web.Response:
        """Unregister an agent"""
        agent_id = request.match_info['agent_id']

        try:
            await self.routing_engine.unregister_agent(agent_id)

            # Close WebSocket if exists
            if agent_id in self.ws_connections:
                await self.ws_connections[agent_id].close()
                del self.ws_connections[agent_id]

            # Update metrics
            active_agents.labels(service_type='unknown').dec()

            # Publish event
            if self.kafka_producer:
                await self.kafka_producer.send(
                    f"{self.config.kafka_topic_prefix}.events",
                    {
                        "event": "agent_unregistered",
                        "agent_id": agent_id,
                        "timestamp": datetime.utcnow().isoformat()
                    }
                )

            logger.info(f"Agent unregistered: {agent_id}")

            return web.json_response({
                "status": "unregistered",
                "agent_id": agent_id,
                "message": "Agent successfully unregistered"
            })

        except Exception as e:
            logger.error(f"Error unregistering agent: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def agent_heartbeat(self, request: web.Request) -> web.Response:
        """Handle agent heartbeat"""
        agent_id = request.match_info['agent_id']

        try:
            data = await request.json()
            health_score = data.get('health_score', 1.0)
            metrics = data.get('metrics', {})

            # Update agent health
            await self.routing_engine.update_agent_health(agent_id, health_score)

            # Record metrics if provided
            if 'latency' in metrics:
                await self.routing_engine.record_route_metrics(
                    agent_id,
                    success=True,
                    latency=metrics['latency']
                )

            return web.json_response({
                "status": "ok",
                "timestamp": datetime.utcnow().isoformat()
            })

        except Exception as e:
            logger.error(f"Error processing heartbeat: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def list_agents(self, request: web.Request) -> web.Response:
        """List all registered agents"""
        try:
            stats = await self.routing_engine.get_routing_stats()

            agents = []
            for agent_id, route in self.routing_engine.routes.items():
                agents.append({
                    "agent_id": agent_id,
                    "service": route.agent.service,
                    "instance": route.agent.instance,
                    "capabilities": list(route.capabilities),
                    "health_score": route.health_score,
                    "is_healthy": route.is_healthy,
                    "current_load": route.current_load,
                    "max_concurrent": route.max_concurrent,
                    "metrics": {
                        "success_rate": route.metrics.success_rate,
                        "average_latency": route.metrics.average_latency,
                        "success_count": route.metrics.success_count,
                        "failure_count": route.metrics.failure_count
                    }
                })

            return web.json_response({
                "agents": agents,
                "total": len(agents),
                "healthy": stats["healthy_agents"],
                "unhealthy": stats["unhealthy_agents"]
            })

        except Exception as e:
            logger.error(f"Error listing agents: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def get_agent_info(self, request: web.Request) -> web.Response:
        """Get detailed information about specific agent"""
        agent_id = request.match_info['agent_id']

        if agent_id not in self.routing_engine.routes:
            return web.json_response(
                {"error": "Agent not found"},
                status=404
            )

        route = self.routing_engine.routes[agent_id]

        return web.json_response({
            "agent_id": agent_id,
            "service": route.agent.service,
            "instance": route.agent.instance,
            "version": route.agent.version,
            "endpoint": route.endpoint,
            "capabilities": list(route.capabilities),
            "health_score": route.health_score,
            "is_healthy": route.is_healthy,
            "current_load": route.current_load,
            "max_concurrent": route.max_concurrent,
            "last_heartbeat": route.last_heartbeat.isoformat(),
            "metrics": {
                "success_rate": route.metrics.success_rate,
                "average_latency": route.metrics.average_latency,
                "success_count": route.metrics.success_count,
                "failure_count": route.metrics.failure_count,
                "last_success": route.metrics.last_success.isoformat() if route.metrics.last_success else None,
                "last_failure": route.metrics.last_failure.isoformat() if route.metrics.last_failure else None
            }
        })

    async def create_workflow(self, request: web.Request) -> web.Response:
        """Create new workflow definition"""
        try:
            data = await request.json()
            # This would create workflow from definition
            # For now, return success
            return web.json_response({
                "workflow_id": data.get('workflow_id', 'test-workflow'),
                "status": "created"
            })
        except Exception as e:
            return web.json_response({"error": str(e)}, status=500)

    async def start_workflow(self, request: web.Request) -> web.Response:
        """Start workflow instance"""
        workflow_id = request.match_info['workflow_id']

        try:
            data = await request.json()
            instance_id = await self.workflow_engine.start_workflow(
                workflow_id,
                data.get('input', {})
            )

            return web.json_response({
                "instance_id": str(instance_id),
                "workflow_id": workflow_id,
                "status": "started"
            })

        except Exception as e:
            logger.error(f"Error starting workflow: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def get_workflow_status(self, request: web.Request) -> web.Response:
        """Get workflow instance status"""
        instance_id = UUID(request.match_info['instance_id'])

        try:
            status = await self.workflow_engine.get_workflow_status(instance_id)
            return web.json_response(status)

        except ValueError as e:
            return web.json_response(
                {"error": "Workflow instance not found"},
                status=404
            )
        except Exception as e:
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def cancel_workflow(self, request: web.Request) -> web.Response:
        """Cancel running workflow"""
        instance_id = UUID(request.match_info['instance_id'])

        try:
            await self.workflow_engine.cancel_workflow(instance_id)
            return web.json_response({
                "instance_id": str(instance_id),
                "status": "cancelled"
            })

        except Exception as e:
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def websocket_handler(self, request: web.Request) -> web.WebSocketResponse:
        """Handle WebSocket connections from agents"""
        ws = web.WebSocketResponse()
        await ws.prepare(request)

        agent_id = None

        try:
            # First message should be authentication
            async for msg in ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    data = json.loads(msg.data)

                    if data.get('type') == 'auth':
                        agent_id = data.get('agent_id')
                        if agent_id:
                            self.ws_connections[agent_id] = ws
                            await ws.send_json({
                                "type": "auth_success",
                                "agent_id": agent_id
                            })
                            logger.info(f"WebSocket authenticated: {agent_id}")
                        else:
                            await ws.send_json({
                                "type": "auth_error",
                                "error": "Missing agent_id"
                            })
                            break

                    elif data.get('type') == 'message':
                        # Handle incoming message from agent
                        if agent_id:
                            message = MAPMessage.from_json(json.dumps(data['payload']))
                            # Process message
                            asyncio.create_task(self._process_ws_message(message, agent_id))
                        else:
                            await ws.send_json({
                                "type": "error",
                                "error": "Not authenticated"
                            })

                    elif data.get('type') == 'heartbeat':
                        # Handle heartbeat
                        await ws.send_json({"type": "heartbeat_ack"})

                elif msg.type == aiohttp.WSMsgType.ERROR:
                    logger.error(f'WebSocket error: {ws.exception()}')

        except Exception as e:
            logger.error(f"WebSocket handler error: {str(e)}")

        finally:
            if agent_id and agent_id in self.ws_connections:
                del self.ws_connections[agent_id]

            return ws

    async def _process_ws_message(self, message: MAPMessage, sender_id: str):
        """Process message received via WebSocket"""
        try:
            # Set source if not provided
            if not message.header.source:
                if sender_id in self.routing_engine.routes:
                    route = self.routing_engine.routes[sender_id]
                    message.header.source = route.agent

            # Route and deliver message
            route = await self.routing_engine.route_message(message)
            if route:
                await self.transport_manager.send_message(message, route)

        except Exception as e:
            logger.error(f"Error processing WebSocket message: {str(e)}")

    async def health_check(self, request: web.Request) -> web.Response:
        """Health check endpoint"""
        return web.json_response({
            "status": "healthy",
            "timestamp": datetime.utcnow().isoformat(),
            "uptime": (datetime.utcnow() - self.start_time).total_seconds()
        })

    async def readiness_check(self, request: web.Request) -> web.Response:
        """Readiness check endpoint"""
        # Check if all components are ready
        ready = True
        components = {}

        # Check Kafka
        if self.config.kafka_brokers:
            components["kafka"] = self.kafka_producer is not None and self.kafka_consumer is not None
            ready &= components["kafka"]

        # Check routing engine
        stats = await self.routing_engine.get_routing_stats()
        components["routing"] = stats["total_agents"] > 0

        # Check workflow engine
        components["workflow"] = True  # Always ready for now

        if ready:
            return web.json_response({
                "status": "ready",
                "components": components
            })
        else:
            return web.json_response({
                "status": "not_ready",
                "components": components
            }, status=503)

    async def metrics_handler(self, request: web.Request) -> web.Response:
        """Prometheus metrics endpoint"""
        metrics = generate_latest()
        return web.Response(
            body=metrics,
            content_type="text/plain; version=0.0.4"
        )

    async def get_server_stats(self, request: web.Request) -> web.Response:
        """Get comprehensive server statistics"""
        try:
            routing_stats = await self.routing_engine.get_routing_stats()

            stats = {
                "server": {
                    "node_id": self.config.node_id,
                    "version": "1.0.0",
                    "uptime": (datetime.utcnow() - self.start_time).total_seconds(),
                    "start_time": self.start_time.isoformat()
                },
                "connections": {
                    "websocket": len(self.ws_connections),
                    "total_active": len(self.ws_connections)
                },
                "routing": routing_stats,
                "workflows": {
                    "active_instances": len(self.workflow_engine.instances),
                    "registered_definitions": len(self.workflow_engine.definitions)
                },
                "performance": {
                    "messages_per_second": 0,  # Would calculate from metrics
                    "average_latency_ms": 0  # Would calculate from metrics
                }
            }

            return web.json_response(stats)

        except Exception as e:
            logger.error(f"Error getting server stats: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def broadcast_message(self, request: web.Request) -> web.Response:
        """Broadcast message to all agents or specific group"""
        try:
            data = await request.json()

            # Create broadcast message
            message = MAPMessage(
                header=MessageHeader(
                    destination=MessageDestination(broadcast=True),
                    priority=MessagePriority(data.get('priority', 'medium'))
                ),
                payload=MessagePayload(
                    type=MessageType.EVENT,
                    action=data.get('action', 'broadcast'),
                    data=data.get('data', {}),
                    metadata=data.get('metadata', {})
                )
            )

            # Send to all connected WebSocket clients
            broadcast_tasks = []
            for agent_id, ws in self.ws_connections.items():
                if not ws.closed:
                    task = ws.send_json({
                        "type": "message",
                        "payload": json.loads(message.to_json())
                    })
                    broadcast_tasks.append(task)

            if broadcast_tasks:
                await asyncio.gather(*broadcast_tasks, return_exceptions=True)

            # Publish to Kafka for cluster-wide broadcast
            if self.kafka_producer:
                await self.kafka_producer.send(
                    f"{self.config.kafka_topic_prefix}.broadcast",
                    json.loads(message.to_json())
                )

            return web.json_response({
                "status": "broadcast_sent",
                "recipients": len(broadcast_tasks),
                "message_id": str(message.header.message_id)
            })

        except Exception as e:
            logger.error(f"Error broadcasting message: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def _process_kafka_message(self, data: Dict[str, Any]):
        """Process message received from Kafka"""
        try:
            # Skip if message originated from this node
            if data.get('node_id') == self.config.node_id:
                return

            message = MAPMessage.from_json(json.dumps(data['message']))

            # Check if we have the target agent
            if 'route' in data and data['route'] in self.routing_engine.routes:
                route = self.routing_engine.routes[data['route']]
                await self.transport_manager.send_message(message, route)

        except Exception as e:
            logger.error(f"Error processing Kafka message: {str(e)}")

    async def _process_kafka_event(self, data: Dict[str, Any]):
        """Process event received from Kafka"""
        try:
            event_type = data.get('event')

            if event_type == 'agent_registered':
                # Sync agent registration across cluster
                # Implementation depends on cluster coordination strategy
                pass

            elif event_type == 'agent_unregistered':
                # Sync agent removal across cluster
                agent_id = data.get('agent_id')
                if agent_id and agent_id in self.routing_engine.routes:
                    await self.routing_engine.unregister_agent(agent_id)

        except Exception as e:
            logger.error(f"Error processing Kafka event: {str(e)}")

    async def _start_metrics_server(self):
        """Start separate metrics server"""
        metrics_app = web.Application()
        metrics_app.router.add_get('/metrics', self.metrics_handler)

        runner = web.AppRunner(metrics_app)
        await runner.setup()
        site = web.TCPSite(runner, '0.0.0.0', self.config.metrics_port)
        await site.start()

        logger.info(f"Metrics server started on port {self.config.metrics_port}")

    def register_message_handler(self, action: str, handler: Callable):
        """Register custom message handler for specific action"""
        if action not in self.message_handlers:
            self.message_handlers[action] = []
        self.message_handlers[action].append(handler)

    def unregister_message_handler(self, action: str, handler: Callable):
        """Unregister message handler"""
        if action in self.message_handlers:
            self.message_handlers[action].remove(handler)

# File: maple/core/map/server/protocol_server.py
# Description: Main MAP Protocol Server that integrates all components and provides
# the primary service interface for the Multi-Agent Protocol. This server handles
# incoming messages, manages agent connections, and orchestrates all MAP operations.

from __future__ import annotations
import asyncio
import logging
import signal
import sys
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Set, Any, Callable
from uuid import UUID
import json
from aiohttp import web
import aiohttp_cors
from prometheus_client import Counter, Histogram, Gauge, generate_latest
import aiokafka
from dataclasses import dataclass, field

from maple.core.map.models.message import (
    MAPMessage, MessageType, MessagePriority,
    DeliveryMode, AgentIdentifier, MessageDestination
)
from maple.core.map.routing.engine import RoutingEngine, AgentRoute
from maple.core.map.transport.base import TransportManager, DeliveryReceipt
from maple.core.map.orchestration.workflow import WorkflowEngine

logger = logging.getLogger(__name__)

# Prometheus metrics
message_counter = Counter(
    'map_messages_total',
    'Total number of messages processed',
    ['message_type', 'priority', 'status']
)

message_duration = Histogram(
    'map_message_duration_seconds',
    'Time spent processing messages',
    ['message_type', 'priority']
)

active_agents = Gauge(
    'map_active_agents',
    'Number of active agents',
    ['service_type']
)

active_connections = Gauge(
    'map_active_connections',
    'Number of active connections',
    ['transport_type']
)


@dataclass
class ServerConfig:
    """MAP Protocol Server configuration"""
    host: str = "0.0.0.0"
    port: int = 8080
    kafka_brokers: List[str] = field(default_factory=lambda: ["localhost:9092"])
    kafka_topic_prefix: str = "maple.map"
    enable_metrics: bool = True
    metrics_port: int = 9090
    enable_auth: bool = True
    auth_secret: str = "change-me-in-production"
    max_message_size: int = 10 * 1024 * 1024  # 10MB
    request_timeout: int = 300  # 5 minutes
    cleanup_interval: int = 3600  # 1 hour
    enable_clustering: bool = False
    cluster_nodes: List[str] = field(default_factory=list)
    node_id: str = "map-node-1"
    ssl_cert: Optional[str] = None
    ssl_key: Optional[str] = None


class MAPProtocolServer:
    """Main MAP Protocol Server implementation"""

    def __init__(self, config: ServerConfig):
        self.config = config
        self.routing_engine = RoutingEngine()
        self.transport_manager = TransportManager()
        self.workflow_engine = WorkflowEngine(self.routing_engine, self.transport_manager)

        # HTTP app
        self.app = web.Application(
            client_max_size=config.max_message_size
        )
        self._setup_routes()
        self._setup_cors()

        # Kafka components
        self.kafka_producer: Optional[aiokafka.AIOKafkaProducer] = None
        self.kafka_consumer: Optional[aiokafka.AIOKafkaConsumer] = None

        # WebSocket connections
        self.ws_connections: Dict[str, web.WebSocketResponse] = {}

        # Message handlers
        self.message_handlers: Dict[str, List[Callable]] = {}

        # Background tasks
        self.background_tasks: List[asyncio.Task] = []

        # Server state
        self.is_running = False
        self.start_time = datetime.utcnow()

    def _setup_routes(self):
        """Configure HTTP routes"""
        # Message endpoints
        self.app.router.add_post('/api/v1/messages', self.handle_message)
        self.app.router.add_post('/api/v1/messages/batch', self.handle_batch_messages)
        self.app.router.add_get('/api/v1/messages/{message_id}', self.get_message_status)

        # Agent management
        self.app.router.add_post('/api/v1/agents/register', self.register_agent)
        self.app.router.add_delete('/api/v1/agents/{agent_id}', self.unregister_agent)
        self.app.router.add_post('/api/v1/agents/{agent_id}/heartbeat', self.agent_heartbeat)
        self.app.router.add_get('/api/v1/agents', self.list_agents)
        self.app.router.add_get('/api/v1/agents/{agent_id}', self.get_agent_info)

        # Workflow management
        self.app.router.add_post('/api/v1/workflows', self.create_workflow)
        self.app.router.add_post('/api/v1/workflows/{workflow_id}/start', self.start_workflow)
        self.app.router.add_get('/api/v1/workflows/{instance_id}/status', self.get_workflow_status)
        self.app.router.add_post('/api/v1/workflows/{instance_id}/cancel', self.cancel_workflow)

        # WebSocket endpoint
        self.app.router.add_get('/api/v1/ws', self.websocket_handler)

        # Health and metrics
        self.app.router.add_get('/health', self.health_check)
        self.app.router.add_get('/ready', self.readiness_check)
        if self.config.enable_metrics:
            self.app.router.add_get('/metrics', self.metrics_handler)

        # Admin endpoints
        self.app.router.add_get('/api/v1/admin/stats', self.get_server_stats)
        self.app.router.add_post('/api/v1/admin/broadcast', self.broadcast_message)

    def _setup_cors(self):
        """Configure CORS for the application"""
        cors = aiohttp_cors.setup(self.app, defaults={
            "*": aiohttp_cors.ResourceOptions(
                allow_credentials=True,
                expose_headers="*",
                allow_headers="*",
                allow_methods="*"
            )
        })

        for route in list(self.app.router.routes()):
            cors.add(route)

    async def start(self):
        """Start the MAP Protocol Server"""
        logger.info(f"Starting MAP Protocol Server on {self.config.host}:{self.config.port}")

        self.is_running = True

        # Initialize Kafka
        await self._init_kafka()

        # Start background tasks
        self._start_background_tasks()

        # Setup signal handlers
        for sig in (signal.SIGTERM, signal.SIGINT):
            signal.signal(sig, lambda s, f: asyncio.create_task(self.shutdown()))

        # Start metrics server if enabled
        if self.config.enable_metrics:
            asyncio.create_task(self._start_metrics_server())

        # Start main HTTP server
        runner = web.AppRunner(self.app)
        await runner.setup()

        if self.config.ssl_cert and self.config.ssl_key:
            import ssl
            ssl_context = ssl.create_default_context(ssl.Purpose.CLIENT_AUTH)
            ssl_context.load_cert_chain(self.config.ssl_cert, self.config.ssl_key)
            site = web.TCPSite(runner, self.config.host, self.config.port, ssl_context=ssl_context)
        else:
            site = web.TCPSite(runner, self.config.host, self.config.port)

        await site.start()

        logger.info(f"MAP Protocol Server started successfully")

        # Keep server running
        while self.is_running:
            await asyncio.sleep(1)

    async def shutdown(self):
        """Gracefully shutdown the server"""
        logger.info("Shutting down MAP Protocol Server...")

        self.is_running = False

        # Cancel background tasks
        for task in self.background_tasks:
            task.cancel()

        # Close WebSocket connections
        for ws in self.ws_connections.values():
            await ws.close()

        # Shutdown Kafka
        if self.kafka_producer:
            await self.kafka_producer.stop()
        if self.kafka_consumer:
            await self.kafka_consumer.stop()

        # Shutdown transport manager
        await self.transport_manager.close_all()

        # Shutdown workflow engine
        await self.workflow_engine.shutdown()

        logger.info("MAP Protocol Server shutdown complete")

    async def _init_kafka(self):
        """Initialize Kafka producer and consumer"""
        try:
            # Create producer
            self.kafka_producer = aiokafka.AIOKafkaProducer(
                bootstrap_servers=self.config.kafka_brokers,
                value_serializer=lambda v: json.dumps(v).encode('utf-8'),
                compression_type='gzip'
            )
            await self.kafka_producer.start()

            # Create consumer
            self.kafka_consumer = aiokafka.AIOKafkaConsumer(
                f"{self.config.kafka_topic_prefix}.messages",
                f"{self.config.kafka_topic_prefix}.events",
                bootstrap_servers=self.config.kafka_brokers,
                group_id=f"map-server-{self.config.node_id}",
                value_deserializer=lambda v: json.loads(v.decode('utf-8')),
                auto_offset_reset='latest'
            )
            await self.kafka_consumer.start()

            # Start consumer loop
            self.background_tasks.append(
                asyncio.create_task(self._kafka_consumer_loop())
            )

            logger.info("Kafka initialization complete")

        except Exception as e:
            logger.error(f"Failed to initialize Kafka: {str(e)}")
            # Continue without Kafka in development mode

    async def _kafka_consumer_loop(self):
        """Process messages from Kafka"""
        async for msg in self.kafka_consumer:
            try:
                if msg.topic.endswith('.messages'):
                    await self._process_kafka_message(msg.value)
                elif msg.topic.endswith('.events'):
                    await self._process_kafka_event(msg.value)
            except Exception as e:
                logger.error(f"Error processing Kafka message: {str(e)}")

    def _start_background_tasks(self):
        """Start background maintenance tasks"""
        # Cleanup task
        self.background_tasks.append(
            asyncio.create_task(self._cleanup_loop())
        )

        # Health check task
        self.background_tasks.append(
            asyncio.create_task(self._health_monitor_loop())
        )

        # Metrics collection
        if self.config.enable_metrics:
            self.background_tasks.append(
                asyncio.create_task(self._metrics_collection_loop())
            )

    async def _cleanup_loop(self):
        """Periodic cleanup of expired data"""
        while self.is_running:
            try:
                await asyncio.sleep(self.config.cleanup_interval)

                # Cleanup completed workflows
                await self.workflow_engine.cleanup_completed_instances(
                    timedelta(days=7)
                )

                # Cleanup stale WebSocket connections
                stale_connections = []
                for agent_id, ws in self.ws_connections.items():
                    if ws.closed:
                        stale_connections.append(agent_id)

                for agent_id in stale_connections:
                    del self.ws_connections[agent_id]

            except Exception as e:
                logger.error(f"Error in cleanup loop: {str(e)}")

    async def _health_monitor_loop(self):
        """Monitor health of connected agents"""
        while self.is_running:
            try:
                await asyncio.sleep(30)  # Check every 30 seconds

                # Get all registered agents
                stats = await self.routing_engine.get_routing_stats()

                # Update metrics
                for service, count in stats['service_distribution'].items():
                    active_agents.labels(service_type=service).set(count)

            except Exception as e:
                logger.error(f"Error in health monitor loop: {str(e)}")

    async def _metrics_collection_loop(self):
        """Collect and update metrics"""
        while self.is_running:
            try:
                await asyncio.sleep(60)  # Update every minute

                # Update connection metrics
                active_connections.labels(transport_type='websocket').set(
                    len(self.ws_connections)
                )

            except Exception as e:
                logger.error(f"Error in metrics collection: {str(e)}")

    # HTTP Handlers

    async def handle_message(self, request: web.Request) -> web.Response:
        """Handle incoming message"""
        start_time = datetime.utcnow()

        try:
            # Parse message
            data = await request.json()
            message = MAPMessage.from_json(json.dumps(data))

            # Validate message
            if message.is_expired():
                return web.json_response(
                    {"error": "Message expired"},
                    status=400
                )

            # Route message
            route = await self.routing_engine.route_message(message)
            if not route:
                return web.json_response(
                    {"error": "No route found"},
                    status=404
                )

            # Send message
            receipt = await self.transport_manager.send_message(message, route)

            # Update metrics
            duration = (datetime.utcnow() - start_time).total_seconds()
            message_counter.labels(
                message_type=message.payload.type.value,
                priority=message.header.priority.value,
                status='success'
            ).inc()
            message_duration.labels(
                message_type=message.payload.type.value,
                priority=message.header.priority.value
            ).observe(duration)

            # Publish to Kafka for clustering
            if self.config.enable_clustering and self.kafka_producer:
                await self.kafka_producer.send(
                    f"{self.config.kafka_topic_prefix}.messages",
                    {
                        "message": data,
                        "route": route.agent.agent_id,
                        "node_id": self.config.node_id,
                        "timestamp": datetime.utcnow().isoformat()
                    }
                )

            return web.json_response({
                "message_id": str(receipt.message_id),
                "status": receipt.status,
                "delivered_at": receipt.delivered_at.isoformat(),
                "recipient": receipt.recipient
            })

        except Exception as e:
            logger.error(f"Error handling message: {str(e)}")
            message_counter.labels(
                message_type='unknown',
                priority='medium',
                status='error'
            ).inc()
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def handle_batch_messages(self, request: web.Request) -> web.Response:
        """Handle batch of messages"""
        try:
            data = await request.json()
            messages = [MAPMessage.from_json(json.dumps(msg)) for msg in data]

            results = []
            for message in messages:
                try:
                    route = await self.routing_engine.route_message(message)
                    if route:
                        receipt = await self.transport_manager.send_message(message, route)
                        results.append({
                            "message_id": str(message.header.message_id),
                            "status": "delivered",
                            "recipient": route.agent.agent_id
                        })
                    else:
                        results.append({
                            "message_id": str(message.header.message_id),
                            "status": "no_route",
                            "error": "No route found"
                        })
                except Exception as e:
                    results.append({
                        "message_id": str(message.header.message_id),
                        "status": "error",
                        "error": str(e)
                    })

            return web.json_response({"results": results})

        except Exception as e:
            logger.error(f"Error handling batch messages: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )