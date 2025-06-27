# File: maple/core/map/server/websocket/manager.py
# Description: WebSocket connection management extracted from protocol_server.py
# Handles WebSocket connections, message broadcasting, and connection lifecycle.

from __future__ import annotations
import asyncio
import json
import logging
from datetime import datetime
from typing import Dict, Set, Optional, Any, List
from uuid import UUID
from aiohttp import web, WSMsgType
import weakref

from maple.core.map.models.message import MAPMessage
from maple.core.map.routing.engine import RoutingEngine, AgentRoute

logger = logging.getLogger(__name__)


class WebSocketConnection:
    """Represents a single WebSocket connection"""

    def __init__(self, ws: web.WebSocketResponse, agent_id: str, metadata: Dict[str, Any] = None):
        self.ws = ws
        self.agent_id = agent_id
        self.metadata = metadata or {}
        self.connected_at = datetime.utcnow()
        self.last_ping = datetime.utcnow()
        self.message_count = 0

    async def send_message(self, message: Dict[str, Any]) -> bool:
        """Send message to this connection"""
        try:
            if not self.ws.closed:
                await self.ws.send_json(message)
                self.message_count += 1
                return True
        except Exception as e:
            logger.error(f"Failed to send message to {self.agent_id}: {str(e)}")
        return False

    async def send_ping(self) -> bool:
        """Send ping to check connection health"""
        try:
            if not self.ws.closed:
                await self.ws.ping()
                self.last_ping = datetime.utcnow()
                return True
        except Exception as e:
            logger.error(f"Failed to ping {self.agent_id}: {str(e)}")
        return False

    def is_alive(self) -> bool:
        """Check if connection is still alive"""
        return not self.ws.closed

    async def close(self, code: int = 1000, message: str = "Connection closed"):
        """Close the WebSocket connection"""
        if not self.ws.closed:
            await self.ws.close(code=code, message=message.encode())


