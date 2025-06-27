# File: maple/ai_agent/config.py
# Description: Configuration management for AI Agent Service.
# Handles agent configuration, model settings, and environment variables.

from typing import Dict, Any, List, Optional
from dataclasses import dataclass, field
import os
import json
import yaml
from pathlib import Path
import logging

logger = logging.getLogger(__name__)


@dataclass
class ModelConfig:
    """Configuration for an individual model"""
    name: str
    provider: str
    api_key: Optional[str] = None
    endpoint: Optional[str] = None
    model_id: Optional[str] = None
    capabilities: List[str] = field(default_factory=list)
    max_tokens: int = 2000
    temperature: float = 0.7
    timeout: int = 30
    cost_per_1k_tokens: Dict[str, float] = field(
        default_factory=lambda: {"input": 0.001, "output": 0.002}
    )

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "ModelConfig":
        """Create from dictionary"""
        return cls(**data)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary"""
        return {
            "name": self.name,
            "provider": self.provider,
            "api_key": self.api_key,
            "endpoint": self.endpoint,
            "model_id": self.model_id,
            "capabilities": self.capabilities,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "timeout": self.timeout,
            "cost_per_1k_tokens": self.cost_per_1k_tokens
        }


@dataclass
class AIAgentConfig:
    """Main configuration for AI Agent"""
    agent_id: str
    name: str
    description: str = ""

    # Model configurations
    models: List[ModelConfig] = field(default_factory=list)
    fallback_models: List[str] = field(default_factory=list)

    # Selection strategy
    selection_strategy: str = "context_aware"
    selection_config: Dict[str, Any] = field(default_factory=dict)

    # Aggregation strategy
    aggregation_strategy: str = "weighted_average"
    aggregation_config: Dict[str, Any] = field(default_factory=dict)

    # Cache configuration
    cache_enabled: bool = True
    cache_strategy: str = "in_memory"
    cache_ttl: int = 3600
    cache_config: Dict[str, Any] = field(default_factory=dict)

    # Performance settings
    max_concurrent_queries: int = 10
    request_timeout: int = 60
    retry_attempts: int = 3

    # MAPLE integration
    map_endpoint: str = "http://localhost:8000"
    ars_endpoint: str = "http://localhost:8001"
    mall_endpoint: str = "http://localhost:8002"
    mapleverse_endpoint: Optional[str] = None

    # Monitoring
    monitoring_enabled: bool = True
    metrics_interval: int = 60
    alert_thresholds: Dict[str, float] = field(default_factory=dict)

    @classmethod
    def from_file(cls, filepath: str) -> "AIAgentConfig":
        """Load configuration from file"""

        path = Path(filepath)
        if not path.exists():
            raise FileNotFoundError(f"Config file not found: {filepath}")

        # Load file based on extension
        if path.suffix == ".json":
            with open(path) as f:
                data = json.load(f)
        elif path.suffix in [".yaml", ".yml"]:
            with open(path) as f:
                data = yaml.safe_load(f)
        else:
            raise ValueError(f"Unsupported config format: {path.suffix}")

        # Convert model configs
        if "models" in data:
            data["models"] = [
                ModelConfig.from_dict(m) if isinstance(m, dict) else m
                for m in data["models"]
            ]

        return cls(**data)

    @classmethod
    def from_env(cls) -> "AIAgentConfig":
        """Create configuration from environment variables"""

        config = cls(
            agent_id=os.getenv("MAPLE_AGENT_ID", "ai-agent-001"),
            name=os.getenv("MAPLE_AGENT_NAME", "AI Agent"),
            description=os.getenv("MAPLE_AGENT_DESC", "MAPLE AI Agent Service")
        )

        # Load models from environment
        model_count = int(os.getenv("MAPLE_MODEL_COUNT", "0"))
        for i in range(model_count):
            prefix = f"MAPLE_MODEL_{i}_"

            model = ModelConfig(
                name=os.getenv(f"{prefix}NAME", f"model-{i}"),
                provider=os.getenv(f"{prefix}PROVIDER", "openai"),
                api_key=os.getenv(f"{prefix}API_KEY"),
                endpoint=os.getenv(f"{prefix}ENDPOINT"),
                model_id=os.getenv(f"{prefix}MODEL_ID"),
                capabilities=os.getenv(
                    f"{prefix}CAPABILITIES",
                    "text_generation"
                ).split(",")
            )

            config.models.append(model)

        # Load other settings
        config.selection_strategy = os.getenv(
            "MAPLE_SELECTION_STRATEGY",
            "context_aware"
        )
        config.aggregation_strategy = os.getenv(
            "MAPLE_AGGREGATION_STRATEGY",
            "weighted_average"
        )
        config.cache_strategy = os.getenv("MAPLE_CACHE_STRATEGY", "in_memory")
        config.cache_ttl = int(os.getenv("MAPLE_CACHE_TTL", "3600"))

        # MAPLE endpoints
        config.map_endpoint = os.getenv(
            "MAPLE_MAP_ENDPOINT",
            "http://localhost:8000"
        )
        config.ars_endpoint = os.getenv(
            "MAPLE_ARS_ENDPOINT",
            "http://localhost:8001"
        )
        config.mall_endpoint = os.getenv(
            "MAPLE_MALL_ENDPOINT",
            "http://localhost:8002"
        )
        config.mapleverse_endpoint = os.getenv("MAPLE_MAPLEVERSE_ENDPOINT")

        return config

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary"""
        return {
            "agent_id": self.agent_id,
            "name": self.name,
            "description": self.description,
            "models": [m.to_dict() for m in self.models],
            "fallback_models": self.fallback_models,
            "selection_strategy": self.selection_strategy,
            "selection_config": self.selection_config,
            "aggregation_strategy": self.aggregation_strategy,
            "aggregation_config": self.aggregation_config,
            "cache_enabled": self.cache_enabled,
            "cache_strategy": self.cache_strategy,
            "cache_ttl": self.cache_ttl,
            "cache_config": self.cache_config,
            "max_concurrent_queries": self.max_concurrent_queries,
            "request_timeout": self.request_timeout,
            "retry_attempts": self.retry_attempts,
            "map_endpoint": self.map_endpoint,
            "ars_endpoint": self.ars_endpoint,
            "mall_endpoint": self.mall_endpoint,
            "mapleverse_endpoint": self.mapleverse_endpoint,
            "monitoring_enabled": self.monitoring_enabled,
            "metrics_interval": self.metrics_interval,
            "alert_thresholds": self.alert_thresholds
        }

    def save(self, filepath: str):
        """Save configuration to file"""

        path = Path(filepath)
        data = self.to_dict()

        # Save based on extension
        if path.suffix == ".json":
            with open(path, "w") as f:
                json.dump(data, f, indent=2)
        elif path.suffix in [".yaml", ".yml"]:
            with open(path, "w") as f:
                yaml.dump(data, f, default_flow_style=False)
        else:
            raise ValueError(f"Unsupported config format: {path.suffix}")

        logger.info(f"Saved configuration to {filepath}")

    def validate(self) -> List[str]:
        """Validate configuration and return errors"""

        errors = []

        # Check required fields
        if not self.agent_id:
            errors.append("agent_id is required")

        if not self.name:
            errors.append("name is required")

        # Check models
        if not self.models:
            errors.append("At least one model must be configured")

        for i, model in enumerate(self.models):
            if not model.name:
                errors.append(f"Model {i}: name is required")

            if not model.provider:
                errors.append(f"Model {i}: provider is required")

            # Provider-specific validation
            if model.provider in ["openai", "anthropic"] and not model.api_key:
                errors.append(f"Model {model.name}: API key required for {model.provider}")

            if model.provider == "local" and not model.endpoint:
                errors.append(f"Model {model.name}: endpoint required for local models")

        # Check strategies
        valid_selection_strategies = [
            "random", "round_robin", "performance_based",
            "context_aware", "cost_optimized", "latency_optimized"
        ]
        if self.selection_strategy not in valid_selection_strategies:
            errors.append(
                f"Invalid selection strategy: {self.selection_strategy}"
            )

        valid_aggregation_strategies = ["weighted_average", "majority_vote"]
        if self.aggregation_strategy not in valid_aggregation_strategies:
            errors.append(
                f"Invalid aggregation strategy: {self.aggregation_strategy}"
            )

        # Check cache
        valid_cache_strategies = ["in_memory", "redis"]
        if self.cache_strategy not in valid_cache_strategies:
            errors.append(f"Invalid cache strategy: {self.cache_strategy}")

        return errors


def create_default_config() -> AIAgentConfig:
    """Create default configuration"""

    return AIAgentConfig(
        agent_id="ai-agent-default",
        name="Default AI Agent",
        description="MAPLE AI Agent with default configuration",
        models=[
            ModelConfig(
                name="gpt-4",
                provider="openai",
                capabilities=["text_generation", "reasoning", "analysis"],
                max_tokens=2000,
                temperature=0.7
            ),
            ModelConfig(
                name="claude-3-sonnet",
                provider="anthropic",
                capabilities=["text_generation", "reasoning", "analysis"],
                max_tokens=2000,
                temperature=0.7
            )
        ],
        fallback_models=["gpt-3.5-turbo"],
        selection_strategy="context_aware",
        aggregation_strategy="weighted_average",
        cache_enabled=True,
        cache_strategy="in_memory",
        cache_ttl=3600,
        monitoring_enabled=True
    )