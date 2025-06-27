# File: maple/core/map/server/handlers/message.py
# Description: Message handling endpoints extracted from protocol_server.py
# Handles all message-related HTTP endpoints and business logic.

from __future__ import annotations
import json
import logging
from datetime import datetime
from typing import Dict, Optional, List, Any
from uuid import UUID
from aiohttp import web
from prometheus_client import Counter, Histogram

from maple.core.map.models.message import (
    MAPMessage, MessageType, MessagePriority,
    DeliveryMode, MessageDestination
)
from maple.core.map.routing.engine import RoutingEngine
from maple.core.map.transport.base import TransportManager

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


class MessageHandler:
    """Handles all message-related endpoints"""

    def __init__(self,
                 routing_engine: RoutingEngine,
                 transport_manager: TransportManager,
                 kafka_producer: Optional[Any] = None,
                 config: Optional[Dict[str, Any]] = None):
        self.routing_engine = routing_engine
        self.transport_manager = transport_manager
        self.kafka_producer = kafka_producer
        self.config = config or {}

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
            if self.config.get('enable_clustering') and self.kafka_producer:
                await self._publish_to_kafka(message, route, data)

            return web.json_response({
                "status": "success",
                "message_id": str(message.header.message_id),
                "receipt": receipt.to_dict()
            })

        except json.JSONDecodeError:
            return web.json_response(
                {"error": "Invalid JSON"},
                status=400
            )
        except Exception as e:
            logger.error(f"Error handling message: {str(e)}")

            # Update error metrics
            message_counter.labels(
                message_type='unknown',
                priority='unknown',
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
            messages_data = data.get('messages', [])

            if not messages_data:
                return web.json_response(
                    {"error": "No messages provided"},
                    status=400
                )

            # Process messages concurrently
            results = []
            errors = []

            async def process_single_message(msg_data: Dict[str, Any], index: int):
                try:
                    message = MAPMessage.from_json(json.dumps(msg_data))

                    if message.is_expired():
                        errors.append({
                            "index": index,
                            "error": "Message expired",
                            "message_id": str(message.header.message_id)
                        })
                        return

                    route = await self.routing_engine.route_message(message)
                    if not route:
                        errors.append({
                            "index": index,
                            "error": "No route found",
                            "message_id": str(message.header.message_id)
                        })
                        return

                    receipt = await self.transport_manager.send_message(message, route)
                    results.append({
                        "index": index,
                        "message_id": str(message.header.message_id),
                        "receipt": receipt.to_dict()
                    })

                except Exception as e:
                    errors.append({
                        "index": index,
                        "error": str(e)
                    })

            # Process all messages
            import asyncio
            await asyncio.gather(*[
                process_single_message(msg_data, i)
                for i, msg_data in enumerate(messages_data)
            ])

            return web.json_response({
                "status": "batch_processed",
                "total": len(messages_data),
                "successful": len(results),
                "failed": len(errors),
                "results": results,
                "errors": errors
            })

        except Exception as e:
            logger.error(f"Error handling batch messages: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def get_message_status(self, request: web.Request) -> web.Response:
        """Get message delivery status"""
        try:
            message_id = request.match_info['message_id']

            # Check transport manager for delivery status
            status = await self.transport_manager.get_delivery_status(UUID(message_id))

            if not status:
                return web.json_response(
                    {"error": "Message not found"},
                    status=404
                )

            return web.json_response({
                "message_id": message_id,
                "status": status.get('status', 'unknown'),
                "delivered_at": status.get('delivered_at'),
                "recipient": status.get('recipient'),
                "metadata": status.get('metadata', {})
            })

        except ValueError:
            return web.json_response(
                {"error": "Invalid message ID format"},
                status=400
            )
        except Exception as e:
            logger.error(f"Error getting message status: {str(e)}")
            return web.json_response(
                {"error": str(e)},
                status=500
            )

    async def _publish_to_kafka(self, message: MAPMessage, route: Any, original_data: Dict[str, Any]):
        """Publish message to Kafka for cluster synchronization"""
        try:
            await self.kafka_producer.send(
                f"{self.config.get('kafka_topic_prefix', 'maple.map')}.messages",
                {
                    "message": original_data,
                    "route": route.agent.agent_id,
                    "node_id": self.config.get('node_id', 'unknown'),
                    "timestamp": datetime.utcnow().isoformat()
                }
            )
        except Exception as e:
            logger.error(f"Failed to publish to Kafka: {str(e)}")
            # Don't fail the request if Kafka publish fails


def register_routes(app: web.Application, handler: MessageHandler):
    """Register message routes with the application"""
    app.router.add_post('/api/v1/messages', handler.handle_message)
    app.router.add_post('/api/v1/messages/batch', handler.handle_batch_messages)
    app.router.add_get('/api/v1/messages/{message_id}', handler.get_message_status)