# File: core/map/transport/base.py
# Description: Transport layer abstractions and implementations for MAP protocol.
# Supports multiple transport mechanisms including HTTP, gRPC, WebSocket, and message queues.
# This layer handles the actual delivery of messages between agents with reliability guarantees.

from __future__ import annotations
import asyncio
import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import datetime
from typing import Dict, Optional, Any, Callable, List
from uuid import UUID
import json
import aiohttp
import websockets
from concurrent.futures import ThreadPoolExecutor

from core.map.models.message import MAPMessage, DeliveryMode
from core.map.routing.engine import AgentRoute

logger = logging.getLogger(__name__)


class TransportError(Exception):
    """Base exception for transport layer errors"""
    pass


class DeliveryError(TransportError):
    """Error during message delivery"""
    pass


class TimeoutError(TransportError):
    """Message delivery timeout"""
    pass


@dataclass
class DeliveryReceipt:
    """Confirmation of message delivery"""
    message_id: UUID
    delivered_at: datetime
    recipient: str
    status: str = "delivered"
    metadata: Dict[str, Any] = None


class MessageTransport(ABC):
    """Abstract base class for message transport implementations"""

    @abstractmethod
    async def send(self, message: MAPMessage, route: AgentRoute) -> DeliveryReceipt:
        """Send a message to an agent"""
        pass

    @abstractmethod
    async def send_batch(self, messages: List[MAPMessage], route: AgentRoute) -> List[DeliveryReceipt]:
        """Send multiple messages to an agent"""
        pass

    @abstractmethod
    async def connect(self) -> None:
        """Establish transport connection"""
        pass

    @abstractmethod
    async def disconnect(self) -> None:
        """Close transport connection"""
        pass

    @abstractmethod
    def supports_streaming(self) -> bool:
        """Check if transport supports streaming"""
        pass


class HTTPTransport(MessageTransport):
    """HTTP-based message transport"""

    def __init__(self, timeout: int = 30, max_retries: int = 3):
        self.timeout = timeout
        self.max_retries = max_retries
        self.session: Optional[aiohttp.ClientSession] = None
        self._executor = ThreadPoolExecutor(max_workers=10)

    async def connect(self) -> None:
        """Create HTTP session"""
        if not self.session:
            connector = aiohttp.TCPConnector(limit=100, limit_per_host=30)
            timeout = aiohttp.ClientTimeout(total=self.timeout)
            self.session = aiohttp.ClientSession(
                connector=connector,
                timeout=timeout,
                headers={"User-Agent": "MAPLE/1.0"}
            )

    async def disconnect(self) -> None:
        """Close HTTP session"""
        if self.session:
            await self.session.close()
            self.session = None

    async def send(self, message: MAPMessage, route: AgentRoute) -> DeliveryReceipt:
        """Send message via HTTP POST"""
        if not self.session:
            await self.connect()

        url = f"{route.endpoint}/messages"
        headers = {
            "Content-Type": "application/json",
            "X-Message-ID": str(message.header.message_id),
            "X-Priority": message.header.priority.value
        }

        # Add authentication if present
        if message.security.auth_token:
            headers["Authorization"] = f"Bearer {message.security.auth_token}"

        attempts = 0
        last_error = None

        while attempts < self.max_retries:
            try:
                async with self.session.post(
                        url,
                        data=message.to_json(),
                        headers=headers
                ) as response:
                    if response.status == 200:
                        return DeliveryReceipt(
                            message_id=message.header.message_id,
                            delivered_at=datetime.utcnow(),
                            recipient=route.agent.agent_id,
                            status="delivered"
                        )
                    elif response.status == 429:  # Rate limited
                        retry_after = int(response.headers.get("Retry-After", "5"))
                        await asyncio.sleep(retry_after)
                    else:
                        error_text = await response.text()
                        raise DeliveryError(f"HTTP {response.status}: {error_text}")

            except asyncio.TimeoutError:
                last_error = TimeoutError(f"Timeout sending to {route.agent.agent_id}")
            except aiohttp.ClientError as e:
                last_error = DeliveryError(f"Network error: {str(e)}")

            attempts += 1
            if attempts < self.max_retries:
                await asyncio.sleep(2 ** attempts)  # Exponential backoff

        if last_error:
            raise last_error
        raise DeliveryError(f"Failed after {self.max_retries} attempts")

    async def send_batch(self, messages: List[MAPMessage], route: AgentRoute) -> List[DeliveryReceipt]:
        """Send multiple messages in a batch"""
        if not self.session:
            await self.connect()

        url = f"{route.endpoint}/messages/batch"
        batch_data = [msg.to_json() for msg in messages]

        headers = {
            "Content-Type": "application/json",
            "X-Batch-Size": str(len(messages))
        }

        async with self.session.post(url, json=batch_data, headers=headers) as response:
            if response.status == 200:
                receipts = []
                for msg in messages:
                    receipts.append(DeliveryReceipt(
                        message_id=msg.header.message_id,
                        delivered_at=datetime.utcnow(),
                        recipient=route.agent.agent_id,
                        status="delivered"
                    ))
                return receipts
            else:
                raise DeliveryError(f"Batch delivery failed: HTTP {response.status}")

    def supports_streaming(self) -> bool:
        return False


