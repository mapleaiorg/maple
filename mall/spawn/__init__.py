# File: maple/mall/spawn/__init__.py
# Description: Auto-spawn module for dynamic agent creation.

from maple.mall.spawn.auto_spawner import AutoSpawner, SpawnConfig, SpawnRequest
from maple.mall.spawn.predictor import SpawnPredictor, PredictionResult
from maple.mall.spawn.templates import AgentTemplate, TemplateRegistry

__all__ = [
    "AutoSpawner",
    "SpawnConfig",
    "SpawnRequest",
    "SpawnPredictor",
    "PredictionResult",
    "AgentTemplate",
    "TemplateRegistry",
]