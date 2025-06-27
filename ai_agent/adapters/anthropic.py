# File: maple/ai_agent/adapters/anthropic.py
# Description: Anthropic adapter implementation for Claude models.
# Supports Claude 2, Claude 3, and future Anthropic models.

import aiohttp
import json
from typing import Dict, Any, List, Optional
import logging

from .base import LLMAdapter, ModelResponse

logger = logging.getLogger(__name__)


class AnthropicAdapter(LLMAdapter):
    """Adapter for Anthropic Claude models"""

    MODELS = {
        "claude-3-opus": {
            "capabilities": ["text_generation", "reasoning", "analysis", "coding", "vision"],
            "context_window": 200000,
            "cost_per_1k_tokens": {"input": 0.015, "output": 0.075}
        },
        "claude-3-sonnet": {
            "capabilities": ["text_generation", "reasoning", "analysis", "coding"],
            "context_window": 200000,
            "cost_per_1k_tokens": {"input": 0.003, "output": 0.015}
        },
        "claude-3-haiku": {
            "capabilities": ["text_generation", "reasoning", "analysis"],
            "context_window": 200000,
            "cost_per_1k_tokens": {"input": 0.00025, "output": 0.00125}
        },
        "claude-2.1": {
            "capabilities": ["text_generation", "reasoning", "analysis", "coding"],
            "context_window": 100000,
            "cost_per_1k_tokens": {"input": 0.008, "output": 0.024}
        }
    }

    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.model = config.get("model", "claude-3-sonnet")
        self.base_url = config.get(
            "base_url",
            "https://api.anthropic.com/v1"
        )
        self.anthropic_version = config.get(
            "anthropic_version",
            "2023-06-01"
        )

    async def _connect(self):
        """Initialize HTTP session for Anthropic API"""

        if not self.api_key:
            raise ValueError("Anthropic API key is required")

        self._session = aiohttp.ClientSession(
            headers={
                "x-api-key": self.api_key,
                "anthropic-version": self.anthropic_version,
                "Content-Type": "application/json"
            },
            timeout=aiohttp.ClientTimeout(total=self.timeout)
        )

    async def query(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[List[Dict[str, str]]] = None
    ) -> ModelResponse:
        """Query Anthropic model"""

        if not self._initialized:
            await self.initialize()

        # Convert messages to Anthropic format
        messages = self._convert_messages(prompt, context)

        params = {
            "model": self.model,
            "messages": messages,
            "max_tokens": parameters.get("max_tokens", 2000),
            "temperature": parameters.get("temperature", 0.7)
        }

        # Add system prompt if provided
        if parameters and "system" in parameters:
            params["system"] = parameters["system"]

        # Make request
        response_data = await self._retry_with_backoff(
            self._make_request,
            "messages",
            params
        )

        # Parse response
        content = response_data["content"][0]["text"]
        usage = response_data.get("usage", {})

        return ModelResponse(
            text=content,
            model=self.model,
            usage={
                "prompt_tokens": usage.get("input_tokens", 0),
                "completion_tokens": usage.get("output_tokens", 0),
                "total_tokens": usage.get("input_tokens", 0) +
                                usage.get("output_tokens", 0)
            },
            metadata={
                "stop_reason": response_data.get("stop_reason"),
                "model_version": response_data.get("model"),
                "id": response_data.get("id")
            },
            raw_response=response_data
        )

    async def stream_query(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[List[Dict[str, str]]] = None
    ):
        """Stream responses from Anthropic"""

        if not self._initialized:
            await self.initialize()

        messages = self._convert_messages(prompt, context)

        params = {
            "model": self.model,
            "messages": messages,
            "max_tokens": parameters.get("max_tokens", 2000),
            "temperature": parameters.get("temperature", 0.7),
            "stream": True
        }

        if parameters and "system" in parameters:
            params["system"] = parameters["system"]

        async with self._session.post(
                f"{self.base_url}/messages",
                json=params
        ) as response:
            response.raise_for_status()

            async for line in response.content:
                line = line.decode('utf-8').strip()
                if line.startswith("data: "):
                    data = line[6:]

                    try:
                        event = json.loads(data)
                        if event["type"] == "content_block_delta":
                            yield event["delta"]["text"]
                    except json.JSONDecodeError:
                        continue

    def _convert_messages(
            self,
            prompt: str,
            context: Optional[List[Dict[str, str]]] = None
    ) -> List[Dict[str, str]]:
        """Convert messages to Anthropic format"""

        messages = []

        # Add context
        if context:
            for msg in context:
                # Skip system messages as they go in a separate field
                if msg["role"] != "system":
                    messages.append({
                        "role": msg["role"],
                        "content": msg["content"]
                    })

        # Add current prompt
        messages.append({
            "role": "user",
            "content": prompt
        })

        return messages

    async def _make_request(
            self,
            endpoint: str,
            data: Dict[str, Any]
    ) -> Dict[str, Any]:
        """Make API request to Anthropic"""

        url = f"{self.base_url}/{endpoint}"

        async with self._session.post(url, json=data) as response:
            response_data = await response.json()

            if response.status != 200:
                error_msg = response_data.get(
                    "error",
                    {}).get("message", "Unknown error")
                raise Exception(f"Anthropic API error: {error_msg}")

            return response_data

    def get_capabilities(self) -> List[str]:
        """Get model capabilities"""
        return self.MODELS.get(self.model, {}).get("capabilities", [])

    def get_model_info(self) -> Dict[str, Any]:
        """Get model information"""
        model_info = self.MODELS.get(self.model, {})

        return {
            "name": self.model,
            "provider": "anthropic",
            "capabilities": model_info.get("capabilities", []),
            "context_window": model_info.get("context_window", 0),
            "cost_per_1k_tokens": model_info.get("cost_per_1k_tokens", {}),
            "supports_streaming": True,
            "supports_system_prompt": True,
            "supports_vision": "vision" in model_info.get("capabilities", [])
        }

    async def close(self):
        """Close HTTP session"""
        if hasattr(self, "_session") and self._session:
            await self._session.close()
        await super().close()