class WebSocketTransport(MessageTransport):
    """WebSocket-based message transport for real-time communication"""

    def __init__(self, heartbeat_interval: int = 30):
        self.heartbeat_interval = heartbeat_interval
        self.connections: Dict[str, websockets.WebSocketClientProtocol] = {}
        self._heartbeat_tasks: Dict[str, asyncio.Task] = {}
        self._receive_handlers: Dict[str, Callable] = {}

    async def connect(self) -> None:
        """WebSocket connections are established per route"""
        pass

    async def disconnect(self) -> None:
        """Close all WebSocket connections"""
        for conn_id in list(self.connections.keys()):
            await self._disconnect_route(conn_id)

    async def _ensure_connection(self, route: AgentRoute) -> websockets.WebSocketClientProtocol:
        """Ensure WebSocket connection exists for route"""
        conn_id = route.agent.agent_id

        if conn_id not in self.connections:
            ws_url = route.endpoint.replace("http://", "ws://").replace("https://", "wss://")
            ws_url = f"{ws_url}/ws"

            try:
                connection = await websockets.connect(ws_url)
                self.connections[conn_id] = connection

                # Start heartbeat
                self._heartbeat_tasks[conn_id] = asyncio.create_task(
                    self._heartbeat_loop(conn_id)
                )

                # Start receive loop
                asyncio.create_task(self._receive_loop(conn_id))

                logger.info(f"WebSocket connected to {route.agent.agent_id}")
            except Exception as e:
                raise DeliveryError(f"Failed to establish WebSocket: {str(e)}")

        return self.connections[conn_id]

    async def _disconnect_route(self, conn_id: str) -> None:
        """Disconnect specific route"""
        if conn_id in self._heartbeat_tasks:
            self._heartbeat_tasks[conn_id].cancel()
            del self._heartbeat_tasks[conn_id]

        if conn_id in self.connections:
            await self.connections[conn_id].close()
            del self.connections[conn_id]

    async def _heartbeat_loop(self, conn_id: str) -> None:
        """Send periodic heartbeat messages"""
        while conn_id in self.connections:
            try:
                connection = self.connections[conn_id]
                await connection.ping()
                await asyncio.sleep(self.heartbeat_interval)
            except Exception as e:
                logger.error(f"Heartbeat failed for {conn_id}: {str(e)}")
                await self._disconnect_route(conn_id)
                break

    async def _receive_loop(self, conn_id: str) -> None:
        """Handle incoming messages from WebSocket"""
        connection = self.connections.get(conn_id)
        if not connection:
            return

        try:
            async for message in connection:
                if conn_id in self._receive_handlers:
                    await self._receive_handlers[conn_id](message)
        except websockets.exceptions.ConnectionClosed:
            logger.info(f"WebSocket connection closed for {conn_id}")
        except Exception as e:
            logger.error(f"Error in receive loop for {conn_id}: {str(e)}")
        finally:
            await self._disconnect_route(conn_id)

    async def send(self, message: MAPMessage, route: AgentRoute) -> DeliveryReceipt:
        """Send message via WebSocket"""
        connection = await self._ensure_connection(route)

        try:
            await connection.send(message.to_json())

            return DeliveryReceipt(
                message_id=message.header.message_id,
                delivered_at=datetime.utcnow(),
                recipient=route.agent.agent_id,
                status="delivered"
            )
        except Exception as e:
            raise DeliveryError(f"WebSocket send failed: {str(e)}")

    async def send_batch(self, messages: List[MAPMessage], route: AgentRoute) -> List[DeliveryReceipt]:
        """Send multiple messages via WebSocket"""
        connection = await self._ensure_connection(route)
        receipts = []

        for msg in messages:
            try:
                await connection.send(msg.to_json())
                receipts.append(DeliveryReceipt(
                    message_id=msg.header.message_id,
                    delivered_at=datetime.utcnow(),
                    recipient=route.agent.agent_id,
                    status="delivered"
                ))
            except Exception as e:
                logger.error(f"Failed to send message {msg.header.message_id}: {str(e)}")
                receipts.append(DeliveryReceipt(
                    message_id=msg.header.message_id,
                    delivered_at=datetime.utcnow(),
                    recipient=route.agent.agent_id,
                    status="failed",
                    metadata={"error": str(e)}
                ))

        return receipts

    def supports_streaming(self) -> bool:
        return True

    def register_receive_handler(self, agent_id: str, handler: Callable) -> None:
        """Register handler for incoming messages from agent"""
        self._receive_handlers[agent_id] = handler


