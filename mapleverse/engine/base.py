# File: maple/mapleverse/engine/base.py
# Description: Core simulation engine for the Mapleverse distributed physics platform

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Set, Tuple, Callable
from enum import Enum
import asyncio
import time
import uuid
import logging
import numpy as np
from abc import ABC, abstractmethod
import json

logger = logging.getLogger(__name__)


class PhysicsEngine(Enum):
    """Supported physics engines"""
    BULLET = "bullet"
    CUSTOM = "custom"
    SIMPLE = "simple"  # For lightweight simulations


class SimulationState(Enum):
    """Simulation lifecycle states"""
    CREATED = "created"
    INITIALIZING = "initializing"
    RUNNING = "running"
    PAUSED = "paused"
    COMPLETED = "completed"
    FAILED = "failed"


@dataclass
class Vector3:
    """3D vector for physics calculations"""
    x: float = 0.0
    y: float = 0.0
    z: float = 0.0

    def __add__(self, other: Vector3) -> Vector3:
        return Vector3(self.x + other.x, self.y + other.y, self.z + other.z)

    def __sub__(self, other: Vector3) -> Vector3:
        return Vector3(self.x - other.x, self.y - other.y, self.z - other.z)

    def __mul__(self, scalar: float) -> Vector3:
        return Vector3(self.x * scalar, self.y * scalar, self.z * scalar)

    def magnitude(self) -> float:
        return np.sqrt(self.x ** 2 + self.y ** 2 + self.z ** 2)

    def normalize(self) -> Vector3:
        mag = self.magnitude()
        if mag > 0:
            return Vector3(self.x / mag, self.y / mag, self.z / mag)
        return Vector3(0, 0, 0)

    def to_dict(self) -> Dict[str, float]:
        return {"x": self.x, "y": self.y, "z": self.z}


@dataclass
class Transform:
    """Entity transform in 3D space"""
    position: Vector3 = field(default_factory=Vector3)
    rotation: Vector3 = field(default_factory=Vector3)  # Euler angles
    scale: Vector3 = field(default_factory=lambda: Vector3(1, 1, 1))


@dataclass
class PhysicsProperties:
    """Physical properties for simulation entities"""
    mass: float = 1.0
    friction: float = 0.5
    restitution: float = 0.3  # Bounciness
    is_static: bool = False
    gravity_scale: float = 1.0
    linear_damping: float = 0.1
    angular_damping: float = 0.1


@dataclass
class Resource:
    """Resource that can be consumed/produced in simulations"""
    resource_type: str
    amount: float
    max_amount: float
    regeneration_rate: float = 0.0

    def consume(self, amount: float) -> float:
        """Consume resource, returns actual amount consumed"""
        consumed = min(amount, self.amount)
        self.amount -= consumed
        return consumed

    def produce(self, amount: float) -> float:
        """Produce resource, returns actual amount produced"""
        produced = min(amount, self.max_amount - self.amount)
        self.amount += produced
        return produced

    def regenerate(self, delta_time: float):
        """Regenerate resource over time"""
        if self.regeneration_rate > 0:
            self.produce(self.regeneration_rate * delta_time)


class Entity(ABC):
    """Base class for all simulation entities"""

    def __init__(self, entity_id: str, name: str):
        self.id = entity_id
        self.name = name
        self.transform = Transform()
        self.physics = PhysicsProperties()
        self.components: Dict[str, Any] = {}
        self.active = True

    @abstractmethod
    def update(self, delta_time: float):
        """Update entity state"""
        pass

    def add_component(self, name: str, component: Any):
        """Add a component to the entity"""
        self.components[name] = component

    def get_component(self, name: str) -> Optional[Any]:
        """Get a component by name"""
        return self.components.get(name)

    def to_dict(self) -> Dict[str, Any]:
        """Serialize entity to dictionary"""
        return {
            "id": self.id,
            "name": self.name,
            "transform": {
                "position": self.transform.position.to_dict(),
                "rotation": self.transform.rotation.to_dict(),
                "scale": self.transform.scale.to_dict()
            },
            "physics": {
                "mass": self.physics.mass,
                "is_static": self.physics.is_static
            },
            "active": self.active
        }


