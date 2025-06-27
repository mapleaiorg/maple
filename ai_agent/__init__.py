# File: maple/ai_agent/__init__.py
# Description: Main package initialization for MAPLE AI Agent Service.
# This module provides the core AI Agent architecture for connecting to
# multiple LLMs/AGIs while integrating with MAPLE's ecosystem.

from .core.agent import AIAgent, AgentCore
from .core.model_selector import ModelSelector, ModelSelectionStrategy
from .adapters.base import LLMAdapter, AdapterRegistry
from .adapters.openai import OpenAIAdapter
from .adapters.anthropic import AnthropicAdapter
from .adapters.local import LocalModelAdapter
from .aggregation.ensemble import EnsembleAggregator
from .aggregation.strategies import WeightedAverageStrategy, MajorityVoteStrategy
from .config import AIAgentConfig, ModelConfig
from .cache import ResponseCache, CacheStrategy
from .monitoring import AgentMonitor, PerformanceMetrics

__version__ = "0.1.0"

__all__ = [
    "AIAgent",
    "AgentCore",
    "ModelSelector",
    "ModelSelectionStrategy",
    "LLMAdapter",
    "AdapterRegistry",
    "OpenAIAdapter",
    "AnthropicAdapter",
    "LocalModelAdapter",
    "EnsembleAggregator",
    "WeightedAverageStrategy",
    "MajorityVoteStrategy",
    "AIAgentConfig",
    "ModelConfig",
    "ResponseCache",
    "CacheStrategy",
    "AgentMonitor",
    "PerformanceMetrics"
]