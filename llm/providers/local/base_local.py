# File: maple/llm/providers/local/base_local.py
# Description: Base class for local LLM providers.

from __future__ import annotations
import asyncio
import os
import subprocess
import sys
from pathlib import Path
from typing import List, Optional, Dict, Any, AsyncIterator
import logging
from abc import abstractmethod

import torch
import transformers
from transformers import (
    AutoModelForCausalLM, AutoTokenizer,
    TextIteratorStreamer, GenerationConfig
)
from huggingface_hub import hf_hub_download, snapshot_download

from maple.core.llm.base import (
    BaseLLM, LLMProvider, ModelCapability, ModelInfo,
    CompletionRequest, CompletionResponse, Message, Choice, TokenUsage,
    EmbeddingRequest, EmbeddingResponse
)

logger = logging.getLogger(__name__)


class BaseLocalLLM(BaseLLM):
    """Base class for local LLM providers"""

    def __init__(
            self,
            provider: LLMProvider,
            config: Optional[Dict[str, Any]] = None
    ):
        super().__init__(provider, config)

        # Model configuration
        self.model_path = config.get('model_path')
        self.model_name = config.get('model_name')
        self.device = config.get('device', 'cuda' if torch.cuda.is_available() else 'cpu')
        self.precision = config.get('precision', 'float16')
        self.max_memory = config.get('max_memory')
        self.load_in_8bit = config.get('load_in_8bit', False)
        self.load_in_4bit = config.get('load_in_4bit', False)
        self.use_flash_attention = config.get('use_flash_attention', True)
        self.trust_remote_code = config.get('trust_remote_code', False)

        # Generation defaults
        self.default_max_tokens = config.get('default_max_tokens', 2048)
        self.default_temperature = config.get('default_temperature', 0.7)

        # Model and tokenizer
        self.model = None
        self.tokenizer = None
        self.generation_config = None

        # Model info
        self.context_window = 4096  # Default, override in subclasses
        self.max_output = 2048

    @abstractmethod
    def get_model_mappings(self) -> Dict[str, Dict[str, Any]]:
        """Get model name to HuggingFace ID mappings"""
        pass

    async def initialize(self) -> None:
        """Initialize local model"""
        if not self.model_name and not self.model_path:
            raise ValueError("Either model_name or model_path must be specified")

        # Download model if needed
        if self.model_name and not self.model_path:
            self.model_path = await self._download_model(self.model_name)

        # Load model and tokenizer
        await self._load_model()

        # Set up model info
        self._setup_model_info()

        self._initialized = True
        logger.info(f"{self.provider} local model initialized")

    async def _download_model(self, model_name: str) -> str:
        """Download model from HuggingFace"""
        mappings = self.get_model_mappings()

        if model_name not in mappings:
            raise ValueError(f"Unknown model: {model_name}")

        model_info = mappings[model_name]
        hf_model_id = model_info["hf_id"]

        # Set up cache directory
        cache_dir = Path.home() / ".cache" / "maple" / "models" / self.provider.value
        cache_dir.mkdir(parents=True, exist_ok=True)

        model_path = cache_dir / model_name

        if model_path.exists():
            logger.info(f"Model already downloaded: {model_path}")
            return str(model_path)

        logger.info(f"Downloading model {model_name} from {hf_model_id}")

        try:
            # Download model files
            model_path = await asyncio.to_thread(
                snapshot_download,
                repo_id=hf_model_id,
                cache_dir=str(cache_dir),
                local_dir=str(model_path),
                local_dir_use_symlinks=False
            )

            logger.info(f"Model downloaded to: {model_path}")
            return model_path

        except Exception as e:
            logger.error(f"Failed to download model: {e}")
            raise

    async def _load_model(self) -> None:
        """Load model and tokenizer"""
        logger.info(f"Loading model from {self.model_path}")

        # Load tokenizer
        self.tokenizer = await asyncio.to_thread(
            AutoTokenizer.from_pretrained,
            self.model_path,
            trust_remote_code=self.trust_remote_code
        )

        # Set up model loading kwargs
        model_kwargs = {
            "trust_remote_code": self.trust_remote_code,
            "device_map": "auto" if self.device == "auto" else None,
            "torch_dtype": self._get_torch_dtype()
        }

        # Add quantization config
        if self.load_in_8bit:
            model_kwargs["load_in_8bit"] = True
        elif self.load_in_4bit:
            from transformers import BitsAndBytesConfig
            model_kwargs["quantization_config"] = BitsAndBytesConfig(
                load_in_4bit=True,
                bnb_4bit_compute_dtype=torch.float16,
                bnb_4bit_use_double_quant=True,
                bnb_4bit_quant_type="nf4"
            )

        # Add memory config
        if self.max_memory:
            model_kwargs["max_memory"] = self.max_memory

        # Load model
        self.model = await asyncio.to_thread(
            AutoModelForCausalLM.from_pretrained,
            self.model_path,
            **model_kwargs
        )

        # Move to device if not using device_map
        if self.device != "auto" and not self.load_in_8bit and not self.load_in_4bit:
            self.model = self.model.to(self.device)

        # Set up generation config
        if hasattr(self.model, "generation_config"):
            self.generation_config = self.model.generation_config
        else:
            self.generation_config = GenerationConfig()

        logger.info(f"Model loaded successfully on {self.device}")

    def _get_torch_dtype(self) -> torch.dtype:
        """Get torch dtype from precision string"""
        dtype_map = {
            "float32": torch.float32,
            "float16": torch.float16,
            "bfloat16": torch.bfloat16,
            "int8": torch.int8
        }
        return dtype_map.get(self.precision, torch.float16)

    def _setup_model_info(self) -> None:
        """Set up model information"""
        # Try to get context window from model config
        if hasattr(self.model.config, "max_position_embeddings"):
            self.context_window = self.model.config.max_position_embeddings
        elif hasattr(self.model.config, "max_length"):
            self.context_window = self.model.config.max_length

        # Create model info
        model_id = self.model_name or Path(self.model_path).name

        self._models[model_id] = ModelInfo(
            id=model_id,
            provider=self.provider,
            capabilities=[
                ModelCapability.CHAT,
                ModelCapability.COMPLETION,
                ModelCapability.CODE,
                ModelCapability.STREAMING,
                ModelCapability.REASONING
            ],
            context_window=self.context_window,
            max_output_tokens=self.max_output,
            supports_functions=False,  # Can be added via prompting
            supports_tools=False,
            supports_vision=False,  # Override in multimodal models
            supports_streaming=True
        )

    async def complete(
            self,
            request: CompletionRequest
    ) -> CompletionResponse:
        """Generate completion using local model"""
        # Convert messages to prompt
        prompt = self._messages_to_prompt(request.messages)

        # Tokenize
        inputs = self.tokenizer(
            prompt,
            return_tensors="pt",
            truncation=True,
            max_length=self.context_window
        )

        if self.device != "cpu":
            inputs = {k: v.to(self.device) for k, v in inputs.items()}

        # Set up generation parameters
        gen_kwargs = {
            "max_new_tokens": request.max_tokens or self.default_max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "do_sample": request.temperature > 0,
            "num_return_sequences": request.n,
            "pad_token_id": self.tokenizer.pad_token_id,
            "eos_token_id": self.tokenizer.eos_token_id
        }

        # Add stop sequences
        if request.stop:
            if isinstance(request.stop, str):
                stop_ids = self.tokenizer.encode(request.stop, add_special_tokens=False)
                gen_kwargs["eos_token_id"] = stop_ids
            else:
                # Multiple stop sequences
                stop_ids = []
                for stop in request.stop:
                    stop_ids.extend(self.tokenizer.encode(stop, add_special_tokens=False))
                gen_kwargs["eos_token_id"] = stop_ids

        # Generate
        try:
            with torch.no_grad():
                outputs = await asyncio.to_thread(
                    self.model.generate,
                    **inputs,
                    **gen_kwargs
                )

            # Decode outputs
            prompt_length = inputs["input_ids"].shape[1]
            choices = []

            for i, output in enumerate(outputs):
                # Get only the generated tokens
                generated_ids = output[prompt_length:]
                generated_text = self.tokenizer.decode(
                    generated_ids,
                    skip_special_tokens=True,
                    clean_up_tokenization_spaces=True
                )

                choices.append(Choice(
                    index=i,
                    message=Message(
                        role="assistant",
                        content=generated_text
                    ),
                    finish_reason="stop"
                ))

            # Calculate token usage
            prompt_tokens = prompt_length
            completion_tokens = len(generated_ids)

            return CompletionResponse(
                id=f"{self.provider}-{datetime.utcnow().timestamp()}",
                model=self.model_name or "local",
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
            logger.error(f"Local model completion error: {e}")
            raise

    async def stream_complete(
            self,
            request: CompletionRequest
    ) -> AsyncIterator[CompletionResponse]:
        """Stream completion from local model"""
        # Convert messages to prompt
        prompt = self._messages_to_prompt(request.messages)

        # Tokenize
        inputs = self.tokenizer(
            prompt,
            return_tensors="pt",
            truncation=True,
            max_length=self.context_window
        )

        if self.device != "cpu":
            inputs = {k: v.to(self.device) for k, v in inputs.items()}

        # Set up generation parameters
        gen_kwargs = {
            "max_new_tokens": request.max_tokens or self.default_max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "do_sample": request.temperature > 0,
            "pad_token_id": self.tokenizer.pad_token_id,
            "eos_token_id": self.tokenizer.eos_token_id
        }

        # Create streamer
        streamer = TextIteratorStreamer(
            self.tokenizer,
            skip_prompt=True,
            skip_special_tokens=True
        )

        # Start generation in background
        generation_task = asyncio.create_task(
            asyncio.to_thread(
                self.model.generate,
                **inputs,
                **gen_kwargs,
                streamer=streamer
            )
        )

        # Stream tokens
        accumulated_text = ""
        prompt_tokens = inputs["input_ids"].shape[1]
        completion_tokens = 0

        try:
            for token_text in streamer:
                accumulated_text += token_text
                completion_tokens += 1

                yield CompletionResponse(
                    id=f"{self.provider}-stream-{datetime.utcnow().timestamp()}",
                    model=self.model_name or "local",
                    created=datetime.utcnow(),
                    choices=[Choice(
                        index=0,
                        message=Message(
                            role="assistant",
                            content=accumulated_text
                        ),
                        finish_reason=None
                    )],
                    usage=TokenUsage(
                        prompt_tokens=prompt_tokens,
                        completion_tokens=completion_tokens,
                        total_tokens=prompt_tokens + completion_tokens
                    ),
                    provider=self.provider
                )

            # Wait for generation to complete
            await generation_task

            # Final response
            yield CompletionResponse(
                id=f"{self.provider}-stream-{datetime.utcnow().timestamp()}",
                model=self.model_name or "local",
                created=datetime.utcnow(),
                choices=[Choice(
                    index=0,
                    message=Message(
                        role="assistant",
                        content=accumulated_text
                    ),
                    finish_reason="stop"
                )],
                usage=TokenUsage(
                    prompt_tokens=prompt_tokens,
                    completion_tokens=completion_tokens,
                    total_tokens=prompt_tokens + completion_tokens
                ),
                provider=self.provider
            )

        except Exception as e:
            logger.error(f"Local model streaming error: {e}")
            raise

    async def embed(
            self,
            request: EmbeddingRequest
    ) -> EmbeddingResponse:
        """Local models typically don't provide embeddings"""
        raise NotImplementedError(f"{self.provider} does not support embeddings")

    async def list_models(self) -> List[ModelInfo]:
        """List available local models"""
        return list(self._models.values())

    @abstractmethod
    def _messages_to_prompt(self, messages: List[Message]) -> str:
        """Convert messages to model-specific prompt format"""
        pass

    def _validate_provider_request(self, request: CompletionRequest) -> None:
        """Validate local model request"""
        if not self.model:
            raise RuntimeError("Model not loaded")

        # Check context window
        prompt = self._messages_to_prompt(request.messages)
        tokens = self.tokenizer.encode(prompt)

        if len(tokens) > self.context_window:
            raise ValueError(
                f"Prompt too long: {len(tokens)} tokens "
                f"(max: {self.context_window})"
            )