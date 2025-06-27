# File: maple/llm/base.py
# Description: Base classes and interfaces for LLM integration in MAPLE.
# Provides unified abstraction for all LLM providers.

from __future__ import annotations
import asyncio
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime
from typing import (
    List, Optional, Dict, Any, Union, AsyncIterator,
    Callable, TypeVar, Generic, Literal
)
from enum import Enum
import logging
import json
from contextlib import asynccontextmanager

import numpy as np
from pydantic import BaseModel, Field, validator

logger = logging.getLogger(__name__)

T = TypeVar('T')


class LLMProvider(str, Enum):
    """Supported LLM providers"""
    # Local models
    LLAMA = "llama"
    MISTRAL = "mistral"
    MIXTRAL = "mixtral"
    CODELLAMA = "codellama"
    VICUNA = "vicuna"
    ALPACA = "alpaca"
    FALCON = "falcon"
    MPT = "mpt"

    # Cloud providers
    OPENAI = "openai"
    ANTHROPIC = "anthropic"
    GOOGLE = "google"
    COHERE = "cohere"
    HUGGINGFACE = "huggingface"
    REPLICATE = "replicate"

    # Specialized
    CUSTOM = "custom"


class ModelCapability(str, Enum):
    """Model capabilities"""
    CHAT = "chat"
    COMPLETION = "completion"
    EMBEDDING = "embedding"
    CODE = "code"
    FUNCTION_CALLING = "function_calling"
    VISION = "vision"
    AUDIO = "audio"
    REASONING = "reasoning"
    AGENTS = "agents"
    FINE_TUNING = "fine_tuning"
    STREAMING = "streaming"
    BATCH = "batch"


class TokenUsage(BaseModel):
    """Token usage statistics"""
    prompt_tokens: int = 0
    completion_tokens: int = 0
    total_tokens: int = 0
    cached_tokens: Optional[int] = None

    @property
    def cost(self) -> Optional[float]:
        """Calculate cost if pricing info available"""
        # Implementation would use provider-specific pricing
        return None


class Message(BaseModel):
    """Universal message format"""
    role: Literal["system", "user", "assistant", "function", "tool"] = "user"
    content: Union[str, List[Dict[str, Any]]]
    name: Optional[str] = None
    function_call: Optional[Dict[str, Any]] = None
    tool_calls: Optional[List[Dict[str, Any]]] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)

    @validator('content')
    def validate_content(cls, v):
        if isinstance(v, list):
            # Validate multimodal content
            for item in v:
                if 'type' not in item:
                    raise ValueError("Multimodal content must have 'type' field")
        return v


class CompletionRequest(BaseModel):
    """Universal completion request"""
    messages: List[Message]
    model: Optional[str] = None
    temperature: float = Field(default=0.7, ge=0.0, le=2.0)
    max_tokens: Optional[int] = Field(default=None, gt=0)
    top_p: float = Field(default=1.0, ge=0.0, le=1.0)
    frequency_penalty: float = Field(default=0.0, ge=-2.0, le=2.0)
    presence_penalty: float = Field(default=0.0, ge=-2.0, le=2.0)
    stop: Optional[Union[str, List[str]]] = None
    stream: bool = False
    n: int = Field(default=1, ge=1)
    functions: Optional[List[Dict[str, Any]]] = None
    function_call: Optional[Union[str, Dict[str, Any]]] = None
    tools: Optional[List[Dict[str, Any]]] = None
    tool_choice: Optional[Union[str, Dict[str, Any]]] = None
    response_format: Optional[Dict[str, Any]] = None
    seed: Optional[int] = None
    logprobs: Optional[bool] = None
    top_logprobs: Optional[int] = None
    user: Optional[str] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)

    class Config:
        extra = "allow"  # Allow provider-specific parameters


class CompletionResponse(BaseModel):
    """Universal completion response"""
    id: str
    model: str
    created: datetime
    choices: List['Choice']
    usage: TokenUsage
    provider: LLMProvider
    cached: bool = False
    metadata: Dict[str, Any] = Field(default_factory=dict)

    @property
    def content(self) -> str:
        """Get first choice content"""
        if self.choices:
            return self.choices[0].message.content
        return ""

    @property
    def function_calls(self) -> List[Dict[str, Any]]:
        """Get function calls from first choice"""
        if self.choices and self.choices[0].message.function_call:
            return [self.choices[0].message.function_call]
        return []

    @property
    def tool_calls(self) -> List[Dict[str, Any]]:
        """Get tool calls from first choice"""
        if self.choices and self.choices[0].message.tool_calls:
            return self.choices[0].message.tool_calls
        return []