class AgentEntity(Entity):
    """Entity representing an AI agent in the simulation"""

    def __init__(self, entity_id: str, agent_did: str, capabilities: List[str]):
        super().__init__(entity_id, f"Agent_{agent_did}")
        self.agent_did = agent_did
        self.capabilities = capabilities
        self.resources: Dict[str, Resource] = {}
        self.performance_metrics = {
            "tasks_completed": 0,
            "resources_collected": 0,
            "distance_traveled": 0.0,
            "interactions": 0
        }
        self.behavior_state = {}

    def update(self, delta_time: float):
        """Update agent state"""
        # Regenerate resources
        for resource in self.resources.values():
            resource.regenerate(delta_time)

    def add_resource(self, resource_type: str, initial_amount: float,
                     max_amount: float, regeneration_rate: float = 0.0):
        """Add a resource to the agent"""
        self.resources[resource_type] = Resource(
            resource_type, initial_amount, max_amount, regeneration_rate
        )

    def can_perform_action(self, action: str, required_resources: Dict[str, float]) -> bool:
        """Check if agent has resources to perform action"""
        if action not in self.capabilities:
            return False

        for resource_type, amount in required_resources.items():
            if resource_type not in self.resources:
                return False
            if self.resources[resource_type].amount < amount:
                return False

        return True

    def perform_action(self, action: str, required_resources: Dict[str, float]) -> bool:
        """Perform an action, consuming required resources"""
        if not self.can_perform_action(action, required_resources):
            return False

        # Consume resources
        for resource_type, amount in required_resources.items():
            self.resources[resource_type].consume(amount)

        self.performance_metrics["tasks_completed"] += 1
        return True


@dataclass
class SimulationConfig:
    """Configuration for a simulation scenario"""
    scenario_name: str
    physics_engine: PhysicsEngine = PhysicsEngine.SIMPLE
    world_bounds: Tuple[Vector3, Vector3] = field(
        default_factory=lambda: (Vector3(-100, -100, -100), Vector3(100, 100, 100))
    )
    gravity: Vector3 = field(default_factory=lambda: Vector3(0, -9.81, 0))
    time_scale: float = 1.0
    max_agents: int = 1000
    max_entities: int = 10000
    tick_rate: int = 60  # Simulation updates per second
    custom_rules: Dict[str, Any] = field(default_factory=dict)
    resource_types: List[str] = field(default_factory=list)

    def validate(self) -> List[str]:
        """Validate configuration"""
        errors = []
        if self.tick_rate <= 0:
            errors.append("Tick rate must be positive")
        if self.max_agents <= 0:
            errors.append("Max agents must be positive")
        if self.time_scale <= 0:
            errors.append("Time scale must be positive")
        return errors


