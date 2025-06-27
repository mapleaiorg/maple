# File: maple/llm/providers/local/other_models.py
# Description: Other local model providers (Falcon, MPT, Alpaca).

from __future__ import annotations
from typing import List, Optional, Dict, Any
import logging

from maple.core.llm.base import LLMProvider, Message
from maple.core.llm.providers.local.base_local import BaseLocalLLM

logger = logging.getLogger(__name__)


class FalconProvider(BaseLocalLLM):
    """Falcon model provider"""

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        super().__init__(LLMProvider.FALCON, config)

    def get_model_mappings(self) -> Dict[str, Dict[str, Any]]:
        """Get Falcon model mappings"""
        return {
            "falcon-7b": {
                "hf_id": "tiiuae/falcon-7b-instruct",
                "context_window": 2048,
                "format": "falcon"
            },
            "falcon-40b": {
                "hf_id": "tiiuae/falcon-40b-instruct",
                "context_window": 2048,
                "format": "falcon"
            },
            "falcon-180b": {
                "hf_id": "tiiuae/falcon-180B-chat",
                "context_window": 2048,
                "format": "falcon"
            }
        }

    def _messages_to_prompt(self, messages: List[Message]) -> str:
        """Convert messages to Falcon format"""
        prompt = ""

        for message in messages:
            if message.role == "system":
                prompt += f"System: {message.content}\n"
            elif message.role == "user":
                prompt += f"User: {message.content}\n"
            elif message.role == "assistant":
                prompt += f"Assistant: {message.content}\n"

        prompt += "Assistant:"
        return prompt


class MPTProvider(BaseLocalLLM):
    """MPT (MosaicML) model provider"""

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        super().__init__(LLMProvider.MPT, config)

    def get_model_mappings(self) -> Dict[str, Dict[str, Any]]:
        """Get MPT model mappings"""
        return {
            "mpt-7b": {
                "hf_id": "mosaicml/mpt-7b-instruct",
                "context_window": 2048,
                "format": "mpt"
            },
            "mpt-30b": {
                "hf_id": "mosaicml/mpt-30b-instruct",
                "context_window": 8192,
                "format": "mpt"
            }
        }

    def _messages_to_prompt(self, messages: List[Message]) -> str:
        """Convert messages to MPT format"""
        INSTRUCTION_KEY = "### Instruction:"
        RESPONSE_KEY = "### Response:"

        prompt = ""
        system_msg = ""

        # Extract system message
        for msg in messages:
            if msg.role == "system":
                system_msg = msg.content
                break

        # Build conversation
        for i in range(0, len(messages), 2):
            if messages[i].role == "system":
                continue

            user_msg = messages[i].content if i < len(messages) else ""
            assistant_msg = messages[i + 1].content if i + 1 < len(messages) and messages[
                i + 1].role == "assistant" else ""

            if system_msg and i == 0:
                prompt += f"{INSTRUCTION_KEY}\n{system_msg}\n\n{user_msg}\n\n{RESPONSE_KEY}\n{assistant_msg}"
            else:
                prompt += f"{INSTRUCTION_KEY}\n{user_msg}\n\n{RESPONSE_KEY}\n{assistant_msg}"

            if assistant_msg:
                prompt += "\n\n"

        if not prompt.endswith(RESPONSE_KEY + "\n"):
            prompt += f"{RESPONSE_KEY}\n"

        return prompt


class AlpacaProvider(BaseLocalLLM):
    """Alpaca model provider (LLaMA fine-tune)"""

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        super().__init__(LLMProvider.ALPACA, config)

    def get_model_mappings(self) -> Dict[str, Dict[str, Any]]:
        """Get Alpaca model mappings"""
        return {
            "alpaca-7b": {
                "hf_id": "chavinlo/alpaca-native",
                "context_window": 2048,
                "format": "alpaca"
            },
            "alpaca-lora-7b": {
                "hf_id": "tloen/alpaca-lora-7b",
                "context_window": 2048,
                "format": "alpaca"
            }
        }

    def _messages_to_prompt(self, messages: List[Message]) -> str:
        """Convert messages to Alpaca format"""
        prompt = ""

        for message in messages:
            if message.role == "system":
                # Include as instruction context
                prompt = f"Below is an instruction that describes a task. {message.content}\n\n"
            elif message.role == "user":
                prompt += f"### Instruction:\n{message.content}\n\n"
            elif message.role == "assistant":
                prompt += f"### Response:\n{message.content}\n\n"

        if not prompt.endswith("### Response:\n"):
            prompt += "### Response:\n"

        return prompt