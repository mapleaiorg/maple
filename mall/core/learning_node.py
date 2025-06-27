# File: mall/core/learning_node.py
# Description: Learning node implementation for MALL's distributed learning network.
# Each node is responsible for training, evaluating, and deploying agent models.

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Set
from datetime import datetime
import asyncio
import logging
from enum import Enum
import numpy as np
import torch
import torch.nn as nn
from uuid import uuid4

from core.map.models.message import Message, MessageType
from mall.models.agent_model import AgentModel
from mall.security.privacy import PrivacyManager

logger = logging.getLogger(__name__)


class NodeStatus(Enum):
    """Learning node operational status"""
    INITIALIZING = "initializing"
    ACTIVE = "active"
    TRAINING = "training"
    AGGREGATING = "aggregating"
    IDLE = "idle"
    ERROR = "error"
    SHUTDOWN = "shutdown"


@dataclass
class NodeConfig:
    """Configuration for a learning node"""
    node_id: str = field(default_factory=lambda: f"node-{uuid4().hex[:8]}")
    shard_id: str = "default"
    max_concurrent_training: int = 10
    federated_rounds: int = 5
    aggregation_threshold: float = 0.8
    checkpoint_interval: int = 300  # seconds
    privacy_enabled: bool = True
    differential_privacy_epsilon: float = 1.0
    homomorphic_encryption: bool = False
    metrics_port: int = 9090

    def validate(self) -> None:
        """Validate configuration"""
        if self.max_concurrent_training <= 0:
            raise ValueError("max_concurrent_training must be positive")
        if not 0 < self.aggregation_threshold <= 1:
            raise ValueError("aggregation_threshold must be in (0, 1]")


@dataclass
class TrainingTask:
    """Represents a training task for an agent"""
    task_id: str
    agent_id: str
    model: AgentModel
    dataset: Any  # Would be torch.utils.data.Dataset
    epochs: int
    learning_rate: float
    batch_size: int
    status: str = "pending"
    progress: float = 0.0
    metrics: Dict[str, float] = field(default_factory=dict)
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None


