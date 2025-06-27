# File: mall/spawn/__init__.py
# Description: Auto-spawn module for dynamic agent creation.

from mall.spawn.auto_spawner import AutoSpawner, SpawnConfig, SpawnRequest
from mall.spawn.predictor import SpawnPredictor, PredictionResult
from mall.spawn.templates import AgentTemplate, TemplateRegistry

__all__ = [
    "AutoSpawner",
    "SpawnConfig",
    "SpawnRequest",
    "SpawnPredictor",
    "PredictionResult",
    "AgentTemplate",
    "TemplateRegistry",
]