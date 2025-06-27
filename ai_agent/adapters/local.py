# File: maple/ai_agent/adapters/local.py
# Description: Adapter for local/self-hosted models like LLaMA, Mistral, etc.
# Supports models running via Ollama, vLLM, or custom inference servers.

import aiohttp
import json
from typing import Dict, Any, List, Optional
import logging

from .base import LLMAdapter, ModelResponse

logger = logging.getLogger(__name__)


class LocalModelAdapter(LLMAdapter):
    """Adapter for local/self-hosted models"""

    SUPPORTED_BACKENDS = ["ollama", "vllm", "custom"]

    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)

        self.backend = config.get("backend", "ollama")
        self.model = config.get("model", "llama2")

        # Backend-specific endpoints
        if self.backend == "ollama":
            self.endpoint = config.get("endpoint", "http://localhost:11434")
        elif self.backend == "vllm":
            self.endpoint = config.get("endpoint", "http://localhost:8000")
        else:
            if not self.endpoint:
                raise ValueError("Endpoint required for custom backend")

        self.model_info = config.get("model_info", {})

    async def _connect(self):
        """Initialize HTTP session for local model"""

        self._session = aiohttp.ClientSession(
            timeout=aiohttp.ClientTimeout(total=self.timeout)
        )

        # Test connection
        if not await self.health_check():
            raise ConnectionError(
                f"Failed to connect to {self.backend} at {self.endpoint}"
            )

    async def query(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[List[Dict[str, str]]] = None
    ) -> ModelResponse:
        """Query local model"""

        if not self._initialized:
            await self.initialize()

        if self.backend == "ollama":
            response_data = await self._query_ollama(
                prompt, parameters, context
            )
        elif self.backend == "vllm":
            response_data = await self._query_vllm(
                prompt, parameters, context
            )
        else:
            response_data = await self._query_custom(
                prompt, parameters, context
            )

        return self._parse_response(response_data)

    async def _query_ollama(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[List[Dict[str, str]]] = None
    ) -> Dict[str, Any]:
        """Query Ollama backend"""

        # Build prompt with context
        full_prompt = self._build_full_prompt(prompt, context)

        data = {
            "model": self.model,
            "prompt": full_prompt,
            "stream": False,
            "options": {
                "temperature": parameters.get("temperature", 0.7) if parameters else 0.7,
                "top_p": parameters.get("top_p", 0.9) if parameters else 0.9,
                "num_predict": parameters.get("max_tokens", 2000) if parameters else 2000
            }
        }

        async with self._session.post(
                f"{self.endpoint}/api/generate",
                json=data
        ) as response:
            response.raise_for_status()
            return await response.json()

    async def _query_vllm(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[List[Dict[str, str]]] = None
    ) -> Dict[str, Any]:
        """Query vLLM backend"""

        messages = self._build_messages(prompt, context)

        data = {
            "model": self.model,
            "messages": messages,
            "temperature": parameters.get("temperature", 0.7) if parameters else 0.7,
            "max_tokens": parameters.get("max_tokens", 2000) if parameters else 2000,
            "top_p": parameters.get("top_p", 0.9) if parameters else 0.9
        }

        async with self._session.post(
                f"{self.endpoint}/v1/chat/completions",
                json=data
        ) as response:
            response.raise_for_status()
            return await response.json()

    async def _query_custom(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[List[Dict[str, str]]] = None
    ) -> Dict[str, Any]:
        """Query custom backend"""

        # For custom backends, send raw data
        data = {
            "prompt": prompt,
            "context": context,
            "parameters": parameters or {}
        }

        async with self._session.post(
                f"{self.endpoint}/generate",
                json=data
        ) as response:
            response.raise_for_status()
            return await response.json()

    def _parse_response(self, response_data: Dict[str, Any]) -> ModelResponse:
        """Parse response based on backend type"""

        if self.backend == "ollama":
            return ModelResponse(
                text=response_data.get("response", ""),
                model=self.model,
                usage={
                    "prompt_tokens": response_data.get("prompt_eval_count", 0),
                    "completion_tokens": response_data.get("eval_count", 0),
                    "total_tokens": response_data.get("prompt_eval_count", 0) +
                                    response_data.get("eval_count", 0)
                },
                metadata={
                    "eval_duration": response_data.get("eval_duration"),
                    "model_version": response_data.get("model")
                },
                raw_response=response_data
            )

        elif self.backend == "vllm":
            choice = response_data["choices"][0]
            usage = response_data.get("usage", {})

            return ModelResponse(
                text=choice["message"]["content"],
                model=self.model,
                usage={
                    "prompt_tokens": usage.get("prompt_tokens", 0),
                    "completion_tokens": usage.get("completion_tokens", 0),
                    "total_tokens": usage.get("total_tokens", 0)
                },
                metadata={
                    "finish_reason": choice.get("finish_reason"),
                    "model_version": response_data.get("model")
                },
                raw_response=response_data
            )

        else:
            # Custom backend - expect standard format
            return ModelResponse(
                text=response_data.get("text", ""),
                model=self.model,
                usage=response_data.get("usage", {}),
                metadata=response_data.get("metadata", {}),
                raw_response=response_data
            )

    def _build_full_prompt(
        self,
        prompt: str,
        context: Optional[List[Dict[str, str]]] = None
    ) -> str:
        """Build full prompt string for Ollama"""

        full_prompt = ""

        if context:
            for msg in context:
                role = msg["role"]
                content = msg["content"]

                if role == "system":
                    full_prompt += f"System: {content}\n\n"
                elif role == "user":
                    full_prompt += f"User: {content}\n\n"
                elif role == "assistant":
                    full_prompt += f"Assistant: {content}\n\n"

        full_prompt += f"User: {prompt}\n\nAssistant: "

        return full_prompt

    async def stream_query(
        self,
        prompt: str,
        parameters: Optional[Dict[str, Any]] = None,
        context: Optional[List[Dict[str, str]]] = None
    ):
        """Stream responses from local model"""

        if not self._initialized:
            await self.initialize()

        if self.backend == "ollama":
            async for chunk in self._stream_ollama(prompt, parameters, context):
                yield chunk
        elif self.backend == "vllm":
            async for chunk in self._stream_vllm(prompt, parameters, context):
                yield chunk
        else:
            # Custom backend doesn't support streaming by default
            response = await self.query(prompt, parameters, context)
            yield response.text

    async def _stream_ollama(
        self,
        prompt: str,
        parameters: Optional[Dict[str, Any]] = None,
        context: Optional[List[Dict[str, str]]] = None
    ):
        """Stream from Ollama"""

        full_prompt = self._build_full_prompt(prompt, context)

        data = {
            "model": self.model,
            "prompt": full_prompt,
            "stream": True,
            "options": {
                "temperature": parameters.get("temperature", 0.7) if parameters else 0.7,
                "num_predict": parameters.get("max_tokens", 2000) if parameters else 2000
            }
        }

        async with self._session.post(
            f"{self.endpoint}/api/generate",
            json=data
        ) as response:
            async for line in response.content:
                try:
                    chunk = json.loads(line)
                    if "response" in chunk:
                        yield chunk["response"]
                except json.JSONDecodeError:
                    continue

    async def _stream_vllm(
        self,
        prompt: str,
        parameters: Optional[Dict[str, Any]] = None,
        context: Optional[List[Dict[str, str]]] = None
    ):
        """Stream from vLLM"""

        messages = self._build_messages(prompt, context)

        data = {
            "model": self.model,
            "messages": messages,
            "stream": True,
            "temperature": parameters.get("temperature", 0.7) if parameters else 0.7,
            "max_tokens": parameters.get("max_tokens", 2000) if parameters else 2000
        }

        async with self._session.post(
            f"{self.endpoint}/v1/chat/completions",
            json=data
        ) as response:
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

    def get_capabilities(self) -> List[str]:
        """Get model capabilities"""
        return self.model_info.get("capabilities", [
            "text_generation", "reasoning", "analysis"
        ])

    def get_model_info(self) -> Dict[str, Any]:
        """Get model information"""
        return {
            "name": self.model,
            "provider": self.backend,
            "capabilities": self.get_capabilities(),
            "context_window": self.model_info.get("context_window", 4096),
            "supports_streaming": self.backend in ["ollama", "vllm"],
            "is_local": True,
            "backend": self.backend,
            "endpoint": self.endpoint
        }

    async def health_check(self) -> bool:
        """Check if local model is available"""
        try:
            if self.backend == "ollama":
                async with self._session.get(
                    f"{self.endpoint}/api/tags"
                ) as response:
                    return response.status == 200

            elif self.backend == "vllm":
                async with self._session.get(
                    f"{self.endpoint}/v1/models"
                ) as response:
                    return response.status == 200

            else:
                # For custom, just try the endpoint
                async with self._session.get(
                    f"{self.endpoint}/health"
                ) as response:
                    return response.status == 200

        except Exception as e:
            logger.error(f"Health check failed: {e}")
            return False

    async def close(self):
        """Close HTTP session"""
        if hasattr(self, "_session") and self._session:
            await self._session.close()
        await super().close()