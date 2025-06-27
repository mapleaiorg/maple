# File: maple/ai_agent/adapters/base.py
# Description: Base adapter interface and registry for LLM/AGI connections.
# Provides abstract interface that all model adapters must implement.

from abc import ABC, abstractmethod
from typing import Dict, Any, List, Optional, Union
from dataclasses import dataclass
import asyncio
import logging

logger = logging.getLogger(__name__)


@dataclass
class ModelResponse:
    """Standard response format from LLM/AGI models"""
    text: str
    model: str
    usage: Dict[str, int]  # tokens used, etc.
    metadata: Dict[str, Any] = None
    raw_response: Any = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "text": self.text,
            "model": self.model,
            "usage": self.usage,
            "metadata": self.metadata or {}
        }


class LLMAdapter(ABC):
    """Abstract base class for LLM/AGI adapters"""

    def __init__(self, config: Dict[str, Any]):
        self.config = config
        self.name = config.get("name", self.__class__.__name__)
        self.api_key = config.get("api_key")
        self.endpoint = config.get("endpoint")
        self.timeout = config.get("timeout", 30)
        self.max_retries = config.get("max_retries", 3)

        # Connection pool for reuse
        self._connection_pool = None
        self._initialized = False

    async def initialize(self):
        """Initialize adapter (connect to service, validate credentials, etc.)"""
        if not self._initialized:
            await self._connect()
            self._initialized = True

    @abstractmethod
    async def _connect(self):
        """Establish connection to the LLM/AGI service"""
        pass

    @abstractmethod
    async def query(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[List[Dict[str, str]]] = None
    ) -> ModelResponse:
        """Query the model with prompt and optional context"""
        pass

    @abstractmethod
    async def stream_query(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[List[Dict[str, str]]] = None
    ):
        """Stream responses from the model"""
        pass

    @abstractmethod
    def get_capabilities(self) -> List[str]:
        """Get list of capabilities this model supports"""
        pass

    @abstractmethod
    def get_model_info(self) -> Dict[str, Any]:
        """Get information about the model"""
        pass

    async def health_check(self) -> bool:
        """Check if the adapter is healthy and can connect"""
        try:
            # Simple query to test connection
            response = await self.query(
                "Hello",
                parameters={"max_tokens": 5}
            )
            return response is not None
        except Exception as e:
            logger.error(f"Health check failed for {self.name}: {e}")
            return False

    async def close(self):
        """Clean up resources"""
        if self._connection_pool:
            await self._connection_pool.close()
        self._initialized = False

    def _build_messages(
            self,
            prompt: str,
            context: Optional[List[Dict[str, str]]] = None
    ) -> List[Dict[str, str]]:
        """Build message list from prompt and context"""
        messages = []

        # Add context messages
        if context:
            messages.extend(context)

        # Add current prompt
        messages.append({
            "role": "user",
            "content": prompt
        })

        return messages

    async def _retry_with_backoff(
            self,
            func,
            *args,
            **kwargs
    ):
        """Retry function with exponential backoff"""

        for attempt in range(self.max_retries):
            try:
                return await func(*args, **kwargs)
            except Exception as e:
                if attempt == self.max_retries - 1:
                    raise

                wait_time = 2 ** attempt
                logger.warning(
                    f"Attempt {attempt + 1} failed for {self.name}: {e}. "
                    f"Retrying in {wait_time}s..."
                )
                await asyncio.sleep(wait_time)


class AdapterRegistry:
    """Registry for managing LLM/AGI adapters"""

    def __init__(self):
        self.adapters: Dict[str, LLMAdapter] = {}
        self._adapter_classes: Dict[str, type] = {}

        # Register built-in adapters
        self._register_builtin_adapters()

    def _register_builtin_adapters(self):
        """Register built-in adapter classes"""
        from .openai import OpenAIAdapter
        from .anthropic import AnthropicAdapter
        from .local import LocalModelAdapter

        self._adapter_classes["openai"] = OpenAIAdapter
        self._adapter_classes["anthropic"] = AnthropicAdapter
        self._adapter_classes["local"] = LocalModelAdapter

    def register_adapter_class(self, name: str, adapter_class: type):
        """Register a new adapter class"""
        if not issubclass(adapter_class, LLMAdapter):
            raise ValueError(
                f"Adapter class must inherit from LLMAdapter"
            )
        self._adapter_classes[name] = adapter_class

    def create_adapter(
            self,
            adapter_type: str,
            config: Dict[str, Any]
    ) -> LLMAdapter:
        """Create an adapter instance"""

        if adapter_type not in self._adapter_classes:
            raise ValueError(f"Unknown adapter type: {adapter_type}")

        adapter_class = self._adapter_classes[adapter_type]
        return adapter_class(config)

    def register(self, name: str, adapter: LLMAdapter):
        """Register an adapter instance"""
        self.adapters[name] = adapter

    def unregister(self, name: str):
        """Unregister an adapter"""
        if name in self.adapters:
            del self.adapters[name]

    def get(self, name: str) -> Optional[LLMAdapter]:
        """Get adapter by name"""
        return self.adapters.get(name)

    def list_adapters(self) -> List[str]:
        """List all registered adapters"""
        return list(self.adapters.keys())

    def list_adapter_types(self) -> List[str]:
        """List all available adapter types"""
        return list(self._adapter_classes.keys())

    async def initialize_all(self):
        """Initialize all registered adapters"""
        tasks = [
            adapter.initialize()
            for adapter in self.adapters.values()
        ]
        await asyncio.gather(*tasks, return_exceptions=True)

    async def close_all(self):
        """Close all adapters"""
        tasks = [
            adapter.close()
            for adapter in self.adapters.values()
        ]
        await asyncio.gather(*tasks, return_exceptions=True)