class Choice(BaseModel):
    """Completion choice"""
    index: int
    message: Message
    finish_reason: Optional[str] = None
    logprobs: Optional[Dict[str, Any]] = None


class EmbeddingRequest(BaseModel):
    """Embedding request"""
    input: Union[str, List[str]]
    model: Optional[str] = None
    encoding_format: Optional[str] = None
    dimensions: Optional[int] = None
    user: Optional[str] = None


class EmbeddingResponse(BaseModel):
    """Embedding response"""
    data: List['EmbeddingData']
    model: str
    usage: TokenUsage
    provider: LLMProvider


class EmbeddingData(BaseModel):
    """Single embedding data"""
    index: int
    embedding: List[float]
    object: str = "embedding"


class ModelInfo(BaseModel):
    """Model information"""
    id: str
    provider: LLMProvider
    capabilities: List[ModelCapability]
    context_window: int
    max_output_tokens: Optional[int] = None
    supports_functions: bool = False
    supports_tools: bool = False
    supports_vision: bool = False
    supports_streaming: bool = True
    pricing: Optional['PricingInfo'] = None
    metadata: Dict[str, Any] = Field(default_factory=dict)


class PricingInfo(BaseModel):
    """Model pricing information"""
    currency: str = "USD"
    input_price_per_1k: float  # Price per 1k input tokens
    output_price_per_1k: float  # Price per 1k output tokens
    embedding_price_per_1k: Optional[float] = None
    fine_tuning_price_per_1k: Optional[float] = None

    def calculate_cost(self, usage: TokenUsage) -> float:
        """Calculate cost for token usage"""
        input_cost = (usage.prompt_tokens / 1000) * self.input_price_per_1k
        output_cost = (usage.completion_tokens / 1000) * self.output_price_per_1k
        return input_cost + output_cost


# Update forward references
CompletionResponse.model_rebuild()


# Base LLM interface

class BaseLLM(ABC):
    """
    Abstract base class for all LLM providers.
    Defines the unified interface for LLM interactions.
    """

    def __init__(
            self,
            provider: LLMProvider,
            config: Optional[Dict[str, Any]] = None
    ):
        self.provider = provider
        self.config = config or {}
        self._models: Dict[str, ModelInfo] = {}
        self._initialized = False

    @abstractmethod
    async def initialize(self) -> None:
        """Initialize the LLM provider"""
        pass

    @abstractmethod
    async def complete(
            self,
            request: CompletionRequest
    ) -> CompletionResponse:
        """Generate completion"""
        pass

    @abstractmethod
    async def stream_complete(
            self,
            request: CompletionRequest
    ) -> AsyncIterator[CompletionResponse]:
        """Stream completion chunks"""
        pass

    @abstractmethod
    async def embed(
            self,
            request: EmbeddingRequest
    ) -> EmbeddingResponse:
        """Generate embeddings"""
        pass

    @abstractmethod
    async def list_models(self) -> List[ModelInfo]:
        """List available models"""
        pass

    async def get_model_info(self, model_id: str) -> Optional[ModelInfo]:
        """Get model information"""
        if not self._models:
            models = await self.list_models()
            self._models = {m.id: m for m in models}
        return self._models.get(model_id)

    async def health_check(self) -> bool:
        """Check if provider is healthy"""
        try:
            await self.list_models()
            return True
        except:
            return False

    async def close(self) -> None:
        """Cleanup resources"""
        pass

    def supports_capability(
            self,
            model_id: str,
            capability: ModelCapability
    ) -> bool:
        """Check if model supports capability"""
        model_info = self._models.get(model_id)
        if model_info:
            return capability in model_info.capabilities
        return False

    def estimate_tokens(self, text: str) -> int:
        """Estimate token count (rough approximation)"""
        # Simple approximation: ~4 characters per token
        return len(text) // 4

    def validate_request(self, request: CompletionRequest) -> None:
        """Validate request parameters"""
        if not request.messages:
            raise ValueError("Messages cannot be empty")

        # Validate message roles
        valid_roles = {"system", "user", "assistant", "function", "tool"}
        for msg in request.messages:
            if msg.role not in valid_roles:
                raise ValueError(f"Invalid role: {msg.role}")

        # Provider-specific validation
        self._validate_provider_request(request)

    def _validate_provider_request(self, request: CompletionRequest) -> None:
        """Provider-specific validation (override in subclasses)"""
        pass


