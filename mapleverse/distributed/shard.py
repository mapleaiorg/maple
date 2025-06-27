# File: mapleverse/distributed/shard.py
# Description: Distributed simulation sharding for planetary-scale Mapleverse

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Set, Tuple
import asyncio
import hashlib
import logging
from enum import Enum
import json
import time

from core.mapleverse.engine.base import (
    SimulationEngine, SimulationWorld, Entity, AgentEntity, Vector3
)
from core.map.models.message import MAPMessage, MessageType
from core.map.transport.base import TransportProtocol

logger = logging.getLogger(__name__)


class ShardState(Enum):
    """Shard lifecycle states"""
    INITIALIZING = "initializing"
    ACTIVE = "active"
    SYNCING = "syncing"
    MIGRATING = "migrating"
    DRAINING = "draining"
    STOPPED = "stopped"


@dataclass
class ShardBounds:
    """Spatial bounds for a shard"""
    min_bound: Vector3
    max_bound: Vector3

    def contains(self, position: Vector3) -> bool:
        """Check if position is within bounds"""
        return (
                self.min_bound.x <= position.x <= self.max_bound.x and
                self.min_bound.y <= position.y <= self.max_bound.y and
                self.min_bound.z <= position.z <= self.max_bound.z
        )

    def overlaps(self, other: 'ShardBounds') -> bool:
        """Check if bounds overlap with another"""
        return not (
                self.max_bound.x < other.min_bound.x or
                self.min_bound.x > other.max_bound.x or
                self.max_bound.y < other.min_bound.y or
                self.min_bound.y > other.max_bound.y or
                self.max_bound.z < other.min_bound.z or
                self.min_bound.z > other.max_bound.z
        )


@dataclass
class ShardMetrics:
    """Performance metrics for a shard"""
    entity_count: int = 0
    agent_count: int = 0
    message_throughput: float = 0.0
    sync_latency: float = 0.0
    cpu_usage: float = 0.0
    memory_usage: float = 0.0
    last_update: float = field(default_factory=time.time)

    def update(self, world: SimulationWorld):
        """Update metrics from world state"""
        self.entity_count = len(world.entities)
        self.agent_count = len(world.agents)
        self.last_update = time.time()


