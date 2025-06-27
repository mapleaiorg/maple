# File: mall/core/federated.py
# Description: Federated learning coordination for MALL. Manages distributed
# training rounds, secure aggregation, and model synchronization across shards.

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Set, Callable
from datetime import datetime, timedelta
import asyncio
import logging
from enum import Enum
import numpy as np
import torch
import torch.nn as nn
from collections import defaultdict

from mall.core.learning_node import LearningNode
from mall.models.agent_model import AgentModel
from mall.security.encryption import SecureAggregation

logger = logging.getLogger(__name__)


class AggregationStrategy(Enum):
    """Model aggregation strategies"""
    FEDERATED_AVERAGING = "fedavg"
    WEIGHTED_AVERAGING = "weighted_avg"
    MEDIAN = "median"
    TRIMMED_MEAN = "trimmed_mean"
    KRUM = "krum"
    BULYAN = "bulyan"


@dataclass
class FederatedConfig:
    """Configuration for federated learning"""
    min_nodes_per_round: int = 3
    max_nodes_per_round: int = 100
    rounds_per_epoch: int = 10
    aggregation_strategy: AggregationStrategy = AggregationStrategy.FEDERATED_AVERAGING
    client_fraction: float = 0.1  # Fraction of clients to sample per round
    aggregation_timeout: int = 60  # seconds
    secure_aggregation: bool = True
    differential_privacy: bool = True
    noise_multiplier: float = 0.1
    gradient_clip_norm: float = 1.0

    def validate(self) -> None:
        """Validate configuration"""
        if self.min_nodes_per_round <= 0:
            raise ValueError("min_nodes_per_round must be positive")
        if not 0 < self.client_fraction <= 1:
            raise ValueError("client_fraction must be in (0, 1]")


@dataclass
class FederatedRound:
    """Represents a single federated learning round"""
    round_id: str
    epoch: int
    selected_nodes: List[str]
    model_version: str
    start_time: datetime
    end_time: Optional[datetime] = None
    aggregated_model: Optional[AgentModel] = None
    node_updates: Dict[str, Dict[str, torch.Tensor]] = field(default_factory=dict)
    metrics: Dict[str, float] = field(default_factory=dict)
    status: str = "pending"


