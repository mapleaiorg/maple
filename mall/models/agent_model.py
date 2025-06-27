# File: mall/models/agent_model.py
# Description: Base agent model class and utilities for MALL.

from __future__ import annotations
from dataclasses import dataclass
from typing import Dict, List, Optional, Any
from datetime import datetime
from enum import Enum
import torch
import torch.nn as nn
from abc import ABC, abstractmethod
import logging

logger = logging.getLogger(__name__)


class ModelType(Enum):
    """Types of agent models"""
    DQN = "dqn"
    POLICY_GRADIENT = "policy_gradient"
    ACTOR_CRITIC = "actor_critic"
    TRANSFORMER = "transformer"
    CUSTOM = "custom"


@dataclass
class ModelInfo:
    """Information about an agent model"""
    model_id: str
    model_type: ModelType
    version: str
    created_at: datetime
    updated_at: datetime
    training_steps: int
    performance_metrics: Dict[str, float]
    capabilities: List[str]
    metadata: Dict[str, Any]


class AgentModel(nn.Module, ABC):
    """
    Base class for agent models in MALL.
    All agent models should inherit from this class.
    """

    def __init__(
            self,
            model_id: str,
            model_type: ModelType,
            input_size: int,
            output_size: int
    ):
        super(AgentModel, self).__init__()

        self.model_id = model_id
        self.model_type = model_type
        self.input_size = input_size
        self.output_size = output_size

        self.version = "1.0"
        self.created_at = datetime.utcnow()
        self.updated_at = datetime.utcnow()
        self.training_steps = 0

        self.performance_metrics: Dict[str, float] = {}
        self.metadata: Dict[str, Any] = {}

    @abstractmethod
    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Forward pass through the model"""
        pass

    @abstractmethod
    def get_action(self, state: torch.Tensor) -> Any:
        """Get action from state"""
        pass

    def update_metrics(self, metrics: Dict[str, float]) -> None:
        """Update performance metrics"""
        self.performance_metrics.update(metrics)
        self.updated_at = datetime.utcnow()

    def increment_training_steps(self, steps: int = 1) -> None:
        """Increment training step counter"""
        self.training_steps += steps
        self.updated_at = datetime.utcnow()

    def get_info(self) -> ModelInfo:
        """Get model information"""
        return ModelInfo(
            model_id=self.model_id,
            model_type=self.model_type,
            version=self.version,
            created_at=self.created_at,
            updated_at=self.updated_at,
            training_steps=self.training_steps,
            performance_metrics=self.performance_metrics.copy(),
            capabilities=self.metadata.get("capabilities", []),
            metadata=self.metadata.copy()
        )

    def clone(self) -> AgentModel:
        """Create a copy of the model"""
        # Create new instance
        cloned = self.__class__(
            model_id=f"{self.model_id}-clone",
            model_type=self.model_type,
            input_size=self.input_size,
            output_size=self.output_size
        )

        # Copy state
        cloned.load_state_dict(self.state_dict())
        cloned.version = self.version
        cloned.performance_metrics = self.performance_metrics.copy()
        cloned.metadata = self.metadata.copy()

        return cloned

    def to_holographic(self) -> Dict[str, Any]:
        """Convert model to holographic representation for MAP transfer"""
        # Compress model state
        state_dict = self.state_dict()

        # Quantize weights for compression
        compressed_state = {}
        for key, tensor in state_dict.items():
            # Simple 8-bit quantization
            min_val = tensor.min().item()
            max_val = tensor.max().item()
            scale = (max_val - min_val) / 255.0

            quantized = ((tensor - min_val) / scale).byte()
            compressed_state[key] = {
                "data": quantized.numpy().tobytes(),
                "shape": list(tensor.shape),
                "min": min_val,
                "max": max_val,
            }

        return {
            "model_id": self.model_id,
            "model_type": self.model_type.value,
            "version": self.version,
            "compressed_state": compressed_state,
            "metrics": self.performance_metrics,
            "metadata": self.metadata,
        }

    @classmethod
    def from_holographic(cls, data: Dict[str, Any]) -> AgentModel:
        """Reconstruct model from holographic representation"""
        # Would implement decompression and reconstruction
        raise NotImplementedError("Holographic reconstruction not yet implemented")

    def save_checkpoint(self, path: str) -> None:
        """Save model checkpoint"""
        torch.save({
            "model_state": self.state_dict(),
            "model_id": self.model_id,
            "model_type": self.model_type.value,
            "version": self.version,
            "created_at": self.created_at.isoformat(),
            "updated_at": self.updated_at.isoformat(),
            "training_steps": self.training_steps,
            "performance_metrics": self.performance_metrics,
            "metadata": self.metadata,
        }, path)
        logger.info(f"Model checkpoint saved to {path}")

    def load_checkpoint(self, path: str) -> None:
        """Load model checkpoint"""
        checkpoint = torch.load(path)
        self.load_state_dict(checkpoint["model_state"])
        self.model_id = checkpoint["model_id"]
        self.version = checkpoint["version"]
        self.created_at = datetime.fromisoformat(checkpoint["created_at"])
        self.updated_at = datetime.fromisoformat(checkpoint["updated_at"])
        self.training_steps = checkpoint["training_steps"]
        self.performance_metrics = checkpoint["performance_metrics"]
        self.metadata = checkpoint["metadata"]
        logger.info(f"Model checkpoint loaded from {path}")