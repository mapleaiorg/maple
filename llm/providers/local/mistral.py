# File: llm/providers/local/mistral.py
# Description: Mistral local model provider implementation.

from __future__ import annotations
from typing import List, Optional, Dict, Any
import logging

from llm.base import LLMProvider, Message
from llm.providers.local.base_local import BaseLocalLLM

logger = logging.getLogger(__name__)


class MistralProvider(BaseLocalLLM):
    """Mistral/Mixtral local model provider"""

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        super().__init__(LLMProvider.MISTRAL, config)

    def get_model_mappings(self) -> Dict[str, Dict[str, Any]]:
        """Get Mistral model mappings"""
        return {
            # Mistral models
            "mistral-7b": {
                "hf_id": "mistralai/Mistral-7B-Instruct-v0.2",
                "context_window": 32768,
                "format": "mistral"
            },
            "mistral-7b-v0.1": {
                "hf_id": "mistralai/Mistral-7B-Instruct-v0.1",
                "context_window": 8192,
                "format": "mistral"
            },

            # Mixtral models
            "mixtral-8x7b": {
                "hf_id": "mistralai/Mixtral-8x7B-Instruct-v0.1",
                "context_window": 32768,
                "format": "mistral"
            },
            "mixtral-8x22b": {
                "hf_id": "mistralai/Mixtral-8x22B-Instruct-v0.1",
                "context_window": 65536,
                "format": "mistral"
            },

            # OpenHermes (Mistral fine-tune)
            "openhermes-2.5": {
                "hf_id": "teknium/OpenHermes-2.5-Mistral-7B",
                "context_window": 8192,
                "format": "chatml"
            },

            # Zephyr (Mistral fine-tune)
            "zephyr-7b": {
                "hf_id": "HuggingFaceH4/zephyr-7b-beta",
                "context_window": 8192,
                "format": "chatml"
            }
        }

    def _messages_to_prompt(self, messages: List[Message]) -> str:
        """Convert messages to Mistral prompt format"""
        # Determine format based on model
        model_info = self.get_model_mappings().get(self.model_name, {})
        format_type = model_info.get("format", "mistral")

        if format_type == "mistral":
            return self._format_mistral_prompt(messages)
        elif format_type == "chatml":
            return self._format_chatml_prompt(messages)
        else:
            return self._format_mistral_prompt(messages)

    def _format_mistral_prompt(self, messages: List[Message]) -> str:
        """Format messages for Mistral models"""
        prompt = ""

        for message in messages:
            if message.role == "system":
                # Mistral doesn't have explicit system role, prepend to first user message
                continue
            elif message.role == "user":
                # Check if we need to include system message
                system_content = None
                for msg in messages:
                    if msg.role == "system":
                        system_content = msg.content
                        break

                if system_content and messages.index(message) == 1:  # First user message
                    prompt += f"[INST] {system_content}\n\n{message.content} [/INST]"
                else:
                    prompt += f"[INST] {message.content} [/INST]"
            elif message.role == "assistant":
                prompt += f" {message.content}</s>"

        return prompt

    def _format_chatml_prompt(self, messages: List[Message]) -> str:
        """Format messages for ChatML format (used by some Mistral fine-tunes)"""
        prompt = ""

        for message in messages:
            if message.role == "system":
                prompt += f"<|im_start|>system\n{message.content}<|im_end|>\n"
            elif message.role == "user":
                prompt += f"<|im_start|>user\n{message.content}<|im_end|>\n"
            elif message.role == "assistant":
                prompt += f"<|im_start|>assistant\n{message.content}<|im_end|>\n"

        # Add final assistant prompt
        prompt += "<|im_start|>assistant\n"

        return prompt


class MixtralProvider(MistralProvider):
    """Specialized provider for Mixtral models"""

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        config = config or {}
        config['model_name'] = config.get('model_name', 'mixtral-8x7b')
        super().__init__(config)
        self.provider = LLMProvider.MIXTRAL