class WebSocketManager:
    """Manages all WebSocket connections"""

    def __init__(self, routing_engine: RoutingEngine, config: Dict[str, Any] = None):
        self.routing_engine = routing_engine
        self.config = config or {}
        self.connections: Dict[str, WebSocketConnection] = {}
        self.agent_connections: Dict[str, Set[str]] = {}  # agent_id -> set of connection_ids
        self._cleanup_task: Optional[asyncio.Task] = None
        self._ping_task: Optional[asyncio.Task] = None

    async def start(self):
        """Start background tasks"""
        self._cleanup_task = asyncio.create_task(self._cleanup_loop())
        self._ping_task = asyncio.create_task(self._ping_loop())

    async def stop(self):
        """Stop background tasks and close all connections"""
        if self._cleanup_task:
            self._cleanup_task.cancel()
        if self._ping_task:
            self._ping_task.cancel()

        # Close all connections
        close_tasks = [conn.close() for conn in self.connections.values()]
        if close_tasks:
            await asyncio.gather(*close_tasks, return_exceptions=True)

        self.connections.clear()
        self.agent_connections.clear()

    async def handle_connection(self, request: web.Request) -> web.WebSocketResponse:
        """Handle new WebSocket connection"""
        ws = web.WebSocketResponse()
        await ws.prepare(request)

        agent_id = None
        connection_id = f"ws_{id(ws)}_{datetime.utcnow().timestamp()}"

        try:
            # Wait for authentication message
            auth_msg = await asyncio.wait_for(ws.receive(), timeout=10.0)

            if auth_msg.type == WSMsgType.TEXT:
                auth_data = json.loads(auth_msg.data)

                # Validate authentication
                agent_id = auth_data.get('agent_id')
                auth_token = auth_data.get('auth_token')

                if not agent_id:
                    await ws.close(code=4001, message=b'Missing agent_id')
                    return ws

                # TODO: Validate auth_token

                # Create connection
                connection = WebSocketConnection(ws, agent_id, auth_data.get('metadata'))
                self.connections[connection_id] = connection

                # Track agent connections
                if agent_id not in self.agent_connections:
                    self.agent_connections[agent_id] = set()
                self.agent_connections[agent_id].add(connection_id)

                # Send acknowledgment
                await ws.send_json({
                    'type': 'connected',
                    'connection_id': connection_id,
                    'agent_id': agent_id,
                    'timestamp': datetime.utcnow().isoformat()
                })

                logger.info(f"WebSocket connected: {agent_id} ({connection_id})")

                # Register agent route for WebSocket transport
                await self._register_websocket_route(agent_id, connection_id)

                # Handle messages
                async for msg in ws:
                    if msg.type == WSMsgType.TEXT:
                        await self._handle_message(agent_id, connection_id, json.loads(msg.data))
                    elif msg.type == WSMsgType.ERROR:
                        logger.error(f'WebSocket error: {ws.exception()}')
                        break

            else:
                await ws.close(code=4002, message=b'Expected text message')

        except asyncio.TimeoutError:
            await ws.close(code=4003, message=b'Authentication timeout')
        except Exception as e:
            logger.error(f"WebSocket error: {str(e)}")
            await ws.close(code=4000, message=str(e).encode())
        finally:
            # Cleanup connection
            if connection_id in self.connections:
                del self.connections[connection_id]

            if agent_id and agent_id in self.agent_connections:
                self.agent_connections[agent_id].discard(connection_id)
                if not self.agent_connections[agent_id]:
                    del self.agent_connections[agent_id]

                # Unregister route
                await self._unregister_websocket_route(agent_id)

            logger.info(f"WebSocket disconnected: {agent_id} ({connection_id})")

        return ws

    async def send_to_agent(self, agent_id: str, message: Dict[str, Any]) -> int:
        """Send message to all connections for an agent"""
        if agent_id not in self.agent_connections:
            return 0

        sent_count = 0
        failed_connections = []

        for conn_id in self.agent_connections[agent_id]:
            if conn_id in self.connections:
                success = await self.connections[conn_id].send_message(message)
                if success:
                    sent_count += 1
                else:
                    failed_connections.append(conn_id)

        # Remove failed connections
        for conn_id in failed_connections:
            self.agent_connections[agent_id].discard(conn_id)
            if conn_id in self.connections:
                del self.connections[conn_id]

        return sent_count

    async def broadcast(self, message: Dict[str, Any],
                        filter_func: Optional[callable] = None) -> int:
        """Broadcast message to multiple agents"""
        sent_count = 0

        for connection in list(self.connections.values()):
            if filter_func and not filter_func(connection):
                continue

            if await connection.send_message(message):
                sent_count += 1

        return sent_count

    async def _handle_message(self, agent_id: str, connection_id: str, data: Dict[str, Any]):
        """Handle incoming WebSocket message"""
        msg_type = data.get('type')

        if msg_type == 'ping':
            # Respond to ping
            await self.connections[connection_id].send_message({
                'type': 'pong',
                'timestamp': datetime.utcnow().isoformat()
            })

        elif msg_type == 'message':
            # Handle MAP message
            # This would typically forward to the message handler
            pass

        elif msg_type == 'subscribe':
            # Handle topic subscription
            topics = data.get('topics', [])
            # TODO: Implement topic subscription logic

        elif msg_type == 'unsubscribe':
            # Handle topic unsubscription
            topics = data.get('topics', [])
            # TODO: Implement topic unsubscription logic

        else:
            logger.warning(f"Unknown WebSocket message type: {msg_type}")

    async def _register_websocket_route(self, agent_id: str, connection_id: str):
        """Register WebSocket route with routing engine"""
        try:
            route = AgentRoute(
                agent=AgentIdentifier(
                    agent_id=agent_id,
                    service="websocket",
                    instance=connection_id
                ),
                endpoint=f"ws://{connection_id}",
                capabilities=set(),  # Will be updated based on agent registration
                max_concurrent=100
            )

            await self.routing_engine.register_agent(agent_id, route)

        except Exception as e:
            logger.error(f"Failed to register WebSocket route: {str(e)}")

    async def _unregister_websocket_route(self, agent_id: str):
        """Unregister WebSocket route from routing engine"""
        try:
            await self.routing_engine.unregister_agent(agent_id)
        except Exception as e:
            logger.error(f"Failed to unregister WebSocket route: {str(e)}")

    async def _cleanup_loop(self):
        """Periodically clean up dead connections"""
        while True:
            try:
                await asyncio.sleep(60)  # Check every minute

                dead_connections = []
                for conn_id, connection in self.connections.items():
                    if not connection.is_alive():
                        dead_connections.append(conn_id)

                # Remove dead connections
                for conn_id in dead_connections:
                    if conn_id in self.connections:
                        connection = self.connections[conn_id]
                        agent_id = connection.agent_id

                        del self.connections[conn_id]

                        if agent_id in self.agent_connections:
                            self.agent_connections[agent_id].discard(conn_id)
                            if not self.agent_connections[agent_id]:
                                del self.agent_connections[agent_id]
                                await self._unregister_websocket_route(agent_id)

                if dead_connections:
                    logger.info(f"Cleaned up {len(dead_connections)} dead connections")

            except Exception as e:
                logger.error(f"Error in cleanup loop: {str(e)}")

    async def _ping_loop(self):
        """Periodically ping connections to keep them alive"""
        while True:
            try:
                await asyncio.sleep(30)  # Ping every 30 seconds

                ping_tasks = []
                for connection in self.connections.values():
                    ping_tasks.append(connection.send_ping())

                if ping_tasks:
                    results = await asyncio.gather(*ping_tasks, return_exceptions=True)
                    failed_count = sum(1 for r in results if r is False or isinstance(r, Exception))

                    if failed_count > 0:
                        logger.warning(f"Failed to ping {failed_count} connections")

            except Exception as e:
                logger.error(f"Error in ping loop: {str(e)}")

    def get_connection_stats(self) -> Dict[str, Any]:
        """Get WebSocket connection statistics"""
        total_connections = len(self.connections)
        unique_agents = len(self.agent_connections)

        connections_per_agent = {
            agent_id: len(conns)
            for agent_id, conns in self.agent_connections.items()
        }

        avg_connections = sum(connections_per_agent.values()) / unique_agents if unique_agents > 0 else 0

        return {
            "total_connections": total_connections,
            "unique_agents": unique_agents,
            "average_connections_per_agent": avg_connections,
            "max_connections_per_agent": max(connections_per_agent.values()) if connections_per_agent else 0,
            "connections_by_agent": connections_per_agent
        }


