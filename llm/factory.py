# File: llm/factory.py
# Description: Factory for creating LLM providers.

from __future__ import annotations
from typing import Optional, Dict, Any
import logging

from llm.base import BaseLLM, LLMProvider
from llm.providers.openai import OpenAIProvider
from llm.providers.anthropic import AnthropicProvider
from llm.providers.google import GoogleProvider
from llm.providers.local.llama import (
    LlamaProvider, CodeLlamaProvider, VicunaProvider
)
from llm.providers.local.mistral import MistralProvider, MixtralProvider
from llm.providers.local.other_models import (
    FalconProvider, MPTProvider, AlpacaProvider
)

logger = logging.getLogger(__name__)


class LLMProviderFactory:
    """Factory for creating LLM provider instances"""

    # Provider mappings
    PROVIDER_CLASSES = {
        LLMProvider.OPENAI: OpenAIProvider,
        LLMProvider.ANTHROPIC: AnthropicProvider,
        LLMProvider.GOOGLE: GoogleProvider,
        LLMProvider.LLAMA: LlamaProvider,
        LLMProvider.MISTRAL: MistralProvider,
        LLMProvider.MIXTRAL: MixtralProvider,
        LLMProvider.CODELLAMA: CodeLlamaProvider,
        LLMProvider.VICUNA: VicunaProvider,
        LLMProvider.ALPACA: AlpacaProvider,
        LLMProvider.FALCON: FalconProvider,
        LLMProvider.MPT: MPTProvider
    }

    @classmethod
    def create_provider(
            cls,
            provider: LLMProvider,
            config: Optional[Dict[str, Any]] = None
    ) -> BaseLLM:
        """
        Create an LLM provider instance.

        Args:
            provider: Provider type
            config: Provider configuration

        Returns:
            LLM provider instance
        """
        if provider not in cls.PROVIDER_CLASSES:
            raise ValueError(f"Unknown provider: {provider}")

        provider_class = cls.PROVIDER_CLASSES[provider]
        return provider_class(config)

    @classmethod
    def create_from_config(
            cls,
            config: Dict[str, Any]
    ) -> BaseLLM:
        """
        Create provider from configuration dictionary.

        Args:
            config: Configuration with 'provider' key

        Returns:
            LLM provider instance
        """
        provider_name = config.get('provider')
        if not provider_name:
            raise ValueError("Provider not specified in config")

        provider = LLMProvider(provider_name)
        return cls.create_provider(provider, config)

    @classmethod
    def get_available_providers(cls) -> List[str]:
        """Get list of available provider names"""
        return [p.value for p in cls.PROVIDER_CLASSES.keys()]

    @classmethod
    def get_provider_info(cls, provider: LLMProvider) -> Dict[str, Any]:
        """Get information about a provider"""
        if provider not in cls.PROVIDER_CLASSES:
            raise ValueError(f"Unknown provider: {provider}")

        provider_class = cls.PROVIDER_CLASSES[provider]

        info = {
            "provider": provider.value,
            "class": provider_class.__name__,
            "is_local": provider in [
                LLMProvider.LLAMA, LLMProvider.MISTRAL, LLMProvider.MIXTRAL,
                LLMProvider.CODELLAMA, LLMProvider.VICUNA, LLMProvider.ALPACA,
                LLMProvider.FALCON, LLMProvider.MPT
            ],
            "requires_api_key": provider in [
                LLMProvider.OPENAI, LLMProvider.ANTHROPIC, LLMProvider.GOOGLE
            ]
        }

        # Add model list for local providers
        if info["is_local"]:
            try:
                instance = provider_class({})
                if hasattr(instance, 'get_model_mappings'):
                    info["available_models"] = list(instance.get_model_mappings().keys())
            except:
                pass

        return info


# Convenience function
def create_llm_provider(
        provider: str,
        **kwargs
) -> BaseLLM:
    """
    Convenience function to create an LLM provider.

    Args:
        provider: Provider name (string)
        **kwargs: Provider configuration

    Returns:
        LLM provider instance
    """
    return LLMProviderFactory.create_provider(
        LLMProvider(provider),
        kwargs
    )


# Export public API
__all__ = [
    "LLMProviderFactory",
    "create_llm_provider"
]