# LLM Manager for intelligent routing and fallback

class LLMManager:
    """
    Intelligent LLM manager for MAPLE agents.
    Handles provider selection, load balancing, and fallback.
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        self.config = config or {}
        self._providers: Dict[LLMProvider, BaseLLM] = {}
        self._default_provider: Optional[LLMProvider] = None
        self._model_routing: Dict[str, LLMProvider] = {}
        self._usage_stats: Dict[str, 'UsageStats'] = {}
        self._cache: Optional['LLMCache'] = None
        self._initialized = False

    async def initialize(self) -> None:
        """Initialize LLM manager"""
        if self._initialized:
            return

        # Initialize cache if enabled
        if self.config.get('enable_cache', True):
            self._cache = LLMCache(
                max_size=self.config.get('cache_max_size', 1000),
                ttl=self.config.get('cache_ttl', 3600)
            )

        # Initialize configured providers
        await self._initialize_providers()

        self._initialized = True
        logger.info("LLM Manager initialized")

    async def _initialize_providers(self) -> None:
        """Initialize configured providers"""
        providers_config = self.config.get('providers', {})

        for provider_name, provider_config in providers_config.items():
            if not provider_config.get('enabled', True):
                continue

            try:
                provider = LLMProvider(provider_name)
                llm = await self._create_provider(provider, provider_config)
                await llm.initialize()
                self._providers[provider] = llm

                # Set default provider
                if provider_config.get('default', False):
                    self._default_provider = provider

                logger.info(f"Initialized {provider} provider")

            except Exception as e:
                logger.error(f"Failed to initialize {provider_name}: {e}")

    async def _create_provider(
            self,
            provider: LLMProvider,
            config: Dict[str, Any]
    ) -> BaseLLM:
        """Create provider instance"""
        # Import provider implementations
        if provider == LLMProvider.OPENAI:
            from maple.core.llm.providers.openai import OpenAIProvider
            return OpenAIProvider(config)
        elif provider == LLMProvider.ANTHROPIC:
            from maple.core.llm.providers.anthropic import AnthropicProvider
            return AnthropicProvider(config)
        elif provider == LLMProvider.GOOGLE:
            from maple.core.llm.providers.google import GoogleProvider
            return GoogleProvider(config)
        elif provider == LLMProvider.LLAMA:
            from maple.core.llm.providers.local.llama import LlamaProvider
            return LlamaProvider(config)
        elif provider == LLMProvider.MISTRAL:
            from maple.core.llm.providers.local.mistral import MistralProvider
            return MistralProvider(config)
        else:
            raise ValueError(f"Unsupported provider: {provider}")

    async def complete(
            self,
            request: CompletionRequest,
            provider: Optional[LLMProvider] = None,
            fallback: bool = True
    ) -> CompletionResponse:
        """
        Generate completion with intelligent routing.

        Args:
            request: Completion request
            provider: Specific provider to use (optional)
            fallback: Enable fallback to other providers

        Returns:
            Completion response
        """
        # Check cache
        if self._cache:
            cached = await self._cache.get(request)
            if cached:
                cached.cached = True
                return cached

        # Determine provider
        if not provider:
            provider = self._select_provider(request)

        # Try primary provider
        try:
            llm = self._providers.get(provider)
            if not llm:
                raise ValueError(f"Provider {provider} not available")

            # Validate and execute
            llm.validate_request(request)
            response = await llm.complete(request)

            # Cache response
            if self._cache:
                await self._cache.set(request, response)

            # Update usage stats
            self._update_usage_stats(provider, response.usage)

            return response

        except Exception as e:
            logger.error(f"Provider {provider} failed: {e}")

            if fallback:
                # Try fallback providers
                return await self._complete_with_fallback(request, exclude=[provider])
            else:
                raise

    async def _complete_with_fallback(
            self,
            request: CompletionRequest,
            exclude: List[LLMProvider]
    ) -> CompletionResponse:
        """Complete with fallback providers"""
        # Get fallback order based on capabilities and load
        fallback_providers = self._get_fallback_providers(request, exclude)

        for provider in fallback_providers:
            try:
                logger.info(f"Trying fallback provider: {provider}")
                return await self.complete(request, provider, fallback=False)
            except Exception as e:
                logger.error(f"Fallback provider {provider} failed: {e}")
                continue

        raise RuntimeError("All providers failed")

    def _select_provider(self, request: CompletionRequest) -> LLMProvider:
        """Select optimal provider for request"""
        # Model-specific routing
        if request.model and request.model in self._model_routing:
            return self._model_routing[request.model]

        # Capability-based selection
        required_capabilities = self._extract_required_capabilities(request)

        candidates = []
        for provider, llm in self._providers.items():
            # Check if provider supports all required capabilities
            if all(
                    any(
                        llm.supports_capability(model.id, cap)
                        for model in llm._models.values()
                    )
                    for cap in required_capabilities
            ):
                candidates.append(provider)

        if not candidates:
            if self._default_provider:
                return self._default_provider
            raise ValueError("No suitable provider found")

        # Select based on load and performance
        return self._select_by_load(candidates)

    def _extract_required_capabilities(
            self,
            request: CompletionRequest
    ) -> List[ModelCapability]:
        """Extract required capabilities from request"""
        capabilities = [ModelCapability.CHAT]

        if request.functions or request.function_call:
            capabilities.append(ModelCapability.FUNCTION_CALLING)

        if request.tools or request.tool_choice:
            capabilities.append(ModelCapability.FUNCTION_CALLING)

        if request.stream:
            capabilities.append(ModelCapability.STREAMING)

        # Check for multimodal content
        for msg in request.messages:
            if isinstance(msg.content, list):
                for item in msg.content:
                    if item.get('type') == 'image':
                        capabilities.append(ModelCapability.VISION)
                    elif item.get('type') == 'audio':
                        capabilities.append(ModelCapability.AUDIO)

        return list(set(capabilities))

    def _select_by_load(self, candidates: List[LLMProvider]) -> LLMProvider:
        """Select provider based on load balancing"""
        # Simple round-robin for now
        # TODO: Implement sophisticated load balancing
        if not hasattr(self, '_last_selected_index'):
            self._last_selected_index = 0

        self._last_selected_index = (self._last_selected_index + 1) % len(candidates)
        return candidates[self._last_selected_index]

    def _get_fallback_providers(
            self,
            request: CompletionRequest,
            exclude: List[LLMProvider]
    ) -> List[LLMProvider]:
        """Get ordered list of fallback providers"""
        fallback = []

        for provider in self._providers:
            if provider not in exclude:
                fallback.append(provider)

        # Sort by reliability and cost
        # TODO: Implement sophisticated ranking
        return fallback

    def _update_usage_stats(
            self,
            provider: LLMProvider,
            usage: TokenUsage
    ) -> None:
        """Update usage statistics"""
        if provider not in self._usage_stats:
            self._usage_stats[provider] = UsageStats()

        stats = self._usage_stats[provider]
        stats.total_requests += 1
        stats.total_tokens += usage.total_tokens
        stats.last_used = datetime.utcnow()

    async def stream_complete(
            self,
            request: CompletionRequest,
            provider: Optional[LLMProvider] = None
    ) -> AsyncIterator[CompletionResponse]:
        """Stream completion with intelligent routing"""
        # Determine provider
        if not provider:
            provider = self._select_provider(request)

        llm = self._providers.get(provider)
        if not llm:
            raise ValueError(f"Provider {provider} not available")

        # Validate and stream
        llm.validate_request(request)

        total_usage = TokenUsage()
        async for chunk in llm.stream_complete(request):
            # Accumulate usage
            total_usage.prompt_tokens += chunk.usage.prompt_tokens
            total_usage.completion_tokens += chunk.usage.completion_tokens
            total_usage.total_tokens += chunk.usage.total_tokens

            yield chunk

        # Update stats after streaming
        self._update_usage_stats(provider, total_usage)

    async def embed(
            self,
            request: EmbeddingRequest,
            provider: Optional[LLMProvider] = None
    ) -> EmbeddingResponse:
        """Generate embeddings with intelligent routing"""
        # Select provider that supports embeddings
        if not provider:
            for p, llm in self._providers.items():
                if any(
                        ModelCapability.EMBEDDING in model.capabilities
                        for model in llm._models.values()
                ):
                    provider = p
                    break

        if not provider:
            raise ValueError("No embedding provider available")

        llm = self._providers[provider]
        return await llm.embed(request)

    def register_model_routing(
            self,
            model_id: str,
            provider: LLMProvider
    ) -> None:
        """Register specific model to provider routing"""
        self._model_routing[model_id] = provider

    def get_usage_stats(self) -> Dict[str, Dict[str, Any]]:
        """Get usage statistics for all providers"""
        return {
            provider.value: {
                "total_requests": stats.total_requests,
                "total_tokens": stats.total_tokens,
                "last_used": stats.last_used.isoformat() if stats.last_used else None
            }
            for provider, stats in self._usage_stats.items()
        }

    async def list_all_models(self) -> List[ModelInfo]:
        """List all available models across providers"""
        all_models = []

        for provider, llm in self._providers.items():
            try:
                models = await llm.list_models()
                all_models.extend(models)
            except Exception as e:
                logger.error(f"Failed to list models for {provider}: {e}")

        return all_models

    async def close(self) -> None:
        """Close all providers"""
        for provider, llm in self._providers.items():
            try:
                await llm.close()
            except Exception as e:
                logger.error(f"Error closing {provider}: {e}")


@dataclass
class UsageStats:
    """Provider usage statistics"""
    total_requests: int = 0
    total_tokens: int = 0
    total_errors: int = 0
    last_used: Optional[datetime] = None
    last_error: Optional[datetime] = None


class LLMCache:
    """Simple LRU cache for LLM responses"""

    def __init__(self, max_size: int = 1000, ttl: int = 3600):
        self.max_size = max_size
        self.ttl = ttl
        self._cache: Dict[str, tuple[CompletionResponse, datetime]] = {}
        self._access_order: List[str] = []
        self._lock = asyncio.Lock()

    def _get_cache_key(self, request: CompletionRequest) -> str:
        """Generate cache key from request"""
        # Create deterministic key from request
        key_parts = [
            request.model or "default",
            str(request.temperature),
            str(request.max_tokens),
            str(request.top_p),
            json.dumps([msg.model_dump() for msg in request.messages], sort_keys=True)
        ]

        import hashlib
        key_str = "|".join(key_parts)
        return hashlib.sha256(key_str.encode()).hexdigest()

    async def get(
            self,
            request: CompletionRequest
    ) -> Optional[CompletionResponse]:
        """Get cached response"""
        key = self._get_cache_key(request)

        async with self._lock:
            if key in self._cache:
                response, timestamp = self._cache[key]

                # Check TTL
                if (datetime.utcnow() - timestamp).total_seconds() < self.ttl:
                    # Update access order
                    self._access_order.remove(key)
                    self._access_order.append(key)
                    return response
                else:
                    # Expired
                    del self._cache[key]
                    self._access_order.remove(key)

        return None

    async def set(
            self,
            request: CompletionRequest,
            response: CompletionResponse
    ) -> None:
        """Cache response"""
        key = self._get_cache_key(request)

        async with self._lock:
            # Evict if at capacity
            if len(self._cache) >= self.max_size and key not in self._cache:
                # Remove least recently used
                lru_key = self._access_order.pop(0)
                del self._cache[lru_key]

            self._cache[key] = (response, datetime.utcnow())

            if key in self._access_order:
                self._access_order.remove(key)
            self._access_order.append(key)

    async def clear(self) -> None:
        """Clear cache"""
        async with self._lock:
            self._cache.clear()
            self._access_order.clear()


# Export public API
__all__ = [
    "BaseLLM",
    "LLMManager",
    "LLMProvider",
    "ModelCapability",
    "Message",
    "CompletionRequest",
    "CompletionResponse",
    "Choice",
    "TokenUsage",
    "EmbeddingRequest",
    "EmbeddingResponse",
    "ModelInfo",
    "PricingInfo",
    "LLMCache"
]