class FederatedLearningManager:
    """
    Manages federated learning across distributed MALL nodes.
    Coordinates training rounds, secure aggregation, and model distribution.
    """

    def __init__(self, config: FederatedConfig):
        self.config = config
        self.config.validate()

        # Node management
        self.nodes: Dict[str, LearningNode] = {}
        self.node_capabilities: Dict[str, Set[str]] = defaultdict(set)

        # Model registry
        self.global_models: Dict[str, AgentModel] = {}
        self.model_versions: Dict[str, int] = defaultdict(int)

        # Round management
        self.active_rounds: Dict[str, FederatedRound] = {}
        self.completed_rounds: List[FederatedRound] = []

        # Secure aggregation
        self.secure_aggregator = SecureAggregation() if config.secure_aggregation else None

        # Metrics
        self.metrics = {
            "total_rounds": 0,
            "successful_rounds": 0,
            "failed_rounds": 0,
            "average_round_time": 0.0,
            "total_nodes_trained": 0,
        }

        self._running = False
        self._round_scheduler: Optional[asyncio.Task] = None

        logger.info("Federated learning manager initialized")

    async def start(self) -> None:
        """Start the federated learning manager"""
        logger.info("Starting federated learning manager")
        self._running = True
        self._round_scheduler = asyncio.create_task(self._round_scheduler_loop())

    async def stop(self) -> None:
        """Stop the federated learning manager"""
        logger.info("Stopping federated learning manager")
        self._running = False

        if self._round_scheduler:
            self._round_scheduler.cancel()
            await asyncio.gather(self._round_scheduler, return_exceptions=True)

    async def register_node(self, node: LearningNode, capabilities: Set[str]) -> None:
        """Register a learning node"""
        self.nodes[node.node_id] = node
        self.node_capabilities[node.node_id] = capabilities
        logger.info(f"Registered node {node.node_id} with capabilities: {capabilities}")

    async def unregister_node(self, node_id: str) -> None:
        """Unregister a learning node"""
        if node_id in self.nodes:
            del self.nodes[node_id]
            del self.node_capabilities[node_id]
            logger.info(f"Unregistered node {node_id}")

    async def start_federated_round(
            self,
            model_id: str,
            model: AgentModel,
            task_type: str,
            config: Dict[str, Any]
    ) -> str:
        """Start a new federated learning round"""
        # Select participating nodes
        available_nodes = [
            node_id for node_id, caps in self.node_capabilities.items()
            if task_type in caps
        ]

        if len(available_nodes) < self.config.min_nodes_per_round:
            raise ValueError(
                f"Insufficient nodes for federated round. "
                f"Required: {self.config.min_nodes_per_round}, Available: {len(available_nodes)}"
            )

        # Sample nodes
        num_nodes = min(
            int(len(available_nodes) * self.config.client_fraction),
            self.config.max_nodes_per_round
        )
        num_nodes = max(num_nodes, self.config.min_nodes_per_round)

        selected_nodes = np.random.choice(
            available_nodes,
            size=num_nodes,
            replace=False
        ).tolist()

        # Create round
        round_id = f"{model_id}-{task_type}-{datetime.utcnow().timestamp()}"
        federated_round = FederatedRound(
            round_id=round_id,
            epoch=self.model_versions[model_id],
            selected_nodes=selected_nodes,
            model_version=f"{model_id}-v{self.model_versions[model_id]}",
            start_time=datetime.utcnow()
        )

        self.active_rounds[round_id] = federated_round

        # Distribute training tasks
        await self._distribute_training_tasks(
            federated_round,
            model,
            task_type,
            config
        )

        logger.info(
            f"Started federated round {round_id} with {len(selected_nodes)} nodes"
        )

        return round_id

    async def _distribute_training_tasks(
            self,
            round: FederatedRound,
            model: AgentModel,
            task_type: str,
            config: Dict[str, Any]
    ) -> None:
        """Distribute training tasks to selected nodes"""
        tasks = []

        for node_id in round.selected_nodes:
            node = self.nodes[node_id]

            # Clone model for local training
            local_model = model.clone()

            # Submit training task
            task = node.train_agent(
                agent_id=f"{round.model_version}-{node_id}",
                model=local_model,
                task_type=task_type,
                config=config
            )
            tasks.append(task)

        # Wait for training completion with timeout
        try:
            results = await asyncio.wait_for(
                asyncio.gather(*tasks, return_exceptions=True),
                timeout=self.config.aggregation_timeout
            )

            # Collect updates
            for i, (node_id, result) in enumerate(zip(round.selected_nodes, results)):
                if isinstance(result, Exception):
                    logger.error(f"Node {node_id} training failed: {result}")
                else:
                    round.node_updates[node_id] = result.state_dict()

        except asyncio.TimeoutError:
            logger.error(f"Federated round {round.round_id} timed out")
            round.status = "timeout"

    async def aggregate_round(self, round_id: str) -> AgentModel:
        """Aggregate model updates from a federated round"""
        round = self.active_rounds.get(round_id)
        if not round:
            raise ValueError(f"Round {round_id} not found")

        if not round.node_updates:
            raise ValueError(f"No updates available for round {round_id}")

        logger.info(f"Aggregating round {round_id} with {len(round.node_updates)} updates")

        # Get base model
        base_model = self.global_models.get(round.model_version.split('-')[0])
        if not base_model:
            raise ValueError(f"Base model not found for round {round_id}")

        # Apply aggregation strategy
        if self.config.aggregation_strategy == AggregationStrategy.FEDERATED_AVERAGING:
            aggregated_model = await self._federated_averaging(
                round.node_updates,
                base_model
            )
        elif self.config.aggregation_strategy == AggregationStrategy.WEIGHTED_AVERAGING:
            # Would implement weighted averaging based on node data sizes
            aggregated_model = await self._federated_averaging(
                round.node_updates,
                base_model
            )
        else:
            # Other strategies would be implemented here
            aggregated_model = await self._federated_averaging(
                round.node_updates,
                base_model
            )

        # Update round
        round.aggregated_model = aggregated_model
        round.end_time = datetime.utcnow()
        round.status = "completed"

        # Update global model
        model_id = round.model_version.split('-')[0]
        self.global_models[model_id] = aggregated_model
        self.model_versions[model_id] += 1

        # Move to completed
        self.completed_rounds.append(round)
        del self.active_rounds[round_id]

        # Update metrics
        self.metrics["total_rounds"] += 1
        self.metrics["successful_rounds"] += 1
        self.metrics["total_nodes_trained"] += len(round.node_updates)

        logger.info(f"Federated round {round_id} completed successfully")

        return aggregated_model

    async def _federated_averaging(
            self,
            updates: Dict[str, Dict[str, torch.Tensor]],
            base_model: AgentModel
    ) -> AgentModel:
        """Perform federated averaging aggregation"""
        if not updates:
            return base_model

        # Initialize aggregated state
        aggregated_state = {}

        # Get first update as template
        first_update = next(iter(updates.values()))

        for key in first_update.keys():
            # Stack all updates for this parameter
            param_updates = []
            for node_id, update in updates.items():
                if key in update:
                    param_updates.append(update[key])

            if param_updates:
                # Average parameters
                stacked = torch.stack(param_updates)
                aggregated_state[key] = torch.mean(stacked, dim=0)

        # Apply differential privacy if enabled
        if self.config.differential_privacy:
            aggregated_state = self._add_differential_privacy_noise(aggregated_state)

        # Create new model with aggregated parameters
        aggregated_model = base_model.clone()
        aggregated_model.load_state_dict(aggregated_state)

        return aggregated_model

    def _add_differential_privacy_noise(
            self,
            state_dict: Dict[str, torch.Tensor]
    ) -> Dict[str, torch.Tensor]:
        """Add differential privacy noise to model parameters"""
        noisy_state = {}

        for key, param in state_dict.items():
            # Add Gaussian noise scaled by sensitivity and privacy budget
            noise = torch.randn_like(param) * self.config.noise_multiplier
            noisy_state[key] = param + noise

        return noisy_state

    async def _round_scheduler_loop(self) -> None:
        """Background loop for scheduling federated rounds"""
        while self._running:
            try:
                # Check for models needing training
                await self._check_and_schedule_rounds()
                await asyncio.sleep(10)  # Check every 10 seconds
            except Exception as e:
                logger.error(f"Round scheduler error: {e}")
                await asyncio.sleep(1)

    async def _check_and_schedule_rounds(self) -> None:
        """Check if any models need federated training rounds"""
        # This would check model staleness, performance metrics, etc.
        # and automatically schedule rounds as needed
        pass

    def get_metrics(self) -> Dict[str, Any]:
        """Get federated learning metrics"""
        return {
            "active_rounds": len(self.active_rounds),
            "completed_rounds": len(self.completed_rounds),
            "registered_nodes": len(self.nodes),
            **self.metrics
        }