# WebSocket message handlers
class WebSocketMessageHandler:
    """Handles different types of WebSocket messages"""

    def __init__(self, manager: WebSocketManager, routing_engine: RoutingEngine):
        self.manager = manager
        self.routing_engine = routing_engine
        self.handlers = {
            'message': self.handle_map_message,
            'subscribe': self.handle_subscribe,
            'unsubscribe': self.handle_unsubscribe,
            'heartbeat': self.handle_heartbeat,
            'status': self.handle_status_request
        }

    async def handle(self, agent_id: str, connection_id: str, data: Dict[str, Any]):
        """Route message to appropriate handler"""
        msg_type = data.get('type')
        handler = self.handlers.get(msg_type)

        if handler:
            await handler(agent_id, connection_id, data)
        else:
            logger.warning(f"Unknown message type: {msg_type}")

    async def handle_map_message(self, agent_id: str, connection_id: str, data: Dict[str, Any]):
        """Handle MAP protocol message via WebSocket"""
        try:
            message_data = data.get('message')
            if not message_data:
                return

            # Create MAP message
            message = MAPMessage.from_json(json.dumps(message_data))

            # Route message
            route = await self.routing_engine.route_message(message)
            if route:
                # If route is WebSocket, use WebSocket manager
                if route.agent.service == "websocket":
                    await self.manager.send_to_agent(
                        route.agent.agent_id,
                        {
                            'type': 'message',
                            'payload': message_data
                        }
                    )
                else:
                    # Use transport manager for other protocols
                    # This would be injected/passed to the handler
                    pass

        except Exception as e:
            logger.error(f"Error handling MAP message: {str(e)}")

    async def handle_subscribe(self, agent_id: str, connection_id: str, data: Dict[str, Any]):
        """Handle topic subscription"""
        topics = data.get('topics', [])

        # TODO: Implement topic subscription logic
        # This would typically update a subscription registry

        await self.manager.connections[connection_id].send_message({
            'type': 'subscribed',
            'topics': topics,
            'timestamp': datetime.utcnow().isoformat()
        })

    async def handle_unsubscribe(self, agent_id: str, connection_id: str, data: Dict[str, Any]):
        """Handle topic unsubscription"""
        topics = data.get('topics', [])

        # TODO: Implement topic unsubscription logic

        await self.manager.connections[connection_id].send_message({
            'type': 'unsubscribed',
            'topics': topics,
            'timestamp': datetime.utcnow().isoformat()
        })

    async def handle_heartbeat(self, agent_id: str, connection_id: str, data: Dict[str, Any]):
        """Handle heartbeat message"""
        if connection_id in self.manager.connections:
            connection = self.manager.connections[connection_id]
            connection.last_ping = datetime.utcnow()

            # Update routing engine with health status
            await self.routing_engine.update_agent_health(agent_id, 1.0)

    async def handle_status_request(self, agent_id: str, connection_id: str, data: Dict[str, Any]):
        """Handle status request"""
        if connection_id in self.manager.connections:
            connection = self.manager.connections[connection_id]

            await connection.send_message({
                'type': 'status',
                'connection_id': connection_id,
                'agent_id': agent_id,
                'connected_since': connection.connected_at.isoformat(),
                'message_count': connection.message_count,
                'last_ping': connection.last_ping.isoformat()
            })


def create_websocket_endpoint(manager: WebSocketManager):
    """Create WebSocket endpoint handler"""

    async def websocket_handler(request: web.Request):
        return await manager.handle_connection(request)

    return websocket_handler