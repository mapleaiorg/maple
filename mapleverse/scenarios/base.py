# File: maple/mapleverse/scenarios/base.py
# Description: Pre-built scenarios and templates for Mapleverse simulations

from __future__ import annotations
from typing import Dict, List, Optional, Any, Callable
from dataclasses import dataclass, field
from abc import ABC, abstractmethod
import random
import asyncio
import logging

from maple.core.mapleverse.engine.base import (
    SimulationConfig, SimulationEngine, SimulationWorld,
    Vector3, AgentEntity, Entity, PhysicsEngine, Resource
)

logger = logging.getLogger(__name__)


class Scenario(ABC):
    """Base class for simulation scenarios"""

    def __init__(self, name: str, description: str):
        self.name = name
        self.description = description
        self.config = self.create_config()
        self.objectives: List[Objective] = []

    @abstractmethod
    def create_config(self) -> SimulationConfig:
        """Create the simulation configuration"""
        pass

    @abstractmethod
    async def setup_world(self, engine: SimulationEngine):
        """Setup the initial world state"""
        pass

    @abstractmethod
    async def on_tick(self, world: SimulationWorld, delta_time: float):
        """Called every simulation tick"""
        pass

    def add_objective(self, objective: 'Objective'):
        """Add an objective to the scenario"""
        self.objectives.append(objective)

    async def check_objectives(self, world: SimulationWorld) -> Dict[str, bool]:
        """Check if objectives are met"""
        results = {}
        for objective in self.objectives:
            results[objective.name] = await objective.is_completed(world)
        return results


@dataclass
class Objective(ABC):
    """Base class for scenario objectives"""
    name: str
    description: str

    @abstractmethod
    async def is_completed(self, world: SimulationWorld) -> bool:
        """Check if objective is completed"""
        pass


class ResourceCollectionObjective(Objective):
    """Objective to collect a certain amount of resources"""

    def __init__(self, name: str, resource_type: str, target_amount: float):
        super().__init__(
            name,
            f"Collect {target_amount} units of {resource_type}"
        )
        self.resource_type = resource_type
        self.target_amount = target_amount

    async def is_completed(self, world: SimulationWorld) -> bool:
        total = 0
        for agent in world.agents.values():
            if self.resource_type in agent.resources:
                total += agent.resources[self.resource_type].amount
        return total >= self.target_amount


