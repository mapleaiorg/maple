# File: llm/providers/local/llama.py
# Description: Llama local model provider implementation.

from __future__ import annotations
from typing import List, Optional, Dict, Any
import logging

from llm.base import LLMProvider, Message
from llm.providers.local.base_local import BaseLocalLLM

logger = logging.getLogger(__name__)


class LlamaProvider(BaseLocalLLM):
    """Llama/Llama2/Llama3 local model provider"""

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        super().__init__(LLMProvider.LLAMA, config)
        self.use_llama3_format = config.get('use_llama3_format', True)

    def get_model_mappings(self) -> Dict[str, Dict[str, Any]]:
        """Get Llama model mappings"""
        return {
            # Llama 3 models
            "llama-3-8b": {
                "hf_id": "meta-llama/Meta-Llama-3-8B-Instruct",
                "context_window": 8192,
                "format": "llama3"
            },
            "llama-3-70b": {
                "hf_id": "meta-llama/Meta-Llama-3-70B-Instruct",
                "context_window": 8192,
                "format": "llama3"
            },

            # Llama 2 models
            "llama-2-7b": {
                "hf_id": "meta-llama/Llama-2-7b-chat-hf",
                "context_window": 4096,
                "format": "llama2"
            },
            "llama-2-13b": {
                "hf_id": "meta-llama/Llama-2-13b-chat-hf",
                "context_window": 4096,
                "format": "llama2"
            },
            "llama-2-70b": {
                "hf_id": "meta-llama/Llama-2-70b-chat-hf",
                "context_window": 4096,
                "format": "llama2"
            },

            # Code Llama models
            "codellama-7b": {
                "hf_id": "codellama/CodeLlama-7b-Instruct-hf",
                "context_window": 16384,
                "format": "llama2"
            },
            "codellama-13b": {
                "hf_id": "codellama/CodeLlama-13b-Instruct-hf",
                "context_window": 16384,
                "format": "llama2"
            },
            "codellama-34b": {
                "hf_id": "codellama/CodeLlama-34b-Instruct-hf",
                "context_window": 16384,
                "format": "llama2"
            }
        }

    def _messages_to_prompt(self, messages: List[Message]) -> str:
        """Convert messages to Llama prompt format"""
        # Determine format based on model
        model_info = self.get_model_mappings().get(self.model_name, {})
        format_type = model_info.get("format", "llama3" if self.use_llama3_format else "llama2")

        if format_type == "llama3":
            return self._format_llama3_prompt(messages)
        else:
            return self._format_llama2_prompt(messages)

    def _format_llama3_prompt(self, messages: List[Message]) -> str:
        """Format messages for Llama 3 models"""
        prompt = ""

        for message in messages:
            if message.role == "system":
                prompt += f"<|start_header_id|>system<|end_header_id|>\n\n{message.content}<|eot_id|>"
            elif message.role == "user":
                prompt += f"<|start_header_id|>user<|end_header_id|>\n\n{message.content}<|eot_id|>"
            elif message.role == "assistant":
                prompt += f"<|start_header_id|>assistant<|end_header_id|>\n\n{message.content}<|eot_id|>"

        # Add final assistant header for generation
        prompt += "<|start_header_id|>assistant<|end_header_id|>\n\n"

        return prompt

    def _format_llama2_prompt(self, messages: List[Message]) -> str:
        """Format messages for Llama 2 models"""
        B_INST, E_INST = "[INST]", "[/INST]"
        B_SYS, E_SYS = "<<SYS>>\n", "\n<</SYS>>\n\n"

        prompt = ""

        # Extract system message
        system_msg = None
        for msg in messages:
            if msg.role == "system":
                system_msg = msg.content
                break

        # Build conversation
        for i, message in enumerate(messages):
            if message.role == "system":
                continue

            if message.role == "user":
                if i == 0 or (i == 1 and messages[0].role == "system"):
                    # First user message, include system prompt
                    if system_msg:
                        prompt += f"{B_INST} {B_SYS}{system_msg}{E_SYS}{message.content} {E_INST}"
                    else:
                        prompt += f"{B_INST} {message.content} {E_INST}"
                else:
                    prompt += f"{B_INST} {message.content} {E_INST}"

            elif message.role == "assistant":
                prompt += f" {message.content} </s>"

        return prompt


# Additional specialized Llama variants

class CodeLlamaProvider(LlamaProvider):
    """Specialized provider for Code Llama models"""

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        config = config or {}
        config['model_name'] = config.get('model_name', 'codellama-7b')
        super().__init__(config)
        self.provider = LLMProvider.CODELLAMA

    def _messages_to_prompt(self, messages: List[Message]) -> str:
        """Convert messages to Code Llama format"""
        # Code Llama can use special tokens for code
        prompt = super()._messages_to_prompt(messages)

        # Add code-specific formatting if needed
        # For example, wrap code blocks in special tokens
        return prompt


class VicunaProvider(LlamaProvider):
    """Provider for Vicuna models (Llama fine-tune)"""

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        super().__init__(config)
        self.provider = LLMProvider.VICUNA

    def get_model_mappings(self) -> Dict[str, Dict[str, Any]]:
        """Get Vicuna model mappings"""
        return {
            "vicuna-7b": {
                "hf_id": "lmsys/vicuna-7b-v1.5",
                "context_window": 4096,
                "format": "vicuna"
            },
            "vicuna-13b": {
                "hf_id": "lmsys/vicuna-13b-v1.5",
                "context_window": 4096,
                "format": "vicuna"
            },
            "vicuna-33b": {
                "hf_id": "lmsys/vicuna-33b-v1.3",
                "context_window": 2048,
                "format": "vicuna"
            }
        }

    def _messages_to_prompt(self, messages: List[Message]) -> str:
        """Convert messages to Vicuna format"""
        prompt = ""

        for message in messages:
            if message.role == "system":
                prompt += f"SYSTEM: {message.content}\n"
            elif message.role == "user":
                prompt += f"USER: {message.content}\n"
            elif message.role == "assistant":
                prompt += f"ASSISTANT: {message.content}\n"

        # Add final assistant prompt
        prompt += "ASSISTANT:"

        return prompt