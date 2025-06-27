# File: maple/mall/client/mall_client.py
# Description: Client SDK for interacting with MALL services.

from __future__ import annotations
from dataclasses import dataclass
from typing import Dict, List, Optional, Any, AsyncIterator
import asyncio
import aiohttp
import json
import logging
from datetime import datetime

from maple.mall.models.agent_model import AgentModel, ModelInfo
from maple.mall.spawn.auto_spawner import SpawnRequest

logger = logging.getLogger(__name__)


@dataclass
class MALLClientConfig:
    """Configuration for MALL client"""
    base_url: str = "http://localhost:8080"
    api_key: Optional[str] = None
    timeout: int = 30
    max_retries: int = 3
    verify_ssl: bool = True


class MALLClient:
    """
    Client SDK for MALL operations.
    Provides high-level interface for training, spawning, and managing agents.
    """

    def __init__(self, config: MALLClientConfig):
        self.config = config
        self.session: Optional[aiohttp.ClientSession] = None
        self._connected = False

        # Headers
        self.headers = {
            "Content-Type": "application/json",
            "User-Agent": "MALL-Client/1.0"
        }

        if config.api_key:
            self.headers["Authorization"] = f"Bearer {config.api_key}"

    async def connect(self) -> None:
        """Connect to MALL server"""
        if self._connected:
            return

        self.session = aiohttp.ClientSession(
            timeout=aiohttp.ClientTimeout(total=self.config.timeout),
            headers=self.headers
        )

        # Verify connection
        await self.health_check()
        self._connected = True
        logger.info(f"Connected to MALL at {self.config.base_url}")

    async def disconnect(self) -> None:
        """Disconnect from MALL server"""
        if self.session:
            await self.session.close()
        self._connected = False
        logger.info("Disconnected from MALL")

    async def health_check(self) -> Dict[str, Any]:
        """Check MALL server health"""
        async with self.session.get(f"{self.config.base_url}/health") as resp:
            resp.raise_for_status()
            return await resp.json()

    # Training operations

    async def train_federated(
            self,
            model: AgentModel,
            task_type: str,
            config: Optional[Dict[str, Any]] = None
    ) -> str:
        """Start federated training round"""
        data = {
            "model_id": model.model_id,
            "model_state": model.state_dict(),  # Would serialize properly
            "task_type": task_type,
            "config": config or {}
        }

        async with self.session.post(
                f"{self.config.base_url}/training/federated",
                json=data
        ) as resp:
            resp.raise_for_status()
            result = await resp.json()
            return result["round_id"]

    async def get_training_status(self, round_id: str) -> Dict[str, Any]:
        """Get training round status"""
        async with self.session.get(
                f"{self.config.base_url}/training/federated/{round_id}"
        ) as resp:
            resp.raise_for_status()
            return await resp.json()

    async def get_trained_model(self, round_id: str) -> AgentModel:
        """Get model after training completes"""
        async with self.session.get(
                f"{self.config.base_url}/training/federated/{round_id}/model"
        ) as resp:
            resp.raise_for_status()
            data = await resp.json()
            # Would deserialize to AgentModel
            return data

    # Environment sensing

    async def sense_environment(self, shard_id: str) -> Dict[str, Any]:
        """Get environment data from shard"""
        async with self.session.get(
                f"{self.config.base_url}/environment/{shard_id}/sense"
        ) as resp:
            resp.raise_for_status()
            return await resp.json()

    async def get_environment_predictions(
            self,
            shard_id: str,
            horizon: int = 300
    ) -> Dict[str, Any]:
        """Get environment predictions"""
        params = {"horizon": horizon}
        async with self.session.get(
                f"{self.config.base_url}/environment/{shard_id}/predict",
                params=params
        ) as resp:
            resp.raise_for_status()
            return await resp.json()

        # Auto-spawning

    async def predict_spawn_need(
            self,
            env_data: Dict[str, Any],
            task_type: str
    ) -> Dict[str, Any]:
        """Predict if new agents should be spawned"""
        data = {
            "environment_data": env_data,
            "task_type": task_type
        }

        async with self.session.post(
                f"{self.config.base_url}/spawn/predict",
                json=data
        ) as resp:
            resp.raise_for_status()
            return await resp.json()

    async def spawn_agent(self, spawn_request: SpawnRequest) -> str:
        """Spawn a new agent"""
        data = {
            "agent_id": spawn_request.agent_id,
            "template_name": spawn_request.template_name,
            "capabilities": spawn_request.capabilities,
            "configuration": spawn_request.configuration,
            "parent_agent": spawn_request.parent_agent,
            "priority": spawn_request.priority,
            "metadata": spawn_request.metadata
        }

        async with self.session.post(
                f"{self.config.base_url}/spawn/agent",
                json=data
        ) as resp:
            resp.raise_for_status()
            result = await resp.json()
            return result["agent_id"]

    async def despawn_agent(self, agent_id: str) -> bool:
        """Despawn an agent"""
        async with self.session.delete(
                f"{self.config.base_url}/spawn/agent/{agent_id}"
        ) as resp:
            resp.raise_for_status()
            result = await resp.json()
            return result["success"]

        # Strategy generation

    async def generate_strategy(
            self,
            strategy_type: str,
            constraints: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """Generate new strategy using GAN"""
        data = {
            "strategy_type": strategy_type,
            "constraints": constraints or {}
        }

        async with self.session.post(
                f"{self.config.base_url}/strategies/generate",
                json=data
        ) as resp:
            resp.raise_for_status()
            return await resp.json()

    async def evaluate_strategy(
            self,
            strategy: Dict[str, Any]
    ) -> float:
        """Evaluate strategy quality"""
        async with self.session.post(
                f"{self.config.base_url}/strategies/evaluate",
                json={"strategy": strategy}
        ) as resp:
            resp.raise_for_status()
            result = await resp.json()
            return result["score"]

        # Model management

    async def list_models(
            self,
            agent_type: Optional[str] = None,
            limit: int = 100
    ) -> List[ModelInfo]:
        """List available models"""
        params = {"limit": limit}
        if agent_type:
            params["agent_type"] = agent_type

        async with self.session.get(
                f"{self.config.base_url}/models",
                params=params
        ) as resp:
            resp.raise_for_status()
            data = await resp.json()
            # Would convert to ModelInfo objects
            return data["models"]

    async def get_model(self, model_id: str) -> ModelInfo:
        """Get model information"""
        async with self.session.get(
                f"{self.config.base_url}/models/{model_id}"
        ) as resp:
            resp.raise_for_status()
            data = await resp.json()
            # Would convert to ModelInfo
            return data

    async def upload_model(
            self,
            model: AgentModel,
            metadata: Optional[Dict[str, Any]] = None
    ) -> str:
        """Upload model to MALL"""
        data = {
            "model_id": model.model_id,
            "model_type": model.model_type.value,
            "model_state": model.state_dict(),  # Would serialize
            "metadata": metadata or {}
        }

        async with self.session.post(
                f"{self.config.base_url}/models",
                json=data
        ) as resp:
            resp.raise_for_status()
            result = await resp.json()
            return result["model_id"]

    async def download_model(self, model_id: str) -> AgentModel:
        """Download model from MALL"""
        async with self.session.get(
                f"{self.config.base_url}/models/{model_id}/download"
        ) as resp:
            resp.raise_for_status()
            data = await resp.json()
            # Would deserialize to AgentModel
            return data

        # Learning metrics

    async def get_learning_metrics(
            self,
            shard_id: Optional[str] = None
    ) -> Dict[str, Any]:
        """Get learning metrics"""
        params = {}
        if shard_id:
            params["shard_id"] = shard_id

        async with self.session.get(
                f"{self.config.base_url}/metrics/learning",
                params=params
        ) as resp:
            resp.raise_for_status()
            return await resp.json()

    async def get_spawn_metrics(self) -> Dict[str, Any]:
        """Get auto-spawn metrics"""
        async with self.session.get(
                f"{self.config.base_url}/metrics/spawn"
        ) as resp:
            resp.raise_for_status()
            return await resp.json()

        # Streaming operations

    async def stream_training_logs(
            self,
            round_id: str
    ) -> AsyncIterator[Dict[str, Any]]:
        """Stream training logs in real-time"""
        async with self.session.get(
                f"{self.config.base_url}/training/federated/{round_id}/logs/stream"
        ) as resp:
            resp.raise_for_status()

            async for line in resp.content:
                if line:
                    yield json.loads(line)

    async def stream_environment_updates(
            self,
            shard_id: str
    ) -> AsyncIterator[Dict[str, Any]]:
        """Stream environment updates"""
        async with self.session.get(
                f"{self.config.base_url}/environment/{shard_id}/stream"
        ) as resp:
            resp.raise_for_status()

            async for line in resp.content:
                if line:
                    yield json.loads(line)

        # Batch operations

    async def batch_train(
            self,
            training_requests: List[Dict[str, Any]]
    ) -> List[str]:
        """Submit multiple training requests"""
        data = {"requests": training_requests}

        async with self.session.post(
                f"{self.config.base_url}/training/batch",
                json=data
        ) as resp:
            resp.raise_for_status()
            result = await resp.json()
            return result["round_ids"]

    async def batch_spawn(
            self,
            spawn_requests: List[SpawnRequest]
    ) -> List[str]:
        """Spawn multiple agents"""
        data = {
            "requests": [
                {
                    "agent_id": req.agent_id,
                    "template_name": req.template_name,
                    "capabilities": req.capabilities,
                    "configuration": req.configuration,
                    "parent_agent": req.parent_agent,
                    "metadata": req.metadata
                }
                for req in spawn_requests
            ]
        }

        async with self.session.post(
                f"{self.config.base_url}/spawn/batch",
                json=data
        ) as resp:
            resp.raise_for_status()
            result = await resp.json()
            return result["agent_ids"]

        # Helper methods

    async def wait_for_training(
            self,
            round_id: str,
            poll_interval: float = 5.0
    ) -> Dict[str, Any]:
        """Wait for training to complete"""
        while True:
            status = await self.get_training_status(round_id)

            if status["status"] in ["completed", "failed"]:
                return status

            await asyncio.sleep(poll_interval)

    async def __aenter__(self):
        """Async context manager entry"""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit"""
        await self.disconnect()

    # Convenience functions

    async def create_mall_client(
            url: str = "http://localhost:8080",
            api_key: Optional[str] = None
    ) -> MALLClient:
        """Create and connect MALL client"""
        config = MALLClientConfig(base_url=url, api_key=api_key)
        client = MALLClient(config)
        await client.connect()
        return client