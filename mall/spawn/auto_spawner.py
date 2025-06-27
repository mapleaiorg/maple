# File: maple/mall/spawn/auto_spawner.py
# Description: Auto-spawner for dynamic agent creation based on environmental
# analysis and workload predictions. Integrates with ARS and UAL.

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Set, Callable
from datetime import datetime, timedelta
import asyncio
import logging
from enum import Enum
import json

from maple.mall.spawn.predictor import SpawnPredictor
from maple.mall.spawn.templates import TemplateRegistry, AgentTemplate
from maple.mall.core.environment import EnvironmentMonitor

logger = logging.getLogger(__name__)


class SpawnStrategy(Enum):
    """Agent spawning strategies"""
    REACTIVE = "reactive"  # Spawn when load exceeds threshold
    PREDICTIVE = "predictive"  # Spawn based on predictions
    SCHEDULED = "scheduled"  # Spawn on schedule
    ADAPTIVE = "adaptive"  # Combine multiple strategies


@dataclass
class SpawnConfig:
    """Configuration for auto-spawner"""
    strategy: SpawnStrategy = SpawnStrategy.ADAPTIVE
    min_agents: int = 1
    max_agents: int = 100
    load_threshold: float = 0.8  # 80% load triggers spawn
    scale_up_cooldown: int = 60  # seconds
    scale_down_cooldown: int = 300  # seconds
    prediction_horizon: int = 300  # seconds ahead to predict
    spawn_batch_size: int = 5  # Max agents to spawn at once

    def validate(self) -> None:
        """Validate configuration"""
        if self.min_agents < 0 or self.max_agents < self.min_agents:
            raise ValueError("Invalid agent count limits")
        if not 0 < self.load_threshold <= 1:
            raise ValueError("Load threshold must be in (0, 1]")


@dataclass
class SpawnRequest:
    """Request to spawn a new agent"""
    agent_id: str
    template_name: str
    capabilities: List[str]
    configuration: Dict[str, Any]
    parent_agent: Optional[str] = None
    priority: str = "normal"
    metadata: Dict[str, Any] = field(default_factory=dict)

    def to_ual_command(self) -> str:
        """Convert to UAL SPAWN command"""
        config_str = json.dumps(self.configuration)
        caps_str = ", ".join(f'"{cap}"' for cap in self.capabilities)

        command = f"""
SPAWN {self.agent_id} FROM {self.template_name} {{
    capabilities: [{caps_str}],
    config: {config_str}
"""

        if self.parent_agent:
            command += f"    parent: \"{self.parent_agent}\",\n"

        command += "}"

        return command


@dataclass
class SpawnDecision:
    """Result of spawn decision making"""
    should_spawn: bool
    num_agents: int
    reason: str
    confidence: float
    template: Optional[str] = None
    capabilities: List[str] = field(default_factory=list)