class SimulationShard:
    """A shard managing a portion of the simulation world"""

    def __init__(self, shard_id: str, bounds: ShardBounds,
                 engine: SimulationEngine, transport: TransportProtocol):
        self.id = shard_id
        self.bounds = bounds
        self.engine = engine
        self.transport = transport
        self.state = ShardState.INITIALIZING

        # Neighboring shards for edge synchronization
        self.neighbors: Set[str] = set()

        # Entities near shard boundaries
        self.boundary_entities: Dict[str, Entity] = {}
        self.ghost_entities: Dict[str, Entity] = {}  # Replicas from neighbors

        # Synchronization state
        self.sync_queue: asyncio.Queue = asyncio.Queue()
        self.pending_migrations: Dict[str, Tuple[Entity, str]] = {}

        # Metrics
        self.metrics = ShardMetrics()

        # Holographic state compression
        self.holographic_encoder = HolographicEncoder()

    async def initialize(self):
        """Initialize the shard"""
        logger.info(f"Initializing shard {self.id}")

        # Register message handlers
        await self._register_handlers()

        # Start synchronization tasks
        asyncio.create_task(self._sync_loop())
        asyncio.create_task(self._boundary_check_loop())

        self.state = ShardState.ACTIVE

    async def _register_handlers(self):
        """Register MAP message handlers"""
        handlers = {
            "shard.sync": self._handle_sync_message,
            "shard.migrate": self._handle_migrate_message,
            "shard.ghost_update": self._handle_ghost_update,
            "shard.neighbor_announce": self._handle_neighbor_announce
        }

        for msg_type, handler in handlers.items():
            self.transport.register_handler(msg_type, handler)

    def add_neighbor(self, neighbor_id: str):
        """Add a neighboring shard"""
        self.neighbors.add(neighbor_id)
        logger.debug(f"Shard {self.id} added neighbor {neighbor_id}")

    def is_entity_local(self, entity: Entity) -> bool:
        """Check if entity belongs to this shard"""
        return self.bounds.contains(entity.transform.position)

    def is_near_boundary(self, entity: Entity, threshold: float = 10.0) -> bool:
        """Check if entity is near shard boundary"""
        pos = entity.transform.position
        bounds = self.bounds

        return (
                abs(pos.x - bounds.min_bound.x) < threshold or
                abs(pos.x - bounds.max_bound.x) < threshold or
                abs(pos.y - bounds.min_bound.y) < threshold or
                abs(pos.y - bounds.max_bound.y) < threshold or
                abs(pos.z - bounds.min_bound.z) < threshold or
                abs(pos.z - bounds.max_bound.z) < threshold
        )

    async def update(self, delta_time: float):
        """Update shard simulation state"""
        if self.state != ShardState.ACTIVE:
            return

        # Update local entities
        await self.engine.update(delta_time)

        # Update metrics
        self.metrics.update(self.engine.world)

        # Process pending migrations
        await self._process_migrations()

    async def _sync_loop(self):
        """Main synchronization loop"""
        while self.state in [ShardState.ACTIVE, ShardState.SYNCING]:
            try:
                # Process sync queue
                while not self.sync_queue.empty():
                    sync_data = await self.sync_queue.get()
                    await self._process_sync(sync_data)

                # Send periodic sync to neighbors
                if self.state == ShardState.ACTIVE:
                    await self._broadcast_sync()

                await asyncio.sleep(0.1)  # 100ms sync interval

            except Exception as e:
                logger.error(f"Sync error in shard {self.id}: {e}")

    async def _boundary_check_loop(self):
        """Check for entities crossing boundaries"""
        while self.state == ShardState.ACTIVE:
            try:
                boundary_entities = {}
                migrations = []

                for entity_id, entity in self.engine.world.entities.items():
                    # Check if entity is still in bounds
                    if not self.is_entity_local(entity):
                        # Entity needs migration
                        target_shard = await self._find_target_shard(entity.transform.position)
                        if target_shard:
                            migrations.append((entity, target_shard))

                    # Check if near boundary
                    elif self.is_near_boundary(entity):
                        boundary_entities[entity_id] = entity

                # Update boundary entities
                self.boundary_entities = boundary_entities

                # Queue migrations
                for entity, target_shard in migrations:
                    self.pending_migrations[entity.id] = (entity, target_shard)

                await asyncio.sleep(0.05)  # 50ms check interval

            except Exception as e:
                logger.error(f"Boundary check error in shard {self.id}: {e}")

    async def _process_migrations(self):
        """Process pending entity migrations"""
        if not self.pending_migrations:
            return

        migrations = list(self.pending_migrations.items())
        self.pending_migrations.clear()

        for entity_id, (entity, target_shard) in migrations:
            try:
                # Serialize entity state
                entity_data = self._serialize_entity(entity)

                # Send migration message
                message = MAPMessage(
                    type=MessageType.REQUEST,
                    destination=f"shard:{target_shard}",
                    payload={
                        "type": "shard.migrate",
                        "entity_data": entity_data,
                        "source_shard": self.id
                    }
                )

                await self.transport.send(message)

                # Remove from local world
                self.engine.world.remove_entity(entity_id)

                logger.debug(f"Migrated entity {entity_id} from {self.id} to {target_shard}")

            except Exception as e:
                logger.error(f"Migration error for entity {entity_id}: {e}")
                # Re-add to pending if migration failed
                self.pending_migrations[entity_id] = (entity, target_shard)

    async def _broadcast_sync(self):
        """Broadcast synchronization data to neighbors"""
        if not self.boundary_entities:
            return

        # Compress boundary entity states using holographic encoding
        sync_data = {
            "shard_id": self.id,
            "timestamp": time.time(),
            "entities": {}
        }

        for entity_id, entity in self.boundary_entities.items():
            # Use holographic compression for efficient sync
            compressed_state = self.holographic_encoder.compress(entity)
            sync_data["entities"][entity_id] = compressed_state

        # Send to all neighbors
        for neighbor_id in self.neighbors:
            message = MAPMessage(
                type=MessageType.BROADCAST,
                destination=f"shard:{neighbor_id}",
                payload={
                    "type": "shard.ghost_update",
                    "data": sync_data
                }
            )
            await self.transport.send(message)

    async def _handle_sync_message(self, message: MAPMessage):
        """Handle incoming sync message"""
        await self.sync_queue.put(message.payload.get("data"))

    async def _handle_migrate_message(self, message: MAPMessage):
        """Handle entity migration request"""
        try:
            entity_data = message.payload.get("entity_data")
            source_shard = message.payload.get("source_shard")

            # Deserialize and add entity
            entity = self._deserialize_entity(entity_data)
            self.engine.world.add_entity(entity)

            logger.debug(f"Received migrated entity {entity.id} from {source_shard}")

            # Send acknowledgment
            ack_message = MAPMessage(
                type=MessageType.RESPONSE,
                destination=f"shard:{source_shard}",
                payload={
                    "type": "shard.migrate_ack",
                    "entity_id": entity.id,
                    "success": True
                }
            )
            await self.transport.send(ack_message)

        except Exception as e:
            logger.error(f"Failed to handle migration: {e}")

    async def _handle_ghost_update(self, message: MAPMessage):
        """Handle ghost entity updates from neighbors"""
        sync_data = message.payload.get("data", {})
        source_shard = sync_data.get("shard_id")

        if source_shard not in self.neighbors:
            return

        # Update ghost entities
        for entity_id, compressed_state in sync_data.get("entities", {}).items():
            try:
                # Decompress holographic state
                entity_state = self.holographic_encoder.decompress(compressed_state)

                # Update or create ghost entity
                if entity_id in self.ghost_entities:
                    self._update_ghost_entity(self.ghost_entities[entity_id], entity_state)
                else:
                    ghost = self._create_ghost_entity(entity_id, entity_state)
                    self.ghost_entities[entity_id] = ghost

            except Exception as e:
                logger.error(f"Failed to update ghost entity {entity_id}: {e}")

    async def _handle_neighbor_announce(self, message: MAPMessage):
        """Handle neighbor announcement"""
        neighbor_id = message.payload.get("shard_id")
        neighbor_bounds = message.payload.get("bounds")

        if neighbor_id and neighbor_id != self.id:
            # Check if actually neighboring based on bounds
            if neighbor_bounds:
                bounds = ShardBounds(
                    Vector3(**neighbor_bounds["min"]),
                    Vector3(**neighbor_bounds["max"])
                )

                # Add as neighbor if bounds are adjacent
                if self._are_adjacent(self.bounds, bounds):
                    self.add_neighbor(neighbor_id)

    def _are_adjacent(self, bounds1: ShardBounds, bounds2: ShardBounds,
                      tolerance: float = 1.0) -> bool:
        """Check if two bounds are adjacent"""
        # Check each dimension for adjacency
        x_adjacent = (
                abs(bounds1.max_bound.x - bounds2.min_bound.x) < tolerance or
                abs(bounds2.max_bound.x - bounds1.min_bound.x) < tolerance
        )
        y_adjacent = (
                abs(bounds1.max_bound.y - bounds2.min_bound.y) < tolerance or
                abs(bounds2.max_bound.y - bounds1.min_bound.y) < tolerance
        )
        z_adjacent = (
                abs(bounds1.max_bound.z - bounds2.min_bound.z) < tolerance or
                abs(bounds2.max_bound.z - bounds1.min_bound.z) < tolerance
        )

        # Adjacent if touching in one dimension and overlapping in others
        return (
                (x_adjacent and self._overlaps_2d(bounds1, bounds2, 'yz')) or
                (y_adjacent and self._overlaps_2d(bounds1, bounds2, 'xz')) or
                (z_adjacent and self._overlaps_2d(bounds1, bounds2, 'xy'))
        )

    def _overlaps_2d(self, bounds1: ShardBounds, bounds2: ShardBounds,
                     dimensions: str) -> bool:
        """Check if bounds overlap in 2D plane"""
        if 'x' in dimensions and 'y' in dimensions:
            return not (
                    bounds1.max_bound.x < bounds2.min_bound.x or
                    bounds1.min_bound.x > bounds2.max_bound.x or
                    bounds1.max_bound.y < bounds2.min_bound.y or
                    bounds1.min_bound.y > bounds2.max_bound.y
            )
        # Similar for other dimension pairs...
        return True

    async def _find_target_shard(self, position: Vector3) -> Optional[str]:
        """Find which shard should own a position"""
        # In a real implementation, this would query a shard registry
        # For now, return None (would need shard coordinator)
        return None

    def _serialize_entity(self, entity: Entity) -> Dict[str, Any]:
        """Serialize entity for migration"""
        data = entity.to_dict()

        # Add type information
        data["_type"] = type(entity).__name__

        # Add components
        if hasattr(entity, 'components'):
            data["components"] = {}
            for name, component in entity.components.items():
                if hasattr(component, 'to_dict'):
                    data["components"][name] = component.to_dict()
                else:
                    data["components"][name] = component

        # Add agent-specific data
        if isinstance(entity, AgentEntity):
            data["agent_did"] = entity.agent_did
            data["capabilities"] = entity.capabilities
            data["resources"] = {
                name: {
                    "amount": res.amount,
                    "max_amount": res.max_amount,
                    "regeneration_rate": res.regeneration_rate
                }
                for name, res in entity.resources.items()
            }
            data["performance_metrics"] = entity.performance_metrics

        return data

    def _deserialize_entity(self, data: Dict[str, Any]) -> Entity:
        """Deserialize entity from migration data"""
        entity_type = data.get("_type", "Entity")

        if entity_type == "AgentEntity":
            entity = AgentEntity(
                data["id"],
                data.get("agent_did", "unknown"),
                data.get("capabilities", [])
            )

            # Restore resources
            for res_name, res_data in data.get("resources", {}).items():
                entity.add_resource(
                    res_name,
                    res_data["amount"],
                    res_data["max_amount"],
                    res_data.get("regeneration_rate", 0.0)
                )

            # Restore metrics
            entity.performance_metrics.update(data.get("performance_metrics", {}))
        else:
            # Generic entity
            entity = Entity(data["id"], data["name"])

        # Restore transform
        transform_data = data.get("transform", {})
        entity.transform.position = Vector3(**transform_data.get("position", {}))
        entity.transform.rotation = Vector3(**transform_data.get("rotation", {}))
        entity.transform.scale = Vector3(**transform_data.get("scale", {}))

        # Restore physics
        physics_data = data.get("physics", {})
        entity.physics.mass = physics_data.get("mass", 1.0)
        entity.physics.is_static = physics_data.get("is_static", False)

        # Restore components
        for name, comp_data in data.get("components", {}).items():
            entity.add_component(name, comp_data)

        return entity

    def _create_ghost_entity(self, entity_id: str, state: Dict[str, Any]) -> Entity:
        """Create a ghost entity from state"""
        # Ghost entities are read-only replicas
        ghost = Entity(f"ghost_{entity_id}", f"Ghost of {entity_id}")
        ghost.transform.position = Vector3(**state.get("position", {}))
        ghost.physics.is_static = True  # Ghosts don't participate in physics
        return ghost

    def _update_ghost_entity(self, ghost: Entity, state: Dict[str, Any]):
        """Update ghost entity state"""
        if "position" in state:
            ghost.transform.position = Vector3(**state["position"])

    class HolographicEncoder:
        """Holographic state compression using VAE-inspired techniques"""

        def __init__(self, latent_dim: int = 32):
            self.latent_dim = latent_dim

        def compress(self, entity: Entity) -> Dict[str, Any]:
            """Compress entity state into holographic representation"""
            # Extract key features
            features = {
                "position": entity.transform.position.to_dict(),
                "velocity": None,  # Would extract from components
                "type": type(entity).__name__
            }

            # Add agent-specific features
            if isinstance(entity, AgentEntity):
                features["agent_id"] = entity.agent_did
                features["resource_levels"] = {
                    name: res.amount / res.max_amount
                    for name, res in entity.resources.items()
                }

            # In a real implementation, this would use a trained VAE
            # For now, return compressed features
            return {
                "id": entity.id,
                "features": features,
                "timestamp": time.time()
            }

        def decompress(self, compressed: Dict[str, Any]) -> Dict[str, Any]:
            """Decompress holographic representation to state"""
            features = compressed.get("features", {})

            # Reconstruct state
            state = {
                "position": features.get("position", {"x": 0, "y": 0, "z": 0}),
                "type": features.get("type", "Entity")
            }

            # Add agent features if present
            if "agent_id" in features:
                state["agent_id"] = features["agent_id"]
                state["resources"] = features.get("resource_levels", {})

            return state

    class ShardCoordinator:
        """Coordinates multiple shards in a distributed simulation"""

        def __init__(self, simulation_id: str):
            self.simulation_id = simulation_id
            self.shards: Dict[str, SimulationShard] = {}
            self.shard_topology: Dict[str, Set[str]] = {}  # Neighbor relationships
            self.load_balancer = ShardLoadBalancer()

        async def create_shard_topology(self, world_bounds: Tuple[Vector3, Vector3],
                                        shard_count: Tuple[int, int, int]):
            """Create a grid topology of shards"""
            min_bound, max_bound = world_bounds
            x_shards, y_shards, z_shards = shard_count

            # Calculate shard dimensions
            shard_width = (max_bound.x - min_bound.x) / x_shards
            shard_height = (max_bound.y - min_bound.y) / y_shards
            shard_depth = (max_bound.z - min_bound.z) / z_shards

            # Create shards in a grid
            for x in range(x_shards):
                for y in range(y_shards):
                    for z in range(z_shards):
                        shard_id = f"shard_{x}_{y}_{z}"

                        # Calculate bounds
                        bounds = ShardBounds(
                            Vector3(
                                min_bound.x + x * shard_width,
                                min_bound.y + y * shard_height,
                                min_bound.z + z * shard_depth
                            ),
                            Vector3(
                                min_bound.x + (x + 1) * shard_width,
                                min_bound.y + (y + 1) * shard_height,
                                min_bound.z + (z + 1) * shard_depth
                            )
                        )

                        # Create shard (would need engine and transport instances)
                        # shard = SimulationShard(shard_id, bounds, engine, transport)
                        # self.shards[shard_id] = shard

                        # Calculate neighbors
                        neighbors = set()
                        for dx in [-1, 0, 1]:
                            for dy in [-1, 0, 1]:
                                for dz in [-1, 0, 1]:
                                    if dx == 0 and dy == 0 and dz == 0:
                                        continue

                                    nx, ny, nz = x + dx, y + dy, z + dz
                                    if (0 <= nx < x_shards and
                                            0 <= ny < y_shards and
                                            0 <= nz < z_shards):
                                        neighbor_id = f"shard_{nx}_{ny}_{nz}"
                                        neighbors.add(neighbor_id)

                        self.shard_topology[shard_id] = neighbors

            logger.info(f"Created {len(self.shard_topology)} shards in {shard_count} grid")

        async def rebalance_shards(self):
            """Rebalance load across shards"""
            shard_loads = {}

            for shard_id, shard in self.shards.items():
                shard_loads[shard_id] = self.load_balancer.calculate_load(shard.metrics)

            # Find overloaded and underloaded shards
            avg_load = sum(shard_loads.values()) / len(shard_loads)
            overloaded = [s for s, load in shard_loads.items() if load > avg_load * 1.5]
            underloaded = [s for s, load in shard_loads.items() if load < avg_load * 0.5]

            # Initiate migrations from overloaded to underloaded
            for source in overloaded:
                if not underloaded:
                    break

                target = underloaded.pop(0)
                await self._initiate_shard_split(source, target)

        async def _initiate_shard_split(self, source_id: str, target_id: str):
            """Split load from source shard to target"""
            logger.info(f"Initiating shard split from {source_id} to {target_id}")
            # Implementation would handle entity redistribution

    class ShardLoadBalancer:
        """Load balancing for simulation shards"""

        def calculate_load(self, metrics: ShardMetrics) -> float:
            """Calculate normalized load score for a shard"""
            # Weighted combination of metrics
            weights = {
                "entities": 0.3,
                "agents": 0.4,
                "cpu": 0.2,
                "memory": 0.1
            }

            # Normalize metrics (would need baseline values)
            normalized_entities = min(metrics.entity_count / 1000.0, 1.0)
            normalized_agents = min(metrics.agent_count / 100.0, 1.0)

            load = (
                    weights["entities"] * normalized_entities +
                    weights["agents"] * normalized_agents +
                    weights["cpu"] * metrics.cpu_usage +
                    weights["memory"] * metrics.memory_usage
            )

            return load