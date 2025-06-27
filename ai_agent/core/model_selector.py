# File: maple/ai_agent/core/model_selector.py
# Description: Intelligent model selection using reinforcement learning
# and context-aware strategies to choose the best LLM/AGI for tasks.

import asyncio
from enum import Enum
from typing import List, Dict, Any, Optional
import numpy as np
from dataclasses import dataclass
import logging

from mall.client import MALLClient

logger = logging.getLogger(__name__)


class ModelSelectionStrategy(Enum):
    """Model selection strategies"""
    RANDOM = "random"
    ROUND_ROBIN = "round_robin"
    PERFORMANCE_BASED = "performance_based"
    CONTEXT_AWARE = "context_aware"
    COST_OPTIMIZED = "cost_optimized"
    LATENCY_OPTIMIZED = "latency_optimized"


@dataclass
class ModelProfile:
    """Profile for an LLM/AGI model"""
    name: str
    capabilities: List[str]
    performance_metrics: Dict[str, float]
    cost_per_token: float
    average_latency: float
    specializations: List[str]

    def score_for_task(self, task_type: str, requirements: Dict[str, Any]) -> float:
        """Calculate model score for a specific task"""
        score = 0.0

        # Check capability match
        if task_type in self.capabilities:
            score += 1.0

        # Check specialization bonus
        if task_type in self.specializations:
            score += 0.5

        # Factor in performance
        if task_type in self.performance_metrics:
            score += self.performance_metrics[task_type]

        # Latency penalty
        if requirements.get("max_latency"):
            if self.average_latency <= requirements["max_latency"]:
                score += 0.3
            else:
                score -= 0.5

        # Cost consideration
        if requirements.get("cost_sensitive"):
            score -= self.cost_per_token * 0.1

        return max(0.0, score)