class AutoSpawner:
    """
    Automatic agent spawner that creates new agents based on
    environmental conditions and workload predictions.
    """

    def __init__(
            self,
            config: SpawnConfig,
            environment_monitor: EnvironmentMonitor,
            template_registry: TemplateRegistry
    ):
        self.config = config
        self.config.validate()

        self.environment_monitor = environment_monitor
        self.template_registry = template_registry
        self.spawn_predictor = SpawnPredictor()

        # Spawn management
        self.active_agents: Dict[str, Dict[str, Any]] = {}
        self.spawn_history: List[Dict[str, Any]] = []
        self.last_scale_up = datetime.min
        self.last_scale_down = datetime.min

        # Callbacks
        self.spawn_callback: Optional[Callable] = None
        self.despawn_callback: Optional[Callable] = None

        # Background task
        self._auto_spawn_task: Optional[asyncio.Task] = None
        self._running = False

        logger.info(f"Auto-spawner initialized with strategy: {config.strategy}")

    async def start(self) -> None:
        """Start auto-spawner"""
        logger.info("Starting auto-spawner")
        self._running = True
        self._auto_spawn_task = asyncio.create_task(self._auto_spawn_loop())

    async def stop(self) -> None:
        """Stop auto-spawner"""
        logger.info("Stopping auto-spawner")
        self._running = False

        if self._auto_spawn_task:
            self._auto_spawn_task.cancel()
            await asyncio.gather(self._auto_spawn_task, return_exceptions=True)

    def set_spawn_callback(self, callback: Callable) -> None:
        """Set callback for agent spawning"""
        self.spawn_callback = callback

    def set_despawn_callback(self, callback: Callable) -> None:
        """Set callback for agent despawning"""
        self.despawn_callback = callback

    async def evaluate_spawn_need(self) -> SpawnDecision:
        """Evaluate whether new agents should be spawned"""
        # Get current environment data
        env_data = await self.environment_monitor.sense_environment()

        # Get predictions
        predictions = await self.environment_monitor.get_predictions()

        # Apply strategy
        if self.config.strategy == SpawnStrategy.REACTIVE:
            decision = await self._reactive_strategy(env_data)
        elif self.config.strategy == SpawnStrategy.PREDICTIVE:
            decision = await self._predictive_strategy(env_data, predictions)
        elif self.config.strategy == SpawnStrategy.SCHEDULED:
            decision = await self._scheduled_strategy(env_data)
        else:  # ADAPTIVE
            decision = await self._adaptive_strategy(env_data, predictions)

        return decision

    async def spawn_agents(self, decision: SpawnDecision) -> List[str]:
        """Spawn agents based on decision"""
        if not decision.should_spawn:
            return []

        # Check cooldown
        if datetime.utcnow() - self.last_scale_up < timedelta(seconds=self.config.scale_up_cooldown):
            logger.info("Spawn skipped due to cooldown")
            return []

        # Check limits
        current_count = len(self.active_agents)
        max_spawn = min(
            decision.num_agents,
            self.config.max_agents - current_count,
            self.config.spawn_batch_size
        )

        if max_spawn <= 0:
            logger.info("Max agent limit reached")
            return []

        spawned_agents = []

        # Get template
        template = self.template_registry.get_template(
            decision.template or "default"
        )

        # Spawn agents
        for i in range(max_spawn):
            agent_id = f"{template.name}-{datetime.utcnow().timestamp()}-{i}"

            spawn_request = SpawnRequest(
                agent_id=agent_id,
                template_name=template.name,
                capabilities=decision.capabilities or template.default_capabilities,
                configuration=template.default_config,
                metadata={
                    "spawn_reason": decision.reason,
                    "spawn_time": datetime.utcnow().isoformat(),
                    "auto_spawned": True,
                }
            )

            # Execute spawn
            if self.spawn_callback:
                await self.spawn_callback(spawn_request)

            # Track agent
            self.active_agents[agent_id] = {
                "template": template.name,
                "spawned_at": datetime.utcnow(),
                "capabilities": spawn_request.capabilities,
            }

            spawned_agents.append(agent_id)

            logger.info(f"Spawned agent {agent_id} (reason: {decision.reason})")

        # Update history
        self.spawn_history.append({
            "timestamp": datetime.utcnow(),
            "agents": spawned_agents,
            "reason": decision.reason,
            "decision": decision,
        })

        self.last_scale_up = datetime.utcnow()

        return spawned_agents

    async def despawn_agents(self, agent_ids: List[str]) -> List[str]:
        """Despawn specified agents"""
        # Check cooldown
        if datetime.utcnow() - self.last_scale_down < timedelta(seconds=self.config.scale_down_cooldown):
            logger.info("Despawn skipped due to cooldown")
            return []

        # Check limits
        current_count = len(self.active_agents)
        max_despawn = min(
            len(agent_ids),
            current_count - self.config.min_agents
        )

        if max_despawn <= 0:
            logger.info("Min agent limit reached")
            return []

        despawned_agents = []

        for agent_id in agent_ids[:max_despawn]:
            if agent_id in self.active_agents:
                # Execute despawn
                if self.despawn_callback:
                    await self.despawn_callback(agent_id)

                # Remove from tracking
                del self.active_agents[agent_id]
                despawned_agents.append(agent_id)

                logger.info(f"Despawned agent {agent_id}")

        self.last_scale_down = datetime.utcnow()

        return despawned_agents

    async def _reactive_strategy(self, env_data) -> SpawnDecision:
        """Reactive spawning based on current load"""
        # Calculate current load
        if env_data.active_agents == 0:
            load = 1.0  # No agents, max load
        else:
            load = env_data.task_backlog / (env_data.active_agents * 10)

        if load > self.config.load_threshold:
            # Need more agents
            num_agents = min(
                int((load - self.config.load_threshold) * 10),
                self.config.spawn_batch_size
            )

            return SpawnDecision(
                should_spawn=True,
                num_agents=num_agents,
                reason=f"High load: {load:.2f}",
                confidence=0.9,
                template="worker",
                capabilities=["process", "compute"]
            )
        elif load < 0.3 and len(self.active_agents) > self.config.min_agents:
            # Can reduce agents
            return SpawnDecision(
                should_spawn=False,
                num_agents=-1,  # Negative indicates despawn
                reason=f"Low load: {load:.2f}",
                confidence=0.8
            )

        return SpawnDecision(
            should_spawn=False,
            num_agents=0,
            reason="Load within normal range",
            confidence=0.7
        )

    async def _predictive_strategy(self, env_data, predictions) -> SpawnDecision:
        """Predictive spawning based on forecasts"""
        if predictions.get("insufficient_data"):
            # Fall back to reactive
            return await self._reactive_strategy(env_data)

        # Use predictor
        prediction = await self.spawn_predictor.predict(
            env_data,
            predictions,
            self.config.prediction_horizon
        )

        if prediction.predicted_load > self.config.load_threshold:
            # Preemptively spawn
            num_agents = int(
                (prediction.predicted_load - self.config.load_threshold) * 10
            )

            return SpawnDecision(
                should_spawn=True,
                num_agents=num_agents,
                reason=f"Predicted high load: {prediction.predicted_load:.2f}",
                confidence=prediction.confidence,
                template="worker",
                capabilities=prediction.recommended_capabilities
            )

        return SpawnDecision(
            should_spawn=False,
            num_agents=0,
            reason="No high load predicted",
            confidence=prediction.confidence
        )

    async def _scheduled_strategy(self, env_data) -> SpawnDecision:
        """Scheduled spawning based on time patterns"""
        current_hour = datetime.utcnow().hour

        # Business hours pattern (example)
        if 9 <= current_hour <= 17:  # Business hours
            target_agents = 20
        else:
            target_agents = 5

        current_agents = len(self.active_agents)

        if current_agents < target_agents:
            return SpawnDecision(
                should_spawn=True,
                num_agents=target_agents - current_agents,
                reason=f"Scheduled scaling for hour {current_hour}",
                confidence=1.0,
                template="worker"
            )
        elif current_agents > target_agents + 5:  # Some buffer
            return SpawnDecision(
                should_spawn=False,
                num_agents=-(current_agents - target_agents),
                reason=f"Scheduled downscaling for hour {current_hour}",
                confidence=1.0
            )

        return SpawnDecision(
            should_spawn=False,
            num_agents=0,
            reason="Within scheduled range",
            confidence=1.0
        )

    async def _adaptive_strategy(self, env_data, predictions) -> SpawnDecision:
        """Adaptive strategy combining multiple approaches"""
        # Get decisions from all strategies
        reactive = await self._reactive_strategy(env_data)
        predictive = await self._predictive_strategy(env_data, predictions)
        scheduled = await self._scheduled_strategy(env_data)

        # Weight decisions by confidence
        decisions = [reactive, predictive, scheduled]
        weights = [d.confidence for d in decisions]

        # If any strategy strongly suggests spawning
        spawn_decisions = [d for d in decisions if d.should_spawn and d.confidence > 0.7]

        if spawn_decisions:
            # Use the most confident spawn decision
            best_decision = max(spawn_decisions, key=lambda d: d.confidence)
            return best_decision

        # Check if we should despawn
        despawn_decisions = [d for d in decisions if d.num_agents < 0]
        if len(despawn_decisions) >= 2:  # Multiple strategies agree
            return despawn_decisions[0]

        return SpawnDecision(
            should_spawn=False,
            num_agents=0,
            reason="No consensus for scaling",
            confidence=0.5
        )

    async def _auto_spawn_loop(self) -> None:
        """Background loop for auto-spawning"""
        while self._running:
            try:
                # Evaluate spawn need
                decision = await self.evaluate_spawn_need()

                if decision.should_spawn and decision.num_agents > 0:
                    # Spawn agents
                    await self.spawn_agents(decision)
                elif decision.num_agents < 0:
                    # Despawn agents
                    # Select oldest agents for despawning
                    sorted_agents = sorted(
                        self.active_agents.items(),
                        key=lambda x: x[1]["spawned_at"]
                    )
                    agents_to_remove = [
                        agent_id for agent_id, _ in sorted_agents[:abs(decision.num_agents)]
                    ]
                    await self.despawn_agents(agents_to_remove)

                # Wait before next evaluation
                await asyncio.sleep(10)  # Check every 10 seconds

            except Exception as e:
                logger.error(f"Auto-spawn loop error: {e}")
                await asyncio.sleep(1)

    def get_metrics(self) -> Dict[str, Any]:
        """Get auto-spawner metrics"""
        return {
            "strategy": self.config.strategy.value,
            "active_agents": len(self.active_agents),
            "total_spawned": len(self.spawn_history),
            "last_scale_up": self.last_scale_up.isoformat(),
            "last_scale_down": self.last_scale_down.isoformat(),
            "spawn_history_size": len(self.spawn_history),
        }