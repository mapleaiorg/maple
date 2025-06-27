# File: mall/__init__.py
# Description: Maple Agent Learning Lab (MALL) - The distributed learning environment
# that drives continuous evolution, optimization, and adaptation of cognitive agents.

"""
MALL (Maple Agent Learning Lab) - Decentralized learning infrastructure for MAPLE agents.

Key Features:
- Federated learning across distributed nodes
- Reinforcement learning for agent optimization
- Auto-spawn intelligence for dynamic agent creation
- Privacy-preserving learning with homomorphic encryption
- Emergent behavior simulation
- Transfer learning across shards
"""

from mall.core.learning_node import LearningNode, NodeConfig
from mall.core.federated import FederatedLearningManager, FederatedConfig
from mall.core.reinforcement import ReinforcementEngine, DQNConfig
from mall.spawn.auto_spawner import AutoSpawner, SpawnConfig
from mall.spawn.predictor import SpawnPredictor, EnvironmentAnalyzer
from mall.models.agent_model import AgentModel, ModelInfo
from mall.strategies.gan_strategy import StrategyGAN, StrategyConfig
from mall.security.privacy import PrivacyManager, EncryptionType
from mall.client import MALLClient
from mall.server import MALLServer

__version__ = "0.1.0"

__all__ = [
    # Core
    "LearningNode",
    "NodeConfig",
    "FederatedLearningManager",
    "FederatedConfig",
    "ReinforcementEngine",
    "DQNConfig",

    # Auto-spawn
    "AutoSpawner",
    "SpawnConfig",
    "SpawnPredictor",
    "EnvironmentAnalyzer",

    # Models
    "AgentModel",
    "ModelInfo",

    # Strategies
    "StrategyGAN",
    "StrategyConfig",

    # Security
    "PrivacyManager",
    "EncryptionType",

    # Client/Server
    "MALLClient",
    "MALLServer",
]