class ModelSelector:
    """Intelligent model selection with MALL integration"""

    def __init__(
            self,
            strategy: ModelSelectionStrategy = ModelSelectionStrategy.CONTEXT_AWARE,
            mall_client: Optional[MALLClient] = None
    ):
        self.strategy = strategy
        self.mall_client = mall_client

        # Model profiles cache
        self.model_profiles: Dict[str, ModelProfile] = {}

        # Selection state
        self.round_robin_index = 0
        self.selection_history = []

        # RL components for context-aware selection
        self.q_table = {}  # State-action values
        self.epsilon = 0.1  # Exploration rate
        self.learning_rate = 0.1
        self.discount_factor = 0.9

    async def select_models(
            self,
            task: Dict[str, Any],
            context: Any,
            available_models: List[str],
            num_models: int = 1
    ) -> List[str]:
        """Select best models for the task"""

        # Update model profiles
        await self._update_model_profiles(available_models)

        # Apply selection strategy
        if self.strategy == ModelSelectionStrategy.RANDOM:
            selected = self._select_random(available_models, num_models)

        elif self.strategy == ModelSelectionStrategy.ROUND_ROBIN:
            selected = self._select_round_robin(available_models, num_models)

        elif self.strategy == ModelSelectionStrategy.PERFORMANCE_BASED:
            selected = await self._select_performance_based(
                task, available_models, num_models
            )

        elif self.strategy == ModelSelectionStrategy.CONTEXT_AWARE:
            selected = await self._select_context_aware(
                task, context, available_models, num_models
            )

        elif self.strategy == ModelSelectionStrategy.COST_OPTIMIZED:
            selected = self._select_cost_optimized(
                task, available_models, num_models
            )

        elif self.strategy == ModelSelectionStrategy.LATENCY_OPTIMIZED:
            selected = self._select_latency_optimized(
                task, available_models, num_models
            )

        else:
            # Default to random
            selected = self._select_random(available_models, num_models)

        # Record selection
        self.selection_history.append({
            "task": task,
            "selected_models": selected,
            "strategy": self.strategy.value
        })

        return selected

    def _select_random(self, models: List[str], num: int) -> List[str]:
        """Random selection"""
        import random
        return random.sample(models, min(num, len(models)))

    def _select_round_robin(self, models: List[str], num: int) -> List[str]:
        """Round-robin selection"""
        selected = []
        for _ in range(min(num, len(models))):
            selected.append(models[self.round_robin_index % len(models)])
            self.round_robin_index += 1
        return selected

    async def _select_performance_based(
            self,
            task: Dict[str, Any],
            models: List[str],
            num: int
    ) -> List[str]:
        """Select based on historical performance"""

        task_type = task.get("type", "general")

        # Score each model
        model_scores = []
        for model in models:
            profile = self.model_profiles.get(model)
            if profile:
                score = profile.score_for_task(
                    task_type,
                    task.get("requirements", {})
                )
            else:
                score = 0.5  # Default score

            model_scores.append((model, score))

        # Sort by score and select top N
        model_scores.sort(key=lambda x: x[1], reverse=True)
        return [model for model, _ in model_scores[:num]]

    async def _select_context_aware(
            self,
            task: Dict[str, Any],
            context: Any,
            models: List[str],
            num: int
    ) -> List[str]:
        """Context-aware selection using RL"""

        # Extract state features
        state = self._extract_state_features(task, context)
        state_key = self._hash_state(state)

        # Initialize Q-values if not seen
        if state_key not in self.q_table:
            self.q_table[state_key] = {
                model: 0.0 for model in models
            }

        # Epsilon-greedy selection
        import random
        if random.random() < self.epsilon:
            # Explore
            selected = self._select_random(models, num)
        else:
            # Exploit
            q_values = self.q_table[state_key]
            sorted_models = sorted(
                models,
                key=lambda m: q_values.get(m, 0.0),
                reverse=True
            )
            selected = sorted_models[:num]

        return selected

    def _select_cost_optimized(
            self,
            task: Dict[str, Any],
            models: List[str],
            num: int
    ) -> List[str]:
        """Select models optimizing for cost"""

        # Filter by capability first
        task_type = task.get("type", "general")
        capable_models = [
            m for m in models
            if self.model_profiles.get(m) and
               task_type in self.model_profiles[m].capabilities
        ]

        # Sort by cost
        capable_models.sort(
            key=lambda m: self.model_profiles[m].cost_per_token
        )

        return capable_models[:num]

    def _select_latency_optimized(
            self,
            task: Dict[str, Any],
            models: List[str],
            num: int
    ) -> List[str]:
        """Select models optimizing for latency"""

        # Filter by capability first
        task_type = task.get("type", "general")
        capable_models = [
            m for m in models
            if self.model_profiles.get(m) and
               task_type in self.model_profiles[m].capabilities
        ]

        # Sort by latency
        capable_models.sort(
            key=lambda m: self.model_profiles[m].average_latency
        )

        return capable_models[:num]

    async def _update_model_profiles(self, models: List[str]):
        """Update model profiles from MALL"""

        if not self.mall_client:
            return

        for model in models:
            if model not in self.model_profiles:
                # Fetch profile from MALL
                profile_data = await self.mall_client.get_model_profile(model)

                if profile_data:
                    self.model_profiles[model] = ModelProfile(
                        name=model,
                        capabilities=profile_data.get("capabilities", []),
                        performance_metrics=profile_data.get("performance", {}),
                        cost_per_token=profile_data.get("cost_per_token", 0.001),
                        average_latency=profile_data.get("avg_latency", 100),
                        specializations=profile_data.get("specializations", [])
                    )

    async def update_from_interaction(self, interaction_data: Dict[str, Any]):
        """Update selection strategy from interaction results"""

        if self.strategy != ModelSelectionStrategy.CONTEXT_AWARE:
            return

        # Extract relevant data
        task = interaction_data["task"]
        model_responses = interaction_data["model_responses"]

        # Calculate rewards based on response quality
        state = self._extract_state_features(task, None)
        state_key = self._hash_state(state)

        for response in model_responses:
            model = response["model"]

            # Calculate reward (could be based on accuracy, latency, etc.)
            reward = self._calculate_reward(response)

            # Update Q-value
            if state_key in self.q_table:
                old_value = self.q_table[state_key].get(model, 0.0)

                # Q-learning update
                self.q_table[state_key][model] = old_value + self.learning_rate * (
                        reward + self.discount_factor *
                        max(self.q_table[state_key].values()) -old_value
                )

        # Decay exploration rate
        self.epsilon = max(0.01, self.epsilon * 0.995)

    def _extract_state_features(
        self,
        task: Dict[str, Any],
        context: Any
    ) -> Dict[str, Any]:
        """Extract state features for RL"""

        features = {
            "task_type": task.get("type", "general"),
            "prompt_length": len(task.get("prompt", "")),
            "has_context": context is not None,
            "requires_reasoning": "reasoning" in task.get("prompt", "").lower(),
            "requires_creativity": any(
                word in task.get("prompt", "").lower()
                for word in ["create", "generate", "imagine", "design"]
            ),
            "time_of_day": datetime.now().hour // 6,  # 0-3 for time periods
        }

        # Add context features if available
        if context and hasattr(context, "conversation_history"):
            features["conversation_length"] = len(context.conversation_history)
            features["has_code"] = any(
                "```" in turn.get("content", "")
                for turn in context.conversation_history
            )

        return features

    def _hash_state(self, state: Dict[str, Any]) -> str:
        """Create hashable state representation"""
        import hashlib
        import json

        state_str = json.dumps(state, sort_keys=True)
        return hashlib.md5(state_str.encode()).hexdigest()

    def _calculate_reward(self, response: Dict[str, Any]) -> float:
        """Calculate reward for a model response"""

        reward = 0.0

        # Success bonus
        if response.get("success"):
            reward += 1.0
        else:
            return -1.0  # Failure penalty

        # Latency penalty (normalized)
        latency = response.get("latency", 1.0)
        reward -= min(1.0, latency / 5000)  # 5 second threshold

        # Response quality (would need actual evaluation in production)
        # For now, use response length as proxy
        response_text = response.get("response", {}).get("text", "")
        if len(response_text) > 50:
            reward += 0.2

        return reward