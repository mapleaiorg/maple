from __future__ import annotations
import asyncio
import os
from datetime import datetime
from typing import List, Optional, Dict, Any, AsyncIterator
import logging
import json

import google.generativeai as genai
from google.generativeai.types import (
    ContentType, GenerationConfig, SafetySettingsType
)

from llm.base import (
    BaseLLM, LLMProvider, ModelCapability, ModelInfo, PricingInfo,
    CompletionRequest, CompletionResponse, Message, Choice, TokenUsage,
    EmbeddingRequest, EmbeddingResponse, EmbeddingData
)

logger = logging.getLogger(__name__)


class GoogleProvider(BaseLLM):
    """Google (Gemini) provider implementation"""

    # Model configurations
    MODELS = {
        "gemini-1.5-pro": {
            "context_window": 1048576,  # 1M tokens
            "max_output": 8192,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.FUNCTION_CALLING,
                ModelCapability.VISION,
                ModelCapability.AUDIO,
                ModelCapability.REASONING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.00125, "output": 0.005}
        },
        "gemini-1.5-flash": {
            "context_window": 1048576,  # 1M tokens
            "max_output": 8192,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.FUNCTION_CALLING,
                ModelCapability.VISION,
                ModelCapability.AUDIO,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.00035, "output": 0.00105}
        },
        "gemini-1.0-pro": {
            "context_window": 32768,
            "max_output": 8192,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.FUNCTION_CALLING,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.0005, "output": 0.0015}
        },
        "gemini-1.0-pro-vision": {
            "context_window": 16384,
            "max_output": 2048,
            "capabilities": [
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.VISION,
                ModelCapability.STREAMING,
            ],
            "pricing": {"input": 0.0005, "output": 0.0015}
        },
        "embedding-001": {
            "context_window": 2048,
            "capabilities": [ModelCapability.EMBEDDING],
            "pricing": {"embedding": 0.0001}
        }
    }

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        super().__init__(LLMProvider.GOOGLE, config)
        self.api_key = config.get('api_key') or os.getenv('GOOGLE_API_KEY')
        self.default_model = config.get('default_model', 'gemini-1.5-flash')
        self.safety_settings = config.get('safety_settings', {})
        self.timeout = config.get('timeout', 60)

        self._models_cache: Dict[str, genai.GenerativeModel] = {}

    async def initialize(self) -> None:
        """Initialize Google Gemini client"""
        if not self.api_key:
            raise ValueError("Google API key not provided")

        genai.configure(api_key=self.api_key)

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
        logger.info("Google provider initialized")

    async def complete(
            self,
            request: CompletionRequest
    ) -> CompletionResponse:
        """Generate completion using Google Gemini"""
        model_name = request.model or self.default_model

        # Get or create model
        model = self._get_model(model_name, request)

        # Convert messages to Gemini format
        chat = model.start_chat(history=self._convert_messages(request.messages[:-1]))

        # Prepare generation config
        generation_config = GenerationConfig(
            temperature=request.temperature,
            top_p=request.top_p,
            max_output_tokens=request.max_tokens,
            stop_sequences=request.stop if isinstance(request.stop, list) else [request.stop] if request.stop else None,
            candidate_count=request.n
        )

        # Generate response
        try:
            # Use the last message as the prompt
            last_message = request.messages[-1]
            prompt = self._convert_message_content(last_message.content)

            response = await asyncio.to_thread(
                chat.send_message,
                prompt,
                generation_config=generation_config,
                safety_settings=self.safety_settings,
                stream=False
            )

            # Convert to universal format
            choices = []
            for i, candidate in enumerate(response.candidates):
                content = ""
                function_calls = []

                for part in candidate.content.parts:
                    if hasattr(part, 'text'):
                        content += part.text
                    elif hasattr(part, 'function_call'):
                        function_calls.append({
                            "name": part.function_call.name,
                            "arguments": json.dumps(dict(part.function_call.args))
                        })

                msg = Message(
                    role="assistant",
                    content=content
                )

                if function_calls:
                    msg.function_call = function_calls[0]
                    if len(function_calls) > 1:
                        msg.tool_calls = [
                            {
                                "id": f"call_{j}",
                                "type": "function",
                                "function": fc
                            }
                            for j, fc in enumerate(function_calls)
                        ]

                choices.append(Choice(
                    index=i,
                    message=msg,
                    finish_reason=self._convert_finish_reason(candidate.finish_reason)
                ))

            # Calculate token usage
            prompt_tokens = model.count_tokens(prompt).total_tokens
            completion_tokens = model.count_tokens(response.text).total_tokens

            return CompletionResponse(
                id=f"gemini-{datetime.utcnow().timestamp()}",
                model=model_name,
                created=datetime.utcnow(),
                choices=choices,
                usage=TokenUsage(
                    prompt_tokens=prompt_tokens,
                    completion_tokens=completion_tokens,
                    total_tokens=prompt_tokens + completion_tokens
                ),
                provider=self.provider
            )

        except Exception as e:
            logger.error(f"Google completion error: {e}")
            raise

    async def stream_complete(
            self,
            request: CompletionRequest
    ) -> AsyncIterator[CompletionResponse]:
        """Stream completion from Google Gemini"""
        model_name = request.model or self.default_model

        # Get or create model
        model = self._get_model(model_name, request)

        # Convert messages
        chat = model.start_chat(history=self._convert_messages(request.messages[:-1]))

        # Prepare generation config
        generation_config = GenerationConfig(
            temperature=request.temperature,
            top_p=request.top_p,
            max_output_tokens=request.max_tokens,
            stop_sequences=request.stop if isinstance(request.stop, list) else [request.stop] if request.stop else None
        )

        # Stream response
        try:
            last_message = request.messages[-1]
            prompt = self._convert_message_content(last_message.content)

            response_stream = await asyncio.to_thread(
                chat.send_message,
                prompt,
                generation_config=generation_config,
                safety_settings=self.safety_settings,
                stream=True
            )

            accumulated_content = ""

            for chunk in response_stream:
                if chunk.text:
                    accumulated_content += chunk.text

                    yield CompletionResponse(
                        id=f"gemini-stream-{datetime.utcnow().timestamp()}",
                        model=model_name,
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

            # Final response with usage
            yield CompletionResponse(
                id=f"gemini-stream-{datetime.utcnow().timestamp()}",
                model=model_name,
                created=datetime.utcnow(),
                choices=[Choice(
                    index=0,
                    message=Message(
                        role="assistant",
                        content=accumulated_content
                    ),
                    finish_reason="stop"
                )],
                usage=TokenUsage(
                    prompt_tokens=model.count_tokens(prompt).total_tokens,
                    completion_tokens=model.count_tokens(accumulated_content).total_tokens,
                    total_tokens=model.count_tokens(prompt).total_tokens + model.count_tokens(
                        accumulated_content).total_tokens
                ),
                provider=self.provider
            )

        except Exception as e:
            logger.error(f"Google streaming error: {e}")
            raise

    async def embed(
            self,
            request: EmbeddingRequest
    ) -> EmbeddingResponse:
        """Generate embeddings using Google"""
        model_name = request.model or "embedding-001"

        # Ensure input is a list
        if isinstance(request.input, str):
            texts = [request.input]
        else:
            texts = request.input

        try:
            model = genai.GenerativeModel(model_name)

            embeddings = []
            for i, text in enumerate(texts):
                result = await asyncio.to_thread(
                    model.embed_content,
                    content=text,
                    task_type="retrieval_document"
                )

                embeddings.append(EmbeddingData(
                    index=i,
                    embedding=result['embedding']
                ))

            # Estimate token usage
            total_tokens = sum(len(text.split()) * 1.3 for text in texts)  # Rough estimate

            return EmbeddingResponse(
                data=embeddings,
                model=model_name,
                usage=TokenUsage(
                    prompt_tokens=int(total_tokens),
                    total_tokens=int(total_tokens)
                ),
                provider=self.provider
            )

        except Exception as e:
            logger.error(f"Google embedding error: {e}")
            raise

    async def list_models(self) -> List[ModelInfo]:
        """List available Google models"""
        return list(self._models.values())

    def _get_model(
            self,
            model_name: str,
            request: CompletionRequest
    ) -> genai.GenerativeModel:
        """Get or create a GenerativeModel instance"""
        cache_key = f"{model_name}:{bool(request.functions)}:{bool(request.tools)}"

        if cache_key not in self._models_cache:
            # Prepare model configuration
            model_config = {
                "model_name": model_name
            }

            # Add tools/functions if specified
            tools = []
            if request.functions:
                for func in request.functions:
                    tools.append({
                        "function_declarations": [{
                            "name": func["name"],
                            "description": func.get("description", ""),
                            "parameters": func.get("parameters", {})
                        }]
                    })

            if request.tools:
                for tool in request.tools:
                    if "function" in tool:
                        tools.append({
                            "function_declarations": [{
                                "name": tool["function"]["name"],
                                "description": tool["function"].get("description", ""),
                                "parameters": tool["function"].get("parameters", {})
                            }]
                        })

            if tools:
                model_config["tools"] = tools

            self._models_cache[cache_key] = genai.GenerativeModel(**model_config)

        return self._models_cache[cache_key]

    def _convert_messages(self, messages: List[Message]) -> List[ContentType]:
        """Convert universal messages to Gemini format"""
        history = []

        for msg in messages:
            if msg.role == "system":
                # Gemini doesn't have system role, prepend to first user message
                continue

            parts = self._convert_message_content(msg.content)

            # Add function responses if present
            if msg.role == "function":
                parts.append({
                    "function_response": {
                        "name": msg.name,
                        "response": json.loads(msg.content) if isinstance(msg.content, str) else msg.content
                    }
                })

            history.append({
                "role": "model" if msg.role == "assistant" else "user",
                "parts": parts
            })

        return history

    def _convert_message_content(self, content: Any) -> List[Any]:
        """Convert message content to Gemini parts"""
        if isinstance(content, str):
            return [content]

        parts = []
        for item in content:
            if item["type"] == "text":
                parts.append(item["text"])
            elif item["type"] == "image_url":
                # Convert image URL to Gemini format
                # Note: This is simplified, actual implementation would handle base64, etc.
                parts.append({
                    "inline_data": {
                        "mime_type": "image/jpeg",
                        "data": item["image_url"]["url"]  # This would need proper handling
                    }
                })

        return parts

    def _convert_finish_reason(self, reason: Any) -> str:
        """Convert Gemini finish reason to universal format"""
        reason_map = {
            "STOP": "stop",
            "MAX_TOKENS": "length",
            "SAFETY": "content_filter",
            "RECITATION": "content_filter",
            "OTHER": "stop"
        }
        return reason_map.get(str(reason), "stop")

    def _validate_provider_request(self, request: CompletionRequest) -> None:
        """Validate Google-specific request parameters"""
        model = request.model or self.default_model

        # Check if model exists
        if model not in self._models:
            raise ValueError(f"Unknown model: {model}")

        model_info = self._models[model]

        # Validate vision support
        for msg in request.messages:
            if isinstance(msg.content, list):
                has_vision = any(
                    item.get('type') in ['image_url', 'image']
                    for item in msg.content
                )
                if has_vision and not model_info.supports_vision:
                    raise ValueError(f"Model {model} does not support vision")