class SupplyChainScenario(Scenario):
    """Supply chain optimization scenario"""

    def __init__(self):
        super().__init__(
            "Supply Chain Optimization",
            "Optimize delivery routes and inventory management across multiple warehouses"
        )
        self.warehouses: List[Entity] = []
        self.delivery_points: List[Entity] = []
        self.resource_spawners: List[Dict[str, Any]] = []

    def create_config(self) -> SimulationConfig:
        return SimulationConfig(
            scenario_name="supply_chain",
            physics_engine=PhysicsEngine.SIMPLE,
            world_bounds=(Vector3(-500, 0, -500), Vector3(500, 100, 500)),
            gravity=Vector3(0, -9.81, 0),
            time_scale=1.0,
            max_agents=100,
            max_entities=1000,
            tick_rate=30,
            resource_types=["packages", "fuel", "time"],
            custom_rules={
                "delivery_reward": 10,
                "fuel_cost_per_meter": 0.1,
                "package_spawn_rate": 5.0,
                "max_cargo_capacity": 50
            }
        )

    async def setup_world(self, engine: SimulationEngine):
        """Setup supply chain world"""
        logger.info("Setting up supply chain scenario")

        # Create warehouses
        warehouse_positions = [
            Vector3(-200, 0, -200),
            Vector3(200, 0, -200),
            Vector3(-200, 0, 200),
            Vector3(200, 0, 200),
            Vector3(0, 0, 0)  # Central hub
        ]

        for i, pos in enumerate(warehouse_positions):
            warehouse = WarehouseEntity(f"warehouse_{i}", f"Warehouse {i + 1}")
            warehouse.transform.position = pos
            engine.world.add_entity(warehouse)
            self.warehouses.append(warehouse)

        # Create delivery points
        for i in range(20):
            delivery_point = DeliveryPointEntity(
                f"delivery_{i}",
                f"Delivery Point {i + 1}"
            )
            # Random position
            delivery_point.transform.position = Vector3(
                random.uniform(-400, 400),
                0,
                random.uniform(-400, 400)
            )
            engine.world.add_entity(delivery_point)
            self.delivery_points.append(delivery_point)

        # Setup resource spawning
        self.resource_spawners = [
            {
                "warehouse": warehouse,
                "spawn_timer": 0.0,
                "spawn_interval": 5.0
            }
            for warehouse in self.warehouses
        ]

        # Add objectives
        self.add_objective(
            ResourceCollectionObjective("deliver_packages", "delivered", 1000)
        )

        # Register callbacks
        engine.add_callback("on_tick", self.on_tick)
        engine.add_callback("on_agent_spawn", self._on_agent_spawn)

    async def on_tick(self, world: SimulationWorld, delta_time: float):
        """Update supply chain simulation"""
        # Spawn packages at warehouses
        for spawner in self.resource_spawners:
            spawner["spawn_timer"] += delta_time
            if spawner["spawn_timer"] >= spawner["spawn_interval"]:
                spawner["spawn_timer"] = 0.0
                warehouse = spawner["warehouse"]
                if hasattr(warehouse, 'package_count'):
                    warehouse.package_count = min(
                        warehouse.package_count + 10,
                        warehouse.max_packages
                    )

        # Process agent deliveries
        for agent in world.agents.values():
            # Check if agent is at warehouse
            for warehouse in self.warehouses:
                distance = (agent.transform.position - warehouse.transform.position).magnitude()
                if distance < 10.0:  # Within pickup range
                    await self._handle_warehouse_interaction(agent, warehouse, world)

            # Check if agent is at delivery point
            for delivery_point in self.delivery_points:
                distance = (agent.transform.position - delivery_point.transform.position).magnitude()
                if distance < 5.0:  # Within delivery range
                    await self._handle_delivery(agent, delivery_point, world)

    async def _on_agent_spawn(self, agent: AgentEntity):
        """Initialize agent for supply chain scenario"""
        # Add cargo capacity
        agent.add_component("cargo", {
            "packages": 0,
            "max_capacity": self.config.custom_rules["max_cargo_capacity"]
        })

        # Add fuel resource
        agent.add_resource("fuel", 100.0, 100.0, 0.0)
        agent.add_resource("delivered", 0.0, float('inf'), 0.0)

    async def _handle_warehouse_interaction(self, agent: AgentEntity,
                                            warehouse: 'WarehouseEntity',
                                            world: SimulationWorld):
        """Handle agent picking up packages"""
        cargo = agent.get_component("cargo")
        if cargo and hasattr(warehouse, 'package_count'):
            available_space = cargo["max_capacity"] - cargo["packages"]
            if available_space > 0 and warehouse.package_count > 0:
                pickup_amount = min(available_space, warehouse.package_count)
                cargo["packages"] += pickup_amount
                warehouse.package_count -= pickup_amount

                world.add_event("package_pickup", {
                    "agent_id": agent.id,
                    "warehouse_id": warehouse.id,
                    "amount": pickup_amount
                })

    async def _handle_delivery(self, agent: AgentEntity,
                               delivery_point: 'DeliveryPointEntity',
                               world: SimulationWorld):
        """Handle agent delivering packages"""
        cargo = agent.get_component("cargo")
        if cargo and cargo["packages"] > 0:
            delivered = cargo["packages"]
            cargo["packages"] = 0

            # Update delivered resource
            if "delivered" in agent.resources:
                agent.resources["delivered"].produce(delivered)

            # Reward agent
            reward = delivered * self.config.custom_rules["delivery_reward"]
            agent.performance_metrics["resources_collected"] += reward

            world.add_event("package_delivery", {
                "agent_id": agent.id,
                "delivery_point_id": delivery_point.id,
                "amount": delivered,
                "reward": reward
            })


class WarehouseEntity(Entity):
    """Warehouse entity for supply chain scenario"""

    def __init__(self, entity_id: str, name: str):
        super().__init__(entity_id, name)
        self.package_count = 0
        self.max_packages = 1000
        self.physics.is_static = True

    def update(self, delta_time: float):
        pass