class SimulationWorld:
    """The simulation world containing all entities and physics"""

    def __init__(self, config: SimulationConfig):
        self.config = config
        self.entities: Dict[str, Entity] = {}
        self.agents: Dict[str, AgentEntity] = {}
        self.spatial_index: Dict[Tuple[int, int, int], Set[str]] = {}
        self.time = 0.0
        self.tick_count = 0
        self.events: List[Dict[str, Any]] = []

    def add_entity(self, entity: Entity):
        """Add an entity to the world"""
        if len(self.entities) >= self.config.max_entities:
            raise ValueError("Maximum entity limit reached")

        self.entities[entity.id] = entity

        if isinstance(entity, AgentEntity):
            if len(self.agents) >= self.config.max_agents:
                raise ValueError("Maximum agent limit reached")
            self.agents[entity.id] = entity

        self._update_spatial_index(entity)

    def remove_entity(self, entity_id: str):
        """Remove an entity from the world"""
        if entity_id in self.entities:
            entity = self.entities[entity_id]
            self._remove_from_spatial_index(entity)
            del self.entities[entity_id]

            if entity_id in self.agents:
                del self.agents[entity_id]

    def get_nearby_entities(self, position: Vector3, radius: float) -> List[Entity]:
        """Get entities within radius of position"""
        nearby = []
        # Simple distance check (can be optimized with spatial indexing)
        for entity in self.entities.values():
            distance = (entity.transform.position - position).magnitude()
            if distance <= radius:
                nearby.append(entity)
        return nearby

    def _update_spatial_index(self, entity: Entity):
        """Update spatial index for entity"""
        # Simple grid-based spatial indexing
        grid_pos = self._world_to_grid(entity.transform.position)
        if grid_pos not in self.spatial_index:
            self.spatial_index[grid_pos] = set()
        self.spatial_index[grid_pos].add(entity.id)

    def _remove_from_spatial_index(self, entity: Entity):
        """Remove entity from spatial index"""
        grid_pos = self._world_to_grid(entity.transform.position)
        if grid_pos in self.spatial_index:
            self.spatial_index[grid_pos].discard(entity.id)
            if not self.spatial_index[grid_pos]:
                del self.spatial_index[grid_pos]

    def _world_to_grid(self, position: Vector3) -> Tuple[int, int, int]:
        """Convert world position to grid coordinates"""
        grid_size = 10.0  # 10 units per grid cell
        return (
            int(position.x / grid_size),
            int(position.y / grid_size),
            int(position.z / grid_size)
        )

    def add_event(self, event_type: str, data: Dict[str, Any]):
        """Add an event to the simulation log"""
        self.events.append({
            "type": event_type,
            "time": self.time,
            "tick": self.tick_count,
            "data": data
        })

    def update(self, delta_time: float):
        """Update world state"""
        self.time += delta_time
        self.tick_count += 1

        # Update all entities
        for entity in list(self.entities.values()):
            if entity.active:
                old_pos = entity.transform.position
                entity.update(delta_time)

                # Update spatial index if position changed
                if entity.transform.position != old_pos:
                    self._remove_from_spatial_index(entity)
                    entity.transform.position = old_pos  # Temporary
                    self._update_spatial_index(entity)
                    entity.transform.position = entity.transform.position  # Restore

                    # Track distance for agents
                    if isinstance(entity, AgentEntity):
                        distance = (entity.transform.position - old_pos).magnitude()
                        entity.performance_metrics["distance_traveled"] += distance

    def get_state_snapshot(self) -> Dict[str, Any]:
        """Get current world state snapshot"""
        return {
            "time": self.time,
            "tick": self.tick_count,
            "entity_count": len(self.entities),
            "agent_count": len(self.agents),
            "entities": {
                entity_id: entity.to_dict()
                for entity_id, entity in self.entities.items()
            }
        }


