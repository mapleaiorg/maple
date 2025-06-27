# File: llm/providers/openai.py
# Description: OpenAI (ChatGPT) provider implementation for MAPLE.

from __future__ import annotations
import asyncio
import os
from datetime import datetime
from typing import List, Optional, Dict, Any, AsyncIterator
import logging
import json

import aiohttp
from openai import AsyncOpenAI
import tiktoken

from llm.base import (
    BaseLLM, LLMProvider, ModelCapability, ModelInfo, PricingInfo,
    CompletionRequest, CompletionResponse, Message, Choice, TokenUsage,
    EmbeddingRequest, EmbeddingResponse, EmbeddingData
)

logger = logging.getLogger(__name__)


class OpenAIProvider(BaseLLM):
    """OpenAI provider implementation"""

    # Model configurations
    MODELS = {
        "gpt-4-turbo-preview": {
            "context_window": 128000,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.FUNCTION_CALLING,
                ModelCapability.VISION,
                ModelCapability.REASONING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.01, "output": 0.03}
        },
        "gpt-4-turbo": {
            "context_window": 128000,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.FUNCTION_CALLING,
                ModelCapability.VISION,
                ModelCapability.REASONING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.01, "output": 0.03}
        },
        "gpt-4": {
            "context_window": 8192,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.FUNCTION_CALLING,
                ModelCapability.REASONING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.03, "output": 0.06}
        },
        "gpt-4-32k": {
            "context_window": 32768,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.FUNCTION_CALLING,
                ModelCapability.REASONING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.06, "output": 0.12}
        },
        "gpt-3.5-turbo": {
            "context_window": 16385,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.FUNCTION_CALLING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.0005, "output": 0.0015}
        },
        "gpt-3.5-turbo-1106": {
            "context_window": 16385,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.FUNCTION_CALLING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.001, "output": 0.002}
        },
        "text-embedding-3-small": {
            "context_window": 8191,
            "capabilities": [ModelCapability.EMBEDDING],
            "pricing": {"embedding": 0.00002}
        },
        "text-embedding-3-large": {
            "context_window": 8191,
            "capabilities": [ModelCapability.EMBEDDING],
            "pricing": {"embedding": 0.00013}
        },
        "text-embedding-ada-002": {
            "context_window": 8191,
            "capabilities": [ModelCapability.EMBEDDING],
            "pricing": {"embedding": 0.0001}
        }
    }

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        super().__init__(LLMProvider.OPENAI, config)
        self.api_key = config.get('api_key') or os.getenv('OPENAI_API_KEY')
        self.organization = config.get('organization') or os.getenv('OPENAI_ORG_ID')
        self.base_url = config.get('base_url')
        self.default_model = config.get('default_model', 'gpt-3.5-turbo')
        self.timeout = config.get('timeout', 60)
        self.max_retries = config.get('max_retries', 3)

        self._client: Optional[AsyncOpenAI] = None
        self._tokenizers: Dict[str, tiktoken.Encoding] = {}

    async def initialize(self) -> None:
        """Initialize OpenAI client"""
        if not self.api_key:
            raise ValueError("OpenAI API key not provided")

        self._client = AsyncOpenAI(
            api_key=self.api_key,
            organization=self.organization,
            base_url=self.base_url,
            timeout=self.timeout,
            max_retries=self.max_retries
        )

        # Load model info
        for model_id, info in self.MODELS.items():
            self._models[model_id] = ModelInfo(
                id=model_id,
                provider=self.provider,
                capabilities=info["capabilities"],
                context_window=info["context_window"],
                max_output_tokens=info.get("max_output"),
                supports_functions="function_calling" in info["capabilities"],
                supports_tools="function_calling" in info["capabilities"],
                supports_vision="vision" in info["capabilities"],
                supports_streaming="streaming" in info["capabilities"],
                pricing=PricingInfo(
                    input_price_per_1k=info["pricing"].get("input", 0),
                    output_price_per_1k=info["pricing"].get("output", 0),
                    embedding_price_per_1k=info["pricing"].get("embedding", 0)
                ) if "pricing" in info else None
            )

        self._initialized = True
        logger.info("OpenAI provider initialized")

    async def complete(
            self,
            request: CompletionRequest
    ) -> CompletionResponse:
        """Generate completion using OpenAI"""
        model = request.model or self.default_model

        # Convert messages to OpenAI format
        messages = self._convert_messages(request.messages)

        # Prepare API parameters
        params = {
            "model": model,
            "messages": messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "top_p": request.top_p,
            "frequency_penalty": request.frequency_penalty,
            "presence_penalty": request.presence_penalty,
            "n": request.n,
            "stream": False,
            "user": request.user
        }

        # Add optional parameters
        if request.stop:
            params["stop"] = request.stop
        if request.functions:
            params["functions"] = request.functions
        if request.function_call:
            params["function_call"] = request.function_call
        if request.tools:
            params["tools"] = request.tools
        if request.tool_choice:
            params["tool_choice"] = request.tool_choice
        if request.response_format:
            params["response_format"] = request.response_format
        if request.seed is not None:
            params["seed"] = request.seed
        if request.logprobs is not None:
            params["logprobs"] = request.logprobs
        if request.top_logprobs is not None:
            params["top_logprobs"] = request.top_logprobs

        # Make API call
        try:
            response = await self._client.chat.completions.create(**params)

            # Convert to universal format
            choices = []
            for choice in response.choices:
                msg = Message(
                    role=choice.message.role,
                    content=choice.message.content or ""
                )

                if choice.message.function_call:
                    msg.function_call = {
                        "name": choice.message.function_call.name,
                        "arguments": choice.message.function_call.arguments
                    }

                if choice.message.tool_calls:
                    msg.tool_calls = [
                        {
                            "id": tc.id,
                            "type": tc.type,
                            "function": {
                                "name": tc.function.name,
                                "arguments": tc.function.arguments
                            }
                        }
                        for tc in choice.message.tool_calls
                    ]

                choices.append(Choice(
                    index=choice.index,
                    message=msg,
                    finish_reason=choice.finish_reason,
                    logprobs=choice.logprobs
                ))

            return CompletionResponse(
                id=response.id,
                model=response.model,
                created=datetime.fromtimestamp(response.created),
                choices=choices,
                usage=TokenUsage(
                    prompt_tokens=response.usage.prompt_tokens,
                    completion_tokens=response.usage.completion_tokens,
                    total_tokens=response.usage.total_tokens
                ),
                provider=self.provider
            )

        except Exception as e:
            logger.error(f"OpenAI completion error: {e}")
            raise

    async def stream_complete(
            self,
            request: CompletionRequest
    ) -> AsyncIterator[CompletionResponse]:
        """Stream completion from OpenAI"""
        model = request.model or self.default_model

        # Convert messages
        messages = self._convert_messages(request.messages)

        # Prepare parameters
        params = {
            "model": model,
            "messages": messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "top_p": request.top_p,
            "frequency_penalty": request.frequency_penalty,
            "presence_penalty": request.presence_penalty,
            "n": request.n,
            "stream": True,
            "user": request.user
        }

        # Add optional parameters
        if request.stop:
            params["stop"] = request.stop
        if request.functions:
            params["functions"] = request.functions
        if request.function_call:
            params["function_call"] = request.function_call
        if request.tools:
            params["tools"] = request.tools
        if request.tool_choice:
            params["tool_choice"] = request.tool_choice

        # Stream response
        try:
            stream = await self._client.chat.completions.create(**params)

            accumulated_content = ""
            accumulated_function_call = None
            accumulated_tool_calls = []

            async for chunk in stream:
                if not chunk.choices:
                    continue

                delta = chunk.choices[0].delta

                # Accumulate content
                if delta.content:
                    accumulated_content += delta.content

                # Accumulate function call
                if delta.function_call:
                    if not accumulated_function_call:
                        accumulated_function_call = {
                            "name": delta.function_call.name or "",
                            "arguments": ""
                        }
                    if delta.function_call.arguments:
                        accumulated_function_call["arguments"] += delta.function_call.arguments

                # Create response chunk
                msg = Message(
                    role="assistant",
                    content=accumulated_content
                )

                if accumulated_function_call:
                    msg.function_call = accumulated_function_call

                choice = Choice(
                    index=0,
                    message=msg,
                    finish_reason=chunk.choices[0].finish_reason
                )

                yield CompletionResponse(
                    id=chunk.id,
                    model=chunk.model,
                    created=datetime.fromtimestamp(chunk.created),
                    choices=[choice],
                    usage=TokenUsage(),  # Updated at end
                    provider=self.provider
                )

        except Exception as e:
            logger.error(f"OpenAI streaming error: {e}")
            raise

    async def embed(
            self,
            request: EmbeddingRequest
    ) -> EmbeddingResponse:
        """Generate embeddings using OpenAI"""
        model = request.model or "text-embedding-3-small"

        # Ensure input is a list
        if isinstance(request.input, str):
            inputs = [request.input]
        else:
            inputs = request.input

        try:
            response = await self._client.embeddings.create(
                model=model,
                input=inputs,
                encoding_format=request.encoding_format,
                dimensions=request.dimensions,
                user=request.user
            )

            # Convert to universal format
            data = [
                EmbeddingData(
                    index=item.index,
                    embedding=item.embedding
                )
                for item in response.data
            ]

            return EmbeddingResponse(
                data=data,
                model=response.model,
                usage=TokenUsage(
                    prompt_tokens=response.usage.prompt_tokens,
                    total_tokens=response.usage.total_tokens
                ),
                provider=self.provider
            )

        except Exception as e:
            logger.error(f"OpenAI embedding error: {e}")
            raise

    async def list_models(self) -> List[ModelInfo]:
        """List available OpenAI models"""
        # Return pre-configured models
        # Could also fetch from API if needed
        return list(self._models.values())

    def estimate_tokens(self, text: str, model: Optional[str] = None) -> int:
        """Estimate tokens using tiktoken"""
        model = model or self.default_model

        # Get or create tokenizer
        if model not in self._tokenizers:
            try:
                if model.startswith("gpt-4"):
                    encoding_name = "cl100k_base"
                elif model.startswith("gpt-3.5"):
                    encoding_name = "cl100k_base"
                else:
                    encoding_name = "cl100k_base"  # Default

                self._tokenizers[model] = tiktoken.get_encoding(encoding_name)
            except Exception:
                # Fallback to simple estimation
                return super().estimate_tokens(text)

        tokenizer = self._tokenizers[model]
        return len(tokenizer.encode(text))

    def _convert_messages(self, messages: List[Message]) -> List[Dict[str, Any]]:
        """Convert universal messages to OpenAI format"""
        openai_messages = []

        for msg in messages:
            openai_msg = {
                "role": msg.role,
                "content": msg.content
            }

            if msg.name:
                openai_msg["name"] = msg.name

            if msg.function_call:
                openai_msg["function_call"] = msg.function_call

            if msg.tool_calls:
                openai_msg["tool_calls"] = msg.tool_calls

            openai_messages.append(openai_msg)

        return openai_messages

    def _validate_provider_request(self, request: CompletionRequest) -> None:
        """Validate OpenAI-specific request parameters"""
        model = request.model or self.default_model

        # Check if model exists
        if model not in self._models:
            raise ValueError(f"Unknown model: {model}")

        model_info = self._models[model]

        # Validate context window
        estimated_tokens = sum(
            self.estimate_tokens(str(msg.content), model)
            for msg in request.messages
        )

        if estimated_tokens > model_info.context_window:
            raise ValueError(
                f"Estimated tokens ({estimated_tokens}) exceed "
                f"model context window ({model_info.context_window})"
            )

        # Validate functions/tools support
        if request.functions and not model_info.supports_functions:
            raise ValueError(f"Model {model} does not support functions")

        if request.tools and not model_info.supports_tools:
            raise ValueError(f"Model {model} does not support tools")

        # Validate vision support
        for msg in request.messages:
            if isinstance(msg.content, list):
                for item in msg.content:
                    if item.get('type') == 'image_url' and not model_info.supports_vision:
                        raise ValueError(f"Model {model} does not support vision")