class DeliveryPointEntity(Entity):
    """Delivery point entity"""

    def __init__(self, entity_id: str, name: str):
        super().__init__(entity_id, name)
        self.physics.is_static = True
        self.total_delivered = 0

    def update(self, delta_time: float):
        pass


class CompetitiveResourceScenario(Scenario):
    """Competitive resource gathering scenario"""

    def __init__(self):
        super().__init__(
            "Competitive Resource Gathering",
            "Multiple agents compete for limited resources in an arena"
        )
        self.resource_nodes: List[ResourceNode] = []
        self.respawn_timer = 0.0

    def create_config(self) -> SimulationConfig:
        return SimulationConfig(
            scenario_name="competitive_resources",
            physics_engine=PhysicsEngine.SIMPLE,
            world_bounds=(Vector3(-200, 0, -200), Vector3(200, 50, 200)),
            gravity=Vector3(0, -9.81, 0),
            time_scale=1.0,
            max_agents=50,
            max_entities=500,
            tick_rate=60,
            resource_types=["energy", "materials", "data"],
            custom_rules={
                "resource_respawn_time": 10.0,
                "harvest_time": 2.0,
                "resource_value": {
                    "energy": 10,
                    "materials": 15,
                    "data": 20
                },
                "node_capacity": 100
            }
        )

    async def setup_world(self, engine: SimulationEngine):
        """Setup competitive resource world"""
        logger.info("Setting up competitive resource scenario")

        # Create resource nodes in a grid pattern
        grid_size = 8
        spacing = 40
        offset = (grid_size - 1) * spacing / 2

        resource_types = ["energy", "materials", "data"]

        for x in range(grid_size):
            for z in range(grid_size):
                # Skip center area for agent spawning
                if abs(x - grid_size / 2) < 2 and abs(z - grid_size / 2) < 2:
                    continue

                node_type = random.choice(resource_types)
                node = ResourceNode(
                    f"node_{x}_{z}",
                    node_type,
                    self.config.custom_rules["node_capacity"]
                )

                node.transform.position = Vector3(
                    x * spacing - offset,
                    0,
                    z * spacing - offset
                )

                engine.world.add_entity(node)
                self.resource_nodes.append(node)

        # Add competitive objectives
        for resource_type in resource_types:
            self.add_objective(
                ResourceCollectionObjective(
                    f"collect_{resource_type}",
                    resource_type,
                    5000
                )
            )

        # Register callbacks
        engine.add_callback("on_tick", self.on_tick)
        engine.add_callback("on_agent_spawn", self._on_agent_spawn)

    async def on_tick(self, world: SimulationWorld, delta_time: float):
        """Update competitive resource simulation"""
        # Respawn depleted nodes
        self.respawn_timer += delta_time
        if self.respawn_timer >= self.config.custom_rules["resource_respawn_time"]:
            self.respawn_timer = 0.0
            for node in self.resource_nodes:
                if node.current_amount <= 0:
                    node.respawn()

        # Handle agent harvesting
        for agent in world.agents.values():
            await self._check_harvesting(agent, world)

    async def _on_agent_spawn(self, agent: AgentEntity):
        """Initialize agent for competitive scenario"""
        # Add harvesting state
        agent.add_component("harvesting", {
            "target_node": None,
            "harvest_timer": 0.0,
            "is_harvesting": False
        })

        # Add combat stats
        agent.add_component("combat", {
            "health": 100.0,
            "max_health": 100.0,
            "attack_power": 10.0,
            "defense": 5.0
        })

    async def _check_harvesting(self, agent: AgentEntity, world: SimulationWorld):
        """Check if agent can harvest nearby nodes"""
        harvest_state = agent.get_component("harvesting")
        if not harvest_state:
            return

        # Find nearby resource nodes
        nearby_nodes = []
        for node in self.resource_nodes:
            if node.current_amount > 0:
                distance = (agent.transform.position - node.transform.position).magnitude()
                if distance < 5.0:  # Harvest range
                    nearby_nodes.append((distance, node))

        if not nearby_nodes:
            harvest_state["is_harvesting"] = False
            harvest_state["target_node"] = None
            return

        # Start or continue harvesting closest node
        nearby_nodes.sort(key=lambda x: x[0])
        _, closest_node = nearby_nodes[0]

        if not harvest_state["is_harvesting"]:
            harvest_state["is_harvesting"] = True
            harvest_state["target_node"] = closest_node.id
            harvest_state["harvest_timer"] = 0.0

        if harvest_state["target_node"] == closest_node.id:
            harvest_state["harvest_timer"] += world.tick_count / self.config.tick_rate

            if harvest_state["harvest_timer"] >= self.config.custom_rules["harvest_time"]:
                # Complete harvest
                amount = min(10, closest_node.current_amount)
                closest_node.harvest(amount)

                # Give resources to agent
                if closest_node.resource_type in agent.resources:
                    agent.resources[closest_node.resource_type].produce(amount)

                # Update metrics
                value = amount * self.config.custom_rules["resource_value"][closest_node.resource_type]
                agent.performance_metrics["resources_collected"] += value

                world.add_event("resource_harvested", {
                    "agent_id": agent.id,
                    "node_id": closest_node.id,
                    "resource_type": closest_node.resource_type,
                    "amount": amount,
                    "value": value
                })

                # Reset harvest state
                harvest_state["is_harvesting"] = False
                harvest_state["target_node"] = None
                harvest_state["harvest_timer"] = 0.0