class TransportManager:
    """Manages multiple transport implementations"""

    def __init__(self):
        self.transports: Dict[str, MessageTransport] = {
            "http": HTTPTransport(),
            "websocket": WebSocketTransport()
        }
        self._default_transport = "http"

    async def send_message(self,
                           message: MAPMessage,
                           route: AgentRoute,
                           transport_hint: Optional[str] = None) -> DeliveryReceipt:
        """Send message using appropriate transport"""
        # Determine transport based on route endpoint or hint
        transport_type = transport_hint or self._detect_transport(route.endpoint)

        if transport_type not in self.transports:
            transport_type = self._default_transport

        transport = self.transports[transport_type]

        # Handle delivery mode
        if message.header.delivery_mode == DeliveryMode.EXACTLY_ONCE:
            return await self._send_exactly_once(message, route, transport)
        elif message.header.delivery_mode == DeliveryMode.AT_LEAST_ONCE:
            return await self._send_at_least_once(message, route, transport)
        else:  # AT_MOST_ONCE
            return await self._send_at_most_once(message, route, transport)

    async def _send_exactly_once(self,
                                 message: MAPMessage,
                                 route: AgentRoute,
                                 transport: MessageTransport) -> DeliveryReceipt:
        """Implement exactly-once delivery semantics"""
        # This would involve:
        # 1. Checking if message was already delivered (via deduplication store)
        # 2. Sending with transaction ID
        # 3. Waiting for acknowledgment
        # 4. Recording delivery in deduplication store

        # Simplified implementation for now
        return await transport.send(message, route)

    async def _send_at_least_once(self,
                                  message: MAPMessage,
                                  route: AgentRoute,
                                  transport: MessageTransport) -> DeliveryReceipt:
        """Implement at-least-once delivery with retries"""
        max_attempts = 3
        attempt = 0
        last_error = None

        while attempt < max_attempts:
            try:
                return await transport.send(message, route)
            except TransportError as e:
                last_error = e
                attempt += 1
                if attempt < max_attempts:
                    await asyncio.sleep(2 ** attempt)

        if last_error:
            raise last_error
        raise DeliveryError("Failed to deliver message")

    async def _send_at_most_once(self,
                                 message: MAPMessage,
                                 route: AgentRoute,
                                 transport: MessageTransport) -> DeliveryReceipt:
        """Implement at-most-once delivery (fire and forget)"""
        try:
            return await transport.send(message, route)
        except TransportError:
            # Log but don't retry
            logger.warning(f"Failed to deliver message {message.header.message_id}")
            return DeliveryReceipt(
                message_id=message.header.message_id,
                delivered_at=datetime.utcnow(),
                recipient=route.agent.agent_id,
                status="failed"
            )

    def _detect_transport(self, endpoint: str) -> str:
        """Detect transport type from endpoint URL"""
        if endpoint.startswith(("ws://", "wss://")):
            return "websocket"
        elif endpoint.startswith(("http://", "https://")):
            return "http"
        else:
            return self._default_transport

    async def close_all(self) -> None:
        """Close all transport connections"""
        for transport in self.transports.values():
            await transport.disconnect()