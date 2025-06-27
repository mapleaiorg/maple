# File: maple/llm/providers/anthropic.py
# Description: Anthropic (Claude) provider implementation for MAPLE.

from __future__ import annotations
import asyncio
import os
from datetime import datetime
from typing import List, Optional, Dict, Any, AsyncIterator
import logging
import json

import anthropic
from anthropic import AsyncAnthropic

from maple.core.llm.base import (
    BaseLLM, LLMProvider, ModelCapability, ModelInfo, PricingInfo,
    CompletionRequest, CompletionResponse, Message, Choice, TokenUsage,
    EmbeddingRequest, EmbeddingResponse
)

logger = logging.getLogger(__name__)


class AnthropicProvider(BaseLLM):
    """Anthropic (Claude) provider implementation"""

    # Model configurations
    MODELS = {
        "claude-3-opus-20240229": {
            "context_window": 200000,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.VISION,
                ModelCapability.REASONING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.015, "output": 0.075}
        },
        "claude-3-sonnet-20240229": {
            "context_window": 200000,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.VISION,
                ModelCapability.REASONING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.003, "output": 0.015}
        },
        "claude-3-haiku-20240307": {
            "context_window": 200000,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.VISION,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.00025, "output": 0.00125}
        },
        "claude-2.1": {
            "context_window": 200000,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.REASONING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.008, "output": 0.024}
        },
        "claude-2.0": {
            "context_window": 100000,
            "max_output": 4096,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.008, "output": 0.024}
        }
    }

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        super().__init__(LLMProvider.ANTHROPIC, config)
        self.api_key = config.get('api_key') or os.getenv('ANTHROPIC_API_KEY')
        self.base_url = config.get('base_url')
        self.default_model = config.get('default_model', 'claude-3-sonnet-20240229')
        self.timeout = config.get('timeout', 60)
        self.max_retries = config.get('max_retries', 3)

        self._client: Optional[AsyncAnthropic] = None

    async def initialize(self) -> None:
        """Initialize Anthropic client"""
        if not self.api_key:
            raise ValueError("Anthropic API key not provided")

        self._client = AsyncAnthropic(
            api_key=self.api_key,
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
                supports_functions=False,  # Claude doesn't have native function calling
                supports_tools=False,
                supports_vision="vision" in info["capabilities"],
                supports_streaming="streaming" in info["capabilities"],
                pricing=PricingInfo(
                    input_price_per_1k=info["pricing"]["input"],
                    output_price_per_1k=info["pricing"]["output"]
                ) if "pricing" in info else None
            )

        self._initialized = True
        logger.info("Anthropic provider initialized")

    async def complete(
            self,
            request: CompletionRequest
    ) -> CompletionResponse:
        """Generate completion using Anthropic"""
        model = request.model or self.default_model

        # Convert messages to Anthropic format
        system_prompt, messages = self._convert_messages(request.messages)

        # Prepare parameters
        params = {
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens or 4096,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": False
        }

        if system_prompt:
            params["system"] = system_prompt

        if request.stop:
            if isinstance(request.stop, str):
                params["stop_sequences"] = [request.stop]
            else:
                params["stop_sequences"] = request.stop

        # Handle tools/functions through system prompt
        if request.functions or request.tools:
            tool_prompt = self._create_tool_prompt(request.functions, request.tools)
            if params.get("system"):
                params["system"] += f"\n\n{tool_prompt}"
            else:
                params["system"] = tool_prompt

        # Make API call
        try:
            response = await self._client.messages.create(**params)

            # Convert to universal format
            content = ""
            function_calls = []

            for block in response.content:
                if block.type == "text":
                    content += block.text
                elif block.type == "tool_use":
                    # Parse tool use as function call
                    function_calls.append({
                        "name": block.name,
                        "arguments": json.dumps(block.input)
                    })

            msg = Message(
                role="assistant",
                content=content
            )

            if function_calls:
                msg.function_call = function_calls[0]  # Primary function call
                if len(function_calls) > 1:
                    msg.tool_calls = [
                        {
                            "id": f"call_{i}",
                            "type": "function",
                            "function": fc
                        }
                        for i, fc in enumerate(function_calls)
                    ]

            return CompletionResponse(
                id=response.id,
                model=response.model,
                created=datetime.utcnow(),
                choices=[Choice(
                    index=0,
                    message=msg,
                    finish_reason=response.stop_reason
                )],
                usage=TokenUsage(
                    prompt_tokens=response.usage.input_tokens,
                    completion_tokens=response.usage.output_tokens,
                    total_tokens=response.usage.input_tokens + response.usage.output_tokens
                ),
                provider=self.provider
            )

        except Exception as e:
            logger.error(f"Anthropic completion error: {e}")
            raise

    async def stream_complete(
            self,
            request: CompletionRequest
    ) -> AsyncIterator[CompletionResponse]:
        """Stream completion from Anthropic"""
        model = request.model or self.default_model

        # Convert messages
        system_prompt, messages = self._convert_messages(request.messages)

        # Prepare parameters
        params = {
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens or 4096,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": True
        }

        if system_prompt:
            params["system"] = system_prompt

        if request.stop:
            if isinstance(request.stop, str):
                params["stop_sequences"] = [request.stop]
            else:
                params["stop_sequences"] = request.stop

        # Stream response
        try:
            stream = await self._client.messages.create(**params)

            accumulated_content = ""
            message_id = None

            async for event in stream:
                if event.type == "message_start":
                    message_id = event.message.id

                elif event.type == "content_block_delta":
                    if event.delta.type == "text_delta":
                        accumulated_content += event.delta.text

                        yield CompletionResponse(
                            id=message_id or "stream",
                            model=model,
                            created=datetime.utcnow(),
                            choices=[Choice(
                                index=0,
                                message=Message(
                                    role="assistant",
                                    content=accumulated_content
                                ),
                                finish_reason=None
                            )],
                            usage=TokenUsage(),
                            provider=self.provider
                        )

                elif event.type == "message_delta":
                    # Final message with usage
                    if hasattr(event, 'usage'):
                        yield CompletionResponse(
                            id=message_id or "stream",
                            model=model,
                            created=datetime.utcnow(),
                            choices=[Choice(
                                index=0,
                                message=Message(
                                    role="assistant",
                                    content=accumulated_content
                                ),
                                finish_reason=event.delta.stop_reason
                            )],
                            usage=TokenUsage(
                                prompt_tokens=event.usage.input_tokens,
                                completion_tokens=event.usage.output_tokens,
                                total_tokens=event.usage.input_tokens + event.usage.output_tokens
                            ),
                            provider=self.provider
                        )

        except Exception as e:
            logger.error(f"Anthropic streaming error: {e}")
            raise

    async def embed(
            self,
            request: EmbeddingRequest
    ) -> EmbeddingResponse:
        """Anthropic doesn't provide embeddings"""
        raise NotImplementedError("Anthropic does not support embeddings")

    async def list_models(self) -> List[ModelInfo]:
        """List available Anthropic models"""
        return list(self._models.values())

    def _convert_messages(
            self,
            messages: List[Message]
    ) -> tuple[Optional[str], List[Dict[str, Any]]]:
        """Convert universal messages to Anthropic format"""
        system_prompt = None
        anthropic_messages = []

        for msg in messages:
            if msg.role == "system":
                # Anthropic uses a separate system parameter
                if system_prompt:
                    system_prompt += f"\n\n{msg.content}"
                else:
                    system_prompt = msg.content
            else:
                # Convert content to Anthropic format
                if isinstance(msg.content, str):
                    content = [{"type": "text", "text": msg.content}]
                else:
                    # Handle multimodal content
                    content = []
                    for item in msg.content:
                        if item["type"] == "text":
                            content.append({
                                "type": "text",
                                "text": item["text"]
                            })
                        elif item["type"] == "image_url":
                            # Convert image to Anthropic format
                            content.append({
                                "type": "image",
                                "source": {
                                    "type": "url",
                                    "url": item["image_url"]["url"]
                                }
                            })

                anthropic_messages.append({
                    "role": "user" if msg.role == "user" else "assistant",
                    "content": content
                })

        return system_prompt, anthropic_messages

    def _create_tool_prompt(
            self,
            functions: Optional[List[Dict[str, Any]]],
            tools: Optional[List[Dict[str, Any]]]
    ) -> str:
        """Create system prompt for function/tool calling"""
        prompt = "You have access to the following tools:\n\n"

        all_tools = []
        if functions:
            all_tools.extend(functions)
        if tools:
            all_tools.extend([t["function"] for t in tools if "function" in t])

        for tool in all_tools:
            prompt += f"Tool: {tool['name']}\n"
            prompt += f"Description: {tool.get('description', 'No description')}\n"
            prompt += f"Parameters: {json.dumps(tool.get('parameters', {}), indent=2)}\n\n"

        prompt += (
            "To use a tool, respond with a JSON object in the following format:\n"
            '{"tool": "tool_name", "arguments": {...}}\n\n'
            "Make sure to provide valid JSON that can be parsed."
        )

        return prompt

    def _validate_provider_request(self, request: CompletionRequest) -> None:
        """Validate Anthropic-specific request parameters"""
        model = request.model or self.default_model

        # Check if model exists
        if model not in self._models:
            raise ValueError(f"Unknown model: {model}")

        model_info = self._models[model]

        # Validate vision support
        for msg in request.messages:
            if isinstance(msg.content, list):
                for item in msg.content:
                    if item.get('type') == 'image_url' and not model_info.supports_vision:
                        raise ValueError(f"Model {model} does not support vision")

        # Warn about function calling
        if request.functions or request.tools:
            logger.warning(
                "Anthropic does not have native function calling. "
                "Functions will be handled through system prompts."
            )