class ResourceNode(Entity):
    """Resource node that can be harvested"""

    def __init__(self, entity_id: str, resource_type: str, capacity: float):
        super().__init__(entity_id, f"{resource_type}_node")
        self.resource_type = resource_type
        self.capacity = capacity
        self.current_amount = capacity
        self.physics.is_static = True

    def update(self, delta_time: float):
        pass

    def harvest(self, amount: float) -> float:
        """Harvest resources from node"""
        harvested = min(amount, self.current_amount)
        self.current_amount -= harvested
        return harvested

    def respawn(self):
        """Respawn the resource node"""
        self.current_amount = self.capacity


class SwarmCoordinationScenario(Scenario):
    """Swarm coordination and emergent behavior scenario"""

    def __init__(self):
        super().__init__(
            "Swarm Coordination",
            "Test emergent swarm behaviors and collective intelligence"
        )
        self.targets: List[Entity] = []
        self.obstacles: List[Entity] = []

    def create_config(self) -> SimulationConfig:
        return SimulationConfig(
            scenario_name="swarm_coordination",
            physics_engine=PhysicsEngine.SIMPLE,
            world_bounds=(Vector3(-300, 0, -300), Vector3(300, 100, 300)),
            gravity=Vector3(0, -9.81, 0),
            time_scale=1.0,
            max_agents=200,
            max_entities=1000,
            tick_rate=30,
            resource_types=["energy"],
            custom_rules={
                "swarm_cohesion_radius": 50.0,
                "swarm_separation_radius": 10.0,
                "swarm_alignment_radius": 30.0,
                "target_attraction_strength": 2.0,
                "obstacle_avoidance_strength": 5.0,
                "max_speed": 20.0
            }
        )

    async def setup_world(self, engine: SimulationEngine):
        """Setup swarm coordination world"""
        logger.info("Setting up swarm coordination scenario")

        # Create moving targets
        for i in range(5):
            target = MovingTargetEntity(f"target_{i}", f"Target {i + 1}")
            target.transform.position = Vector3(
                random.uniform(-200, 200),
                random.uniform(10, 50),
                random.uniform(-200, 200)
            )
            engine.world.add_entity(target)
            self.targets.append(target)

        # Create obstacles
        obstacle_positions = [
            Vector3(0, 0, 0),
            Vector3(100, 0, 100),
            Vector3(-100, 0, 100),
            Vector3(100, 0, -100),
            Vector3(-100, 0, -100)
        ]

        for i, pos in enumerate(obstacle_positions):
            obstacle = ObstacleEntity(f"obstacle_{i}", f"Obstacle {i + 1}")
            obstacle.transform.position = pos
            obstacle.transform.scale = Vector3(20, 50, 20)
            engine.world.add_entity(obstacle)
            self.obstacles.append(obstacle)

        # Add swarm objectives
        self.add_objective(SwarmFormationObjective(
            "maintain_formation",
            min_cohesion=0.7,
            max_separation=0.3
        ))

        # Register callbacks
        engine.add_callback("on_tick", self.on_tick)
        engine.add_callback("on_agent_spawn", self._on_agent_spawn)

    async def on_tick(self, world: SimulationWorld, delta_time: float):
        """Update swarm simulation"""
        # Update moving targets
        for target in self.targets:
            if isinstance(target, MovingTargetEntity):
                target.update_movement(delta_time, world.config.world_bounds)

        # Calculate swarm behaviors for each agent
        for agent in world.agents.values():
            velocity = await self._calculate_swarm_velocity(agent, world)
            agent.add_component("velocity", velocity)

            # Update position based on velocity
            max_speed = self.config.custom_rules["max_speed"]
            if velocity.magnitude() > max_speed:
                velocity = velocity.normalize() * max_speed

            agent.transform.position = agent.transform.position + velocity * delta_time

            # Keep agents in bounds
            self._enforce_boundaries(agent, world.config.world_bounds)

    async def _on_agent_spawn(self, agent: AgentEntity):
        """Initialize agent for swarm scenario"""
        # Add swarm behavior components
        agent.add_component("velocity", Vector3(
            random.uniform(-5, 5),
            0,
            random.uniform(-5, 5)
        ))

        agent.add_component("swarm_state", {
            "neighbors": [],
            "nearest_target": None,
            "nearest_obstacle": None
        })

    async def _calculate_swarm_velocity(self, agent: AgentEntity,
                                        world: SimulationWorld) -> Vector3:
        """Calculate swarm behavior velocity"""
        rules = self.config.custom_rules

        # Find neighbors
        neighbors = []
        for other in world.agents.values():
            if other.id != agent.id:
                distance = (other.transform.position - agent.transform.position).magnitude()
                if distance < rules["swarm_cohesion_radius"]:
                    neighbors.append((distance, other))

        velocity = Vector3()

        # Cohesion - move toward average position of neighbors
        if neighbors:
            avg_position = Vector3()
            for _, neighbor in neighbors:
                avg_position = avg_position + neighbor.transform.position
            avg_position = avg_position * (1.0 / len(neighbors))
            cohesion = (avg_position - agent.transform.position).normalize()
            velocity = velocity + cohesion

        # Separation - avoid crowding neighbors
        separation = Vector3()
        for distance, neighbor in neighbors:
            if distance < rules["swarm_separation_radius"] and distance > 0:
                diff = agent.transform.position - neighbor.transform.position
                separation = separation + diff.normalize() * (1.0 / distance)
        if separation.magnitude() > 0:
            velocity = velocity + separation.normalize() * 2.0

        # Alignment - match average velocity of neighbors
        if neighbors:
            avg_velocity = Vector3()
            for _, neighbor in neighbors:
                neighbor_vel = neighbor.get_component("velocity")
                if neighbor_vel:
                    avg_velocity = avg_velocity + neighbor_vel
            avg_velocity = avg_velocity * (1.0 / len(neighbors))
            velocity = velocity + avg_velocity.normalize() * 0.5

        # Target attraction
        nearest_target = None
        min_target_dist = float('inf')
        for target in self.targets:
            distance = (target.transform.position - agent.transform.position).magnitude()
            if distance < min_target_dist:
                min_target_dist = distance
                nearest_target = target

        if nearest_target:
            attraction = (nearest_target.transform.position - agent.transform.position).normalize()
            velocity = velocity + attraction * rules["target_attraction_strength"]

        # Obstacle avoidance
        for obstacle in self.obstacles:
            distance = (obstacle.transform.position - agent.transform.position).magnitude()
            if distance < 50.0:  # Avoidance radius
                avoidance = (agent.transform.position - obstacle.transform.position).normalize()
                velocity = velocity + avoidance * rules["obstacle_avoidance_strength"] * (50.0 - distance) / 50.0

        return velocity

    def _enforce_boundaries(self, agent: AgentEntity, bounds: Tuple[Vector3, Vector3]):
        """Keep agent within world boundaries"""
        min_bound, max_bound = bounds
        pos = agent.transform.position

        # Clamp position
        pos.x = max(min_bound.x + 10, min(pos.x, max_bound.x - 10))
        pos.y = max(0, min(pos.y, max_bound.y - 10))
        pos.z = max(min_bound.z + 10, min(pos.z, max_bound.z - 10))

        agent.transform.position = pos