class SimulationEngine:
    """Main simulation engine orchestrating the virtual world"""

    def __init__(self, config: SimulationConfig):
        self.config = config
        self.world = SimulationWorld(config)
        self.state = SimulationState.CREATED
        self.start_time = None
        self.end_time = None
        self.tick_interval = 1.0 / config.tick_rate
        self.callbacks: Dict[str, List[Callable]] = {
            "on_start": [],
            "on_tick": [],
            "on_end": [],
            "on_agent_spawn": [],
            "on_agent_action": [],
            "on_resource_change": []
        }
        self.performance_data = []

    def add_callback(self, event: str, callback: Callable):
        """Add a callback for simulation events"""
        if event in self.callbacks:
            self.callbacks[event].append(callback)

    async def initialize(self):
        """Initialize simulation"""
        logger.info(f"Initializing simulation: {self.config.scenario_name}")
        self.state = SimulationState.INITIALIZING

        # Validate configuration
        errors = self.config.validate()
        if errors:
            self.state = SimulationState.FAILED
            raise ValueError(f"Invalid configuration: {errors}")

        # Initialize physics engine
        await self._initialize_physics()

        # Run initialization callbacks
        for callback in self.callbacks["on_start"]:
            await callback(self)

        self.state = SimulationState.RUNNING
        self.start_time = time.time()
        logger.info("Simulation initialized successfully")

    async def _initialize_physics(self):
        """Initialize the physics engine"""
        # This would integrate with actual physics engines like Bullet
        # For now, using simple custom physics
        logger.debug(f"Initialized {self.config.physics_engine.value} physics engine")

    def spawn_agent(self, agent_did: str, capabilities: List[str],
                    position: Optional[Vector3] = None) -> AgentEntity:
        """Spawn a new agent in the simulation"""
        if self.state != SimulationState.RUNNING:
            raise RuntimeError("Cannot spawn agent - simulation not running")

        entity_id = f"agent_{uuid.uuid4().hex[:8]}"
        agent = AgentEntity(entity_id, agent_did, capabilities)

        # Set initial position
        if position:
            agent.transform.position = position
        else:
            # Random position within world bounds
            bounds_min, bounds_max = self.config.world_bounds
            agent.transform.position = Vector3(
                np.random.uniform(bounds_min.x, bounds_max.x),
                0,  # Start on ground
                np.random.uniform(bounds_min.z, bounds_max.z)
            )

        # Add default resources based on config
        for resource_type in self.config.resource_types:
            agent.add_resource(resource_type, 100.0, 1000.0, 1.0)

        self.world.add_entity(agent)

        # Trigger callbacks
        for callback in self.callbacks["on_agent_spawn"]:
            asyncio.create_task(callback(agent))

        logger.debug(f"Spawned agent {agent_did} at position {agent.transform.position.to_dict()}")
        return agent

    async def run(self, duration: Optional[float] = None):
        """Run the simulation"""
        if self.state != SimulationState.RUNNING:
            await self.initialize()

        logger.info(f"Starting simulation run (duration: {duration or 'unlimited'}s)")
        last_tick = time.time()

        while self.state == SimulationState.RUNNING:
            current_time = time.time()
            delta_time = current_time - last_tick

            if delta_time >= self.tick_interval:
                # Update world
                scaled_delta = delta_time * self.config.time_scale
                self.world.update(scaled_delta)

                # Run tick callbacks
                for callback in self.callbacks["on_tick"]:
                    await callback(self.world, scaled_delta)

                # Collect performance data
                if self.world.tick_count % 60 == 0:  # Every second
                    self._collect_performance_data()

                last_tick = current_time

            # Check duration limit
            if duration and (current_time - self.start_time) >= duration:
                break

            # Small sleep to prevent CPU spinning
            await asyncio.sleep(0.001)

        await self.stop()

    async def stop(self):
        """Stop the simulation"""
        if self.state == SimulationState.RUNNING:
            self.state = SimulationState.COMPLETED
            self.end_time = time.time()

            # Run end callbacks
            for callback in self.callbacks["on_end"]:
                await callback(self)

            logger.info(f"Simulation completed. Duration: {self.end_time - self.start_time:.2f}s")

    def pause(self):
        """Pause the simulation"""
        if self.state == SimulationState.RUNNING:
            self.state = SimulationState.PAUSED
            logger.info("Simulation paused")

    def resume(self):
        """Resume the simulation"""
        if self.state == SimulationState.PAUSED:
            self.state = SimulationState.RUNNING
            logger.info("Simulation resumed")

    def _collect_performance_data(self):
        """Collect performance metrics"""
        self.performance_data.append({
            "time": self.world.time,
            "tick": self.world.tick_count,
            "entity_count": len(self.world.entities),
            "agent_count": len(self.world.agents),
            "event_count": len(self.world.events),
            "agent_metrics": {
                agent_id: agent.performance_metrics.copy()
                for agent_id, agent in self.world.agents.items()
            }
        })

    def get_results(self) -> Dict[str, Any]:
        """Get simulation results"""
        total_duration = (self.end_time or time.time()) - (self.start_time or 0)

        return {
            "scenario": self.config.scenario_name,
            "state": self.state.value,
            "duration": total_duration,
            "total_ticks": self.world.tick_count,
            "average_tps": self.world.tick_count / total_duration if total_duration > 0 else 0,
            "world_snapshot": self.world.get_state_snapshot(),
            "events": self.world.events[-1000:],  # Last 1000 events
            "performance_data": self.performance_data,
            "agent_summary": self._summarize_agents()
        }

    def _summarize_agents(self) -> Dict[str, Any]:
        """Summarize agent performance"""
        if not self.world.agents:
            return {}

        metrics = {}
        for agent in self.world.agents.values():
            for metric, value in agent.performance_metrics.items():
                if metric not in metrics:
                    metrics[metric] = []
                metrics[metric].append(value)

        summary = {}
        for metric, values in metrics.items():
            summary[metric] = {
                "total": sum(values),
                "average": np.mean(values),
                "min": min(values),
                "max": max(values),
                "std": np.std(values)
            }

        return summary