class LearningNode:
    """
    A distributed learning node in MALL's network.
    Handles local training, model aggregation, and privacy-preserving updates.
    """

    def __init__(self, config: NodeConfig):
        self.config = config
        self.config.validate()

        self.node_id = config.node_id
        self.shard_id = config.shard_id
        self.status = NodeStatus.INITIALIZING

        # Training management
        self.training_queue: asyncio.Queue[TrainingTask] = asyncio.Queue()
        self.active_tasks: Dict[str, TrainingTask] = {}
        self.completed_tasks: List[TrainingTask] = []

        # Model registry
        self.local_models: Dict[str, AgentModel] = {}
        self.global_models: Dict[str, AgentModel] = {}

        # Privacy manager
        self.privacy_manager = PrivacyManager(
            differential_privacy=config.privacy_enabled,
            epsilon=config.differential_privacy_epsilon,
            homomorphic=config.homomorphic_encryption
        )

        # Metrics
        self.metrics = {
            "tasks_completed": 0,
            "tasks_failed": 0,
            "total_training_time": 0.0,
            "average_loss": 0.0,
            "models_trained": 0,
        }

        # Background tasks
        self._training_workers: List[asyncio.Task] = []
        self._checkpoint_task: Optional[asyncio.Task] = None
        self._running = False

        logger.info(f"Learning node {self.node_id} initialized for shard {self.shard_id}")

    async def start(self) -> None:
        """Start the learning node"""
        logger.info(f"Starting learning node {self.node_id}")
        self._running = True
        self.status = NodeStatus.ACTIVE

        # Start training workers
        for i in range(self.config.max_concurrent_training):
            worker = asyncio.create_task(self._training_worker(i))
            self._training_workers.append(worker)

        # Start checkpoint task
        self._checkpoint_task = asyncio.create_task(self._checkpoint_loop())

        logger.info(f"Learning node {self.node_id} started with {len(self._training_workers)} workers")

    async def stop(self) -> None:
        """Stop the learning node"""
        logger.info(f"Stopping learning node {self.node_id}")
        self._running = False
        self.status = NodeStatus.SHUTDOWN

        # Cancel workers
        for worker in self._training_workers:
            worker.cancel()

        if self._checkpoint_task:
            self._checkpoint_task.cancel()

        # Wait for workers to finish
        await asyncio.gather(*self._training_workers, return_exceptions=True)

        logger.info(f"Learning node {self.node_id} stopped")

    async def submit_training_task(self, task: TrainingTask) -> str:
        """Submit a training task to the node"""
        if self.status != NodeStatus.ACTIVE:
            raise RuntimeError(f"Node {self.node_id} is not active")

        await self.training_queue.put(task)
        logger.info(f"Training task {task.task_id} submitted for agent {task.agent_id}")
        return task.task_id

    async def train_agent(
            self,
            agent_id: str,
            model: AgentModel,
            task_type: str,
            config: Dict[str, Any]
    ) -> AgentModel:
        """Train an agent model"""
        task = TrainingTask(
            task_id=f"{agent_id}-{task_type}-{uuid4().hex[:8]}",
            agent_id=agent_id,
            model=model,
            dataset=config.get("dataset"),
            epochs=config.get("epochs", 10),
            learning_rate=config.get("learning_rate", 0.001),
            batch_size=config.get("batch_size", 32)
        )

        task_id = await self.submit_training_task(task)

        # Wait for completion
        while task_id not in [t.task_id for t in self.completed_tasks]:
            await asyncio.sleep(0.1)

        # Return updated model
        return self.local_models.get(agent_id, model)

    async def federated_aggregate(
            self,
            model_updates: List[Dict[str, torch.Tensor]],
            base_model: AgentModel
    ) -> AgentModel:
        """Aggregate model updates using federated averaging"""
        if not model_updates:
            return base_model

        logger.info(f"Aggregating {len(model_updates)} model updates")

        # Apply privacy if enabled
        if self.config.privacy_enabled:
            model_updates = await self.privacy_manager.add_noise(model_updates)

        # Federated averaging
        aggregated_state = {}
        for key in model_updates[0].keys():
            # Average parameters across all updates
            stacked = torch.stack([update[key] for update in model_updates])
            aggregated_state[key] = torch.mean(stacked, dim=0)

        # Update base model
        base_model.load_state_dict(aggregated_state)

        return base_model

    async def _training_worker(self, worker_id: int) -> None:
        """Background worker for processing training tasks"""
        logger.info(f"Training worker {worker_id} started on node {self.node_id}")

        while self._running:
            try:
                # Get task with timeout
                task = await asyncio.wait_for(
                    self.training_queue.get(),
                    timeout=1.0
                )

                # Process training task
                await self._process_training_task(task)

            except asyncio.TimeoutError:
                continue
            except Exception as e:
                logger.error(f"Training worker {worker_id} error: {e}")
                await asyncio.sleep(1)

        logger.info(f"Training worker {worker_id} stopped")

    async def _process_training_task(self, task: TrainingTask) -> None:
        """Process a single training task"""
        logger.info(f"Processing training task {task.task_id}")
        task.status = "running"
        task.started_at = datetime.utcnow()
        self.active_tasks[task.task_id] = task

        try:
            self.status = NodeStatus.TRAINING

            # Simulate training (would be actual PyTorch training loop)
            model = task.model
            optimizer = torch.optim.Adam(
                model.parameters(),
                lr=task.learning_rate
            )

            for epoch in range(task.epochs):
                # Training loop would go here
                task.progress = (epoch + 1) / task.epochs
                task.metrics["loss"] = np.random.random() * 0.5  # Simulated

                # Simulate training time
                await asyncio.sleep(0.1)

            # Update local model registry
            self.local_models[task.agent_id] = model

            # Mark as completed
            task.status = "completed"
            task.completed_at = datetime.utcnow()
            self.completed_tasks.append(task)

            # Update metrics
            self.metrics["tasks_completed"] += 1
            self.metrics["models_trained"] += 1

            logger.info(f"Training task {task.task_id} completed successfully")

        except Exception as e:
            logger.error(f"Training task {task.task_id} failed: {e}")
            task.status = "failed"
            self.metrics["tasks_failed"] += 1

        finally:
            self.active_tasks.pop(task.task_id, None)
            if not self.active_tasks:
                self.status = NodeStatus.IDLE

    async def _checkpoint_loop(self) -> None:
        """Periodically checkpoint models"""
        while self._running:
            try:
                await asyncio.sleep(self.config.checkpoint_interval)
                await self._save_checkpoint()
            except Exception as e:
                logger.error(f"Checkpoint error: {e}")

    async def _save_checkpoint(self) -> None:
        """Save model checkpoints"""
        logger.info(f"Saving checkpoint for node {self.node_id}")
        # Would implement actual model serialization here
        pass

    def get_metrics(self) -> Dict[str, Any]:
        """Get node metrics"""
        return {
            "node_id": self.node_id,
            "shard_id": self.shard_id,
            "status": self.status.value,
            "active_tasks": len(self.active_tasks),
            "completed_tasks": len(self.completed_tasks),
            **self.metrics
        }