class MovingTargetEntity(Entity):
    """Moving target for swarm to follow"""

    def __init__(self, entity_id: str, name: str):
        super().__init__(entity_id, name)
        self.movement_pattern = random.choice(["circle", "figure8", "random"])
        self.movement_speed = random.uniform(5, 15)
        self.movement_timer = 0.0

    def update(self, delta_time: float):
        self.movement_timer += delta_time

    def update_movement(self, delta_time: float, bounds: Tuple[Vector3, Vector3]):
        """Update target movement"""
        self.movement_timer += delta_time

        if self.movement_pattern == "circle":
            radius = 100
            self.transform.position.x = radius * np.cos(self.movement_timer * 0.5)
            self.transform.position.z = radius * np.sin(self.movement_timer * 0.5)

        elif self.movement_pattern == "figure8":
            t = self.movement_timer * 0.3
            self.transform.position.x = 100 * np.cos(t)
            self.transform.position.z = 100 * np.sin(2 * t)

        elif self.movement_pattern == "random":
            if random.random() < 0.02:  # Change direction occasionally
                self.movement_speed = random.uniform(5, 15)

            # Random walk
            self.transform.position.x += random.uniform(-1, 1) * self.movement_speed * delta_time
            self.transform.position.z += random.uniform(-1, 1) * self.movement_speed * delta_time

            # Keep in bounds
            min_bound, max_bound = bounds
            self.transform.position.x = max(min_bound.x + 50, min(self.transform.position.x, max_bound.x - 50))
            self.transform.position.z = max(min_bound.z + 50, min(self.transform.position.z, max_bound.z - 50))


class ObstacleEntity(Entity):
    """Static obstacle in the world"""

    def __init__(self, entity_id: str, name: str):
        super().__init__(entity_id, name)
        self.physics.is_static = True

    def update(self, delta_time: float):
        pass


class SwarmFormationObjective(Objective):
    """Objective to maintain swarm formation"""

    def __init__(self, name: str, min_cohesion: float, max_separation: float):
        super().__init__(
            name,
            f"Maintain swarm cohesion > {min_cohesion} and separation < {max_separation}"
        )
        self.min_cohesion = min_cohesion
        self.max_separation = max_separation

    async def is_completed(self, world: SimulationWorld) -> bool:
        if len(world.agents) < 10:  # Need minimum agents
            return False

        # Calculate cohesion metric
        positions = [agent.transform.position for agent in world.agents.values()]
        if not positions:
            return False

        # Average position
        avg_pos = Vector3()
        for pos in positions:
            avg_pos = avg_pos + pos
        avg_pos = avg_pos * (1.0 / len(positions))

        # Average distance from center
        avg_distance = 0
        for pos in positions:
            avg_distance += (pos - avg_pos).magnitude()
        avg_distance /= len(positions)

        # Cohesion score (normalized)
        max_expected_distance = 100.0
        cohesion = 1.0 - min(avg_distance / max_expected_distance, 1.0)

        # Separation score (check minimum distances)
        min_distances = []
        for i, agent1 in enumerate(world.agents.values()):
            min_dist = float('inf')
            for j, agent2 in enumerate(world.agents.values()):
                if i != j:
                    dist = (agent1.transform.position - agent2.transform.position).magnitude()
                    min_dist = min(min_dist, dist)
            if min_dist < float('inf'):
                min_distances.append(min_dist)

        avg_min_distance = sum(min_distances) / len(min_distances) if min_distances else 0
        separation = avg_min_distance / 20.0  # Normalize by expected separation

        return cohesion >= self.min_cohesion and separation <= self.max_separation


# Scenario registry
SCENARIOS = {
    "supply_chain": SupplyChainScenario,
    "competitive_resources": CompetitiveResourceScenario,
    "swarm_coordination": SwarmCoordinationScenario
}


def get_scenario(name: str) -> Optional[Scenario]:
    """Get a scenario by name"""
    scenario_class = SCENARIOS.get(name)
    if scenario_class:
        return scenario_class()
    return None