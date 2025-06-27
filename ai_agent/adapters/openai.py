# File: maple/ai_agent/adapters/openai.py
# Description: OpenAI adapter implementation for GPT models.
# Supports GPT-3.5, GPT-4, and future OpenAI models.

import aiohttp
import json
from typing import Dict, Any, List, Optional
import logging
from datetime import datetime

from .base import LLMAdapter, ModelResponse

logger = logging.getLogger(__name__)


class OpenAIAdapter(LLMAdapter):
    """Adapter for OpenAI GPT models"""

    MODELS = {
        "gpt-4": {
            "capabilities": ["text_generation", "reasoning", "analysis", "coding"],
            "context_window": 8192,
            "cost_per_1k_tokens": {"input": 0.03, "output": 0.06}
        },
        "gpt-4-turbo": {
            "capabilities": ["text_generation", "reasoning", "analysis", "coding", "vision"],
            "context_window": 128000,
            "cost_per_1k_tokens": {"input": 0.01, "output": 0.03}
        },
        "gpt-3.5-turbo": {
            "capabilities": ["text_generation", "reasoning", "analysis"],
            "context_window": 16384,
            "cost_per_1k_tokens": {"input": 0.0005, "output": 0.0015}
        }
    }

    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.model = config.get("model", "gpt-4")
        self.base_url = config.get(
            "base_url",
            "https://api.openai.com/v1"
        )

        # Validate model
        if self.model not in self.MODELS:
            logger.warning(
                f"Unknown model {self.model}, using gpt-4 as default"
            )
            self.model = "gpt-4"

    async def _connect(self):
        """Initialize HTTP session for OpenAI API"""

        if not self.api_key:
            raise ValueError("OpenAI API key is required")

        # Create session with auth headers
        self._session = aiohttp.ClientSession(
            headers={
                "Authorization": f"Bearer {self.api_key}",
                "Content-Type": "application/json"
            },
            timeout=aiohttp.ClientTimeout(total=self.timeout)
        )

        # Test connection
        if not await self.health_check():
            raise ConnectionError("Failed to connect to OpenAI API")

    async def query(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[List[Dict[str, str]]] = None
    ) -> ModelResponse:
        """Query OpenAI model"""

        if not self._initialized:
            await self.initialize()

        # Build request
        messages = self._build_messages(prompt, context)

        params = {
            "model": self.model,
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 2000,
            "top_p": 1.0,
            "frequency_penalty": 0.0,
            "presence_penalty": 0.0
        }

        # Override with custom parameters
        if parameters:
            params.update(parameters)

        # Make request with retry
        response_data = await self._retry_with_backoff(
            self._make_request,
            "chat/completions",
            params
        )

        # Parse response
        choice = response_data["choices"][0]
        usage = response_data["usage"]

        return ModelResponse(
            text=choice["message"]["content"],
            model=self.model,
            usage={
                "prompt_tokens": usage["prompt_tokens"],
                "completion_tokens": usage["completion_tokens"],
                "total_tokens": usage["total_tokens"]
            },
            metadata={
                "finish_reason": choice["finish_reason"],
                "model_version": response_data.get("model"),
                "created": response_data.get("created")
            },
            raw_response=response_data
        )

    async def stream_query(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[List[Dict[str, str]]] = None
    ):
        """Stream responses from OpenAI"""

        if not self._initialized:
            await self.initialize()

        messages = self._build_messages(prompt, context)

        params = {
            "model": self.model,
            "messages": messages,
            "stream": True,
            "temperature": 0.7,
            "max_tokens": 2000
        }

        if parameters:
            params.update(parameters)

        # Make streaming request
        async with self._session.post(
                f"{self.base_url}/chat/completions",
                json=params
        ) as response:
            response.raise_for_status()

            async for line in response.content:
                line = line.decode('utf-8').strip()
                if line.startswith("data: "):
                    data = line[6:]
                    if data == "[DONE]":
                        break

                    try:
                        chunk = json.loads(data)
                        delta = chunk["choices"][0].get("delta", {})
                        if "content" in delta:
                            yield delta["content"]
                    except json.JSONDecodeError:
                        continue

    async def _make_request(
            self,
            endpoint: str,
            data: Dict[str, Any]
    ) -> Dict[str, Any]:
        """Make API request to OpenAI"""

        url = f"{self.base_url}/{endpoint}"

        async with self._session.post(url, json=data) as response:
            response_data = await response.json()

            if response.status != 200:
                error_msg = response_data.get(
                    "error",
                    {}).get("message", "Unknown error")
                raise Exception(f"OpenAI API error: {error_msg}")

            return response_data

    def get_capabilities(self) -> List[str]:
        """Get model capabilities"""
        return self.MODELS.get(self.model, {}).get("capabilities", [])

    def get_model_info(self) -> Dict[str, Any]:
        """Get model information"""
        model_info = self.MODELS.get(self.model, {})

        return {
            "name": self.model,
            "provider": "openai",
            "capabilities": model_info.get("capabilities", []),
            "context_window": model_info.get("context_window", 0),
            "cost_per_1k_tokens": model_info.get("cost_per_1k_tokens", {}),
            "supports_streaming": True,
            "supports_functions": self.model.startswith("gpt-4"),
            "supports_vision": "vision" in model_info.get("capabilities", [])
        }

    async def close(self):
        """Close HTTP session"""
        if hasattr(self, "_session") and self._session:
            await self._session.close()
        await super().close()