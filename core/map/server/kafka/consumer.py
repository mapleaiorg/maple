# File: maple/core/map/server/kafka/consumer.py
# Description: Kafka consumer implementation extracted from protocol_server.py
# Handles Kafka message consumption for cluster synchronization.

from __future__ import annotations
import asyncio
import json
import logging
from datetime import datetime
from typing import Dict, List, Optional, Callable, Any
from uuid import UUID
import aiokafka

from maple.core.map.models.message import MAPMessage
from maple.core.map.routing.engine import RoutingEngine
from maple.core.map.transport.base import TransportManager

logger = logging.getLogger(__name__)


class KafkaConsumerManager:
    """Manages Kafka consumer for MAP protocol clustering"""

    def __init__(self,
                 brokers: List[str],
                 topic_prefix: str,
                 node_id: str,
                 routing_engine: RoutingEngine,
                 transport_manager: TransportManager):
        self.brokers = brokers
        self.topic_prefix = topic_prefix
        self.node_id = node_id
        self.routing_engine = routing_engine
        self.transport_manager = transport_manager

        self.consumer: Optional[aiokafka.AIOKafkaConsumer] = None
        self.running = False
        self._consumer_task: Optional[asyncio.Task] = None

        # Message handlers for different topics
        self.topic_handlers = {
            f"{topic_prefix}.messages": self._handle_message_topic,
            f"{topic_prefix}.events": self._handle_event_topic,
            f"{topic_prefix}.broadcast": self._handle_broadcast_topic,
            f"{topic_prefix}.sync": self._handle_sync_topic
        }

    async def start(self):
        """Start Kafka consumer"""
        try:
            # Create consumer
            self.consumer = aiokafka.AIOKafkaConsumer(
                *self.topic_handlers.keys(),
                bootstrap_servers=self.brokers,
                group_id=f"map-server-{self.node_id}",
                value_deserializer=lambda v: json.loads(v.decode('utf-8')),
                auto_offset_reset='latest',
                enable_auto_commit=True
            )

            await self.consumer.start()
            self.running = True

            # Start consumer loop
            self._consumer_task = asyncio.create_task(self._consumer_loop())

            logger.info(f"Kafka consumer started for node {self.node_id}")

        except Exception as e:
            logger.error(f"Failed to start Kafka consumer: {str(e)}")
            raise

    async def stop(self):
        """Stop Kafka consumer"""
        self.running = False

        if self._consumer_task:
            self._consumer_task.cancel()
            try:
                await self._consumer_task
            except asyncio.CancelledError:
                pass

        if self.consumer:
            await self.consumer.stop()

        logger.info("Kafka consumer stopped")

    async def _consumer_loop(self):
        """Main consumer loop"""
        while self.running:
            try:
                # Consume messages with timeout
                async for msg in self.consumer:
                    if not self.running:
                        break

                    # Route to appropriate handler
                    handler = self.topic_handlers.get(msg.topic)
                    if handler:
                        try:
                            await handler(msg.value)
                        except Exception as e:
                            logger.error(f"Error processing Kafka message: {str(e)}")
                    else:
                        logger.warning(f"No handler for topic: {msg.topic}")

            except Exception as e:
                if self.running:
                    logger.error(f"Kafka consumer error: {str(e)}")
                    await asyncio.sleep(5)  # Brief pause before retrying

    async def _handle_message_topic(self, data: Dict[str, Any]):
        """Handle messages from the messages topic"""
        try:
            # Skip if message originated from this node
            if data.get('node_id') == self.node_id:
                return

            message_data = data.get('message')
            if not message_data:
                return

            # Recreate MAP message
            message = MAPMessage.from_json(json.dumps(message_data))

            # Check if we have the target agent
            route_id = data.get('route')
            if route_id and route_id in self.routing_engine.routes:
                route = self.routing_engine.routes[route_id]

                # Forward message to local agent
                await self.transport_manager.send_message(message, route)

                logger.debug(f"Forwarded message from node {data.get('node_id')} to agent {route_id}")

        except Exception as e:
            logger.error(f"Error handling message topic: {str(e)}")

    async def _handle_event_topic(self, data: Dict[str, Any]):
        """Handle events from the events topic"""
        try:
            # Skip if event originated from this node
            if data.get('node_id') == self.node_id:
                return

            event_type = data.get('event')

            if event_type == 'agent_registered':
                await self._handle_agent_registered(data)
            elif event_type == 'agent_unregistered':
                await self._handle_agent_unregistered(data)
            elif event_type == 'agent_health_update':
                await self._handle_agent_health_update(data)
            elif event_type == 'workflow_started':
                await self._handle_workflow_event(data)
            else:
                logger.debug(f"Unhandled event type: {event_type}")

        except Exception as e:
            logger.error(f"Error handling event topic: {str(e)}")

    async def _handle_broadcast_topic(self, data: Dict[str, Any]):
        """Handle broadcast messages"""
        try:
            # Skip if broadcast originated from this node
            if data.get('node_id') == self.node_id:
                return

            message_data = data.get('message', data)

            # Create MAP message
            message = MAPMessage.from_json(json.dumps(message_data))

            # Find all local agents matching broadcast criteria
            if message.header.destination.broadcast:
                # Send to all local agents
                for route in self.routing_engine.routes.values():
                    try:
                        await self.transport_manager.send_message(message, route)
                    except Exception as e:
                        logger.error(f"Failed to broadcast to agent {route.agent.agent_id}: {str(e)}")

            elif message.header.destination.multicast_groups:
                # Send to agents in specified groups
                # TODO: Implement group membership tracking
                pass

        except Exception as e:
            logger.error(f"Error handling broadcast topic: {str(e)}")

    async def _handle_sync_topic(self, data: Dict[str, Any]):
        """Handle cluster synchronization messages"""
        try:
            sync_type = data.get('sync_type')

            if sync_type == 'full_state':
                # Full state synchronization request
                # This would typically be used when a new node joins
                await self._handle_full_state_sync(data)
            elif sync_type == 'partial_update':
                # Partial state update
                await self._handle_partial_state_sync(data)

        except Exception as e:
            logger.error(f"Error handling sync topic: {str(e)}")

    async def _handle_agent_registered(self, data: Dict[str, Any]):
        """Handle agent registration event from another node"""
        agent_data = data.get('agent_data', {})

        # Extract agent information
        agent_id = agent_data.get('agent_id')
        if not agent_id:
            return

        # Create a proxy route for the remote agent
        from maple.core.map.models.message import AgentIdentifier
        from maple.core.map.routing.engine import AgentRoute

        proxy_route = AgentRoute(
            agent=AgentIdentifier(
                agent_id=agent_id,
                service=agent_data.get('service', 'unknown'),
                instance=f"{data.get('node_id')}:{agent_id}"
            ),
            endpoint=f"kafka://{data.get('node_id')}/{agent_id}",
            capabilities=set(agent_data.get('capabilities', [])),
            health_score=agent_data.get('health_score', 1.0)
        )

        # Register the proxy route
        self.routing_engine.routes[agent_id] = proxy_route

        logger.info(f"Registered proxy route for remote agent {agent_id} on node {data.get('node_id')}")

    async def _handle_agent_unregistered(self, data: Dict[str, Any]):
        """Handle agent unregistration event from another node"""
        agent_id = data.get('agent_id')
        if agent_id and agent_id in self.routing_engine.routes:
            # Only remove if it's a proxy route
            route = self.routing_engine.routes[agent_id]
            if route.endpoint.startswith('kafka://'):
                del self.routing_engine.routes[agent_id]
                logger.info(f"Removed proxy route for remote agent {agent_id}")

    async def _handle_agent_health_update(self, data: Dict[str, Any]):
        """Handle agent health update from another node"""
        agent_id = data.get('agent_id')
        health_score = data.get('health_score', 1.0)

        if agent_id and agent_id in self.routing_engine.routes:
            route = self.routing_engine.routes[agent_id]
            route.health_score = health_score

    async def _handle_workflow_event(self, data: Dict[str, Any]):
        """Handle workflow-related events"""
        # This would typically update any shared workflow state
        # or coordinate distributed workflow execution
        pass

    async def _handle_full_state_sync(self, data: Dict[str, Any]):
        """Handle full state synchronization request"""
        requesting_node = data.get('requesting_node')

        if requesting_node == self.node_id:
            return

        # Prepare state snapshot
        state_snapshot = {
            'node_id': self.node_id,
            'timestamp': datetime.utcnow().isoformat(),
            'agents': [
                {
                    'agent_id': route.agent.agent_id,
                    'service': route.agent.service,
                    'capabilities': list(route.capabilities),
                    'health_score': route.health_score
                }
                for route in self.routing_engine.routes.values()
                if not route.endpoint.startswith('kafka://')  # Only local agents
            ]
        }

        # Publish state snapshot
        # This would use the Kafka producer
        logger.info(f"Sending state snapshot to node {requesting_node}")

    async def _handle_partial_state_sync(self, data: Dict[str, Any]):
        """Handle partial state update"""
        # Update specific parts of the cluster state
        pass


class KafkaProducerManager:
    """Manages Kafka producer for MAP protocol clustering"""

    def __init__(self,
                 brokers: List[str],
                 topic_prefix: str,
                 node_id: str):
        self.brokers = brokers
        self.topic_prefix = topic_prefix
        self.node_id = node_id
        self.producer: Optional[aiokafka.AIOKafkaProducer] = None

    async def start(self):
        """Start Kafka producer"""
        try:
            self.producer = aiokafka.AIOKafkaProducer(
                bootstrap_servers=self.brokers,
                value_serializer=lambda v: json.dumps(v).encode('utf-8'),
                compression_type='gzip',
                acks='all'
            )

            await self.producer.start()
            logger.info(f"Kafka producer started for node {self.node_id}")

        except Exception as e:
            logger.error(f"Failed to start Kafka producer: {str(e)}")
            raise

    async def stop(self):
        """Stop Kafka producer"""
        if self.producer:
            await self.producer.stop()
            logger.info("Kafka producer stopped")

    async def publish_message(self, message_data: Dict[str, Any], route_id: str):
        """Publish message to cluster"""
        if not self.producer:
            return

        try:
            await self.producer.send(
                f"{self.topic_prefix}.messages",
                {
                    "message": message_data,
                    "route": route_id,
                    "node_id": self.node_id,
                    "timestamp": datetime.utcnow().isoformat()
                }
            )
        except Exception as e:
            logger.error(f"Failed to publish message: {str(e)}")

    async def publish_event(self, event_type: str, event_data: Dict[str, Any]):
        """Publish event to cluster"""
        if not self.producer:
            return

        try:
            await self.producer.send(
                f"{self.topic_prefix}.events",
                {
                    "event": event_type,
                    "node_id": self.node_id,
                    "timestamp": datetime.utcnow().isoformat(),
                    **event_data
                }
            )
        except Exception as e:
            logger.error(f"Failed to publish event: {str(e)}")

    async def publish_broadcast(self, message_data: Dict[str, Any]):
        """Publish broadcast message to cluster"""
        if not self.producer:
            return

        try:
            await self.producer.send(
                f"{self.topic_prefix}.broadcast",
                {
                    "message": message_data,
                    "node_id": self.node_id,
                    "timestamp": datetime.utcnow().isoformat()
                }
            )
        except Exception as e:
            logger.error(f"Failed to publish broadcast: {str(e)}")

    async def request_state_sync(self):
        """Request full state synchronization from cluster"""
        if not self.producer:
            return

        try:
            await self.producer.send(
                f"{self.topic_prefix}.sync",
                {
                    "sync_type": "full_state",
                    "requesting_node": self.node_id,
                    "timestamp": datetime.utcnow().isoformat()
                }
            )
        except Exception as e:
            logger.error(f"Failed to request state sync: {str(e)}")