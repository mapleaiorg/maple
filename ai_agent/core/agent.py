# File: maple/ai_agent/core/agent.py
# Description: Core AI Agent implementation that manages LLM/AGI connections,
# integrates with MAPLE services, and coordinates agent intelligence.

import asyncio
import json
import time
from typing import Dict, Any, List, Optional, Union
from dataclasses import dataclass, field
from datetime import datetime
import logging

from core.map.models.message import Message, MessageType, MessagePayload
from core.map.client import MAPClient
from core.ual.runtime import UALRuntime
from core.ars.client import ARSClient
from mall.client import MALLClient
from mapleverse.client import MapleverseClient

from ..adapters.base import LLMAdapter, AdapterRegistry
from ..aggregation.ensemble import EnsembleAggregator
from ..cache import ResponseCache
from ..monitoring import AgentMonitor, PerformanceMetrics
from .model_selector import ModelSelector, ModelSelectionStrategy

logger = logging.getLogger(__name__)


@dataclass
class AgentContext:
    """Runtime context for AI agent operations"""
    agent_id: str
    task_id: str
    conversation_history: List[Dict[str, Any]] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)
    start_time: float = field(default_factory=time.time)

    def add_turn(self, role: str, content: str, model: Optional[str] = None):
        """Add a conversation turn to history"""
        self.conversation_history.append({
            "role": role,
            "content": content,
            "model": model,
            "timestamp": datetime.utcnow().isoformat()
        })


class AgentCore:
    """Core module managing AI agent operations and MAPLE integration"""

    def __init__(
            self,
            agent_id: str,
            config: Dict[str, Any],
            map_client: MAPClient,
            ars_client: ARSClient,
            mall_client: MALLClient,
            mapleverse_client: Optional[MapleverseClient] = None
    ):
        self.agent_id = agent_id
        self.config = config

        # MAPLE service clients
        self.map_client = map_client
        self.ars_client = ars_client
        self.mall_client = mall_client
        self.mapleverse_client = mapleverse_client

        # AI components
        self.model_selector = ModelSelector(
            strategy=config.get("selection_strategy", ModelSelectionStrategy.CONTEXT_AWARE),
            mall_client=mall_client
        )
        self.adapter_registry = AdapterRegistry()
        self.aggregator = EnsembleAggregator(
            strategy=config.get("aggregation_strategy", "weighted_average")
        )
        self.cache = ResponseCache(
            strategy=config.get("cache_strategy", "redis"),
            ttl=config.get("cache_ttl", 3600)
        )
        self.monitor = AgentMonitor(agent_id)

        # Initialize adapters
        self._initialize_adapters()

        # Register with ARS
        asyncio.create_task(self._register_with_ars())

    def _initialize_adapters(self):
        """Initialize LLM/AGI adapters from configuration"""
        adapters_config = self.config.get("adapters", {})

        for adapter_name, adapter_config in adapters_config.items():
            adapter_class = adapter_config.get("class")
            if adapter_class:
                adapter = self.adapter_registry.create_adapter(
                    adapter_class,
                    adapter_config
                )
                self.adapter_registry.register(adapter_name, adapter)
                logger.info(f"Initialized adapter: {adapter_name}")

    async def _register_with_ars(self):
        """Register agent capabilities with ARS"""
        capabilities = {
            "llm_models": list(self.adapter_registry.adapters.keys()),
            "supported_tasks": ["text_generation", "reasoning", "analysis"],
            "aggregation_strategies": ["weighted_average", "majority_vote"],
            "performance_metrics": await self.monitor.get_summary()
        }

        await self.ars_client.register_agent(
            agent_id=self.agent_id,
            capabilities=capabilities,
            metadata={
                "type": "ai_agent",
                "version": "0.1.0",
                "created_at": datetime.utcnow().isoformat()
            }
        )

    async def process_task(
            self,
            task: Dict[str, Any],
            context: Optional[AgentContext] = None
    ) -> Dict[str, Any]:
        """Process a task using appropriate LLMs/AGIs"""

        if context is None:
            context = AgentContext(
                agent_id=self.agent_id,
                task_id=task.get("id", "unknown")
            )

        start_time = time.time()

        try:
            # Check cache first
            cache_key = self._generate_cache_key(task)
            cached_response = await self.cache.get(cache_key)
            if cached_response:
                self.monitor.record_cache_hit()
                return cached_response

            # Select models for the task
            selected_models = await self.model_selector.select_models(
                task=task,
                context=context,
                available_models=list(self.adapter_registry.adapters.keys())
            )

            # Query selected models
            model_responses = await self._query_models(
                task=task,
                models=selected_models,
                context=context
            )

            # Aggregate responses
            aggregated_response = await self.aggregator.aggregate(
                responses=model_responses,
                weights=await self._get_model_weights(selected_models)
            )

            # Record metrics
            self.monitor.record_task_completion(
                task_id=task.get("id"),
                duration=time.time() - start_time,
                models_used=selected_models,
                success=True
            )

            # Cache response
            await self.cache.set(cache_key, aggregated_response)

            # Send results via MAP
            await self._share_results_via_map(task, aggregated_response)

            # Update MALL with interaction data
            await self._update_mall(task, model_responses, aggregated_response)

            return aggregated_response

        except Exception as e:
            logger.error(f"Error processing task: {e}")
            self.monitor.record_task_completion(
                task_id=task.get("id"),
                duration=time.time() - start_time,
                models_used=[],
                success=False,
                error=str(e)
            )

            # Try fallback models
            return await self._handle_failure(task, context, e)

    async def _query_models(
            self,
            task: Dict[str, Any],
            models: List[str],
            context: AgentContext
    ) -> List[Dict[str, Any]]:
        """Query multiple models in parallel"""

        async def query_single_model(model_name: str) -> Dict[str, Any]:
            adapter = self.adapter_registry.get(model_name)
            if not adapter:
                raise ValueError(f"Adapter not found: {model_name}")

            start_time = time.time()

            try:
                response = await adapter.query(
                    prompt=task.get("prompt"),
                    parameters=task.get("parameters", {}),
                    context=context.conversation_history
                )

                latency = time.time() - start_time

                return {
                    "model": model_name,
                    "response": response,
                    "latency": latency,
                    "success": True
                }

            except Exception as e:
                logger.error(f"Error querying {model_name}: {e}")
                return {
                    "model": model_name,
                    "response": None,
                    "latency": time.time() - start_time,
                    "success": False,
                    "error": str(e)
                }

        # Query all models in parallel
        tasks = [query_single_model(model) for model in models]
        responses = await asyncio.gather(*tasks)

        # Filter successful responses
        successful_responses = [r for r in responses if r["success"]]

        if not successful_responses:
            raise Exception("All model queries failed")

        return successful_responses

    async def _get_model_weights(self, models: List[str]) -> Dict[str, float]:
        """Get model weights from MALL based on historical performance"""
        weights = {}

        for model in models:
            performance = await self.mall_client.get_model_performance(
                agent_id=self.agent_id,
                model_name=model
            )

            # Calculate weight based on accuracy and latency
            accuracy_weight = performance.get("accuracy", 0.5)
            latency_penalty = min(1.0, performance.get("avg_latency", 1.0) / 1000)

            weights[model] = accuracy_weight * (2 - latency_penalty)

        # Normalize weights
        total_weight = sum(weights.values())
        if total_weight > 0:
            weights = {k: v / total_weight for k, v in weights.items()}
        else:
            # Equal weights if no performance data
            weights = {k: 1.0 / len(models) for k in models}

        return weights

    async def _share_results_via_map(
            self,
            task: Dict[str, Any],
            response: Dict[str, Any]
    ):
        """Share AI response with other agents via MAP"""
        message = Message(
            id=f"{self.agent_id}-{task.get('id')}-response",
            type=MessageType.RESPONSE,
            source=self.agent_id,
            destination=task.get("requester", "broadcast"),
            payload=MessagePayload(
                content_type="application/json",
                data=json.dumps({
                    "task_id": task.get("id"),
                    "response": response,
                    "agent_id": self.agent_id,
                    "timestamp": datetime.utcnow().isoformat()
                })
            ),
            headers={
                "task_type": task.get("type", "ai_query"),
                "models_used": ",".join(response.get("models_used", []))
            }
        )

        await self.map_client.send_message(message)

    async def _update_mall(
            self,
            task: Dict[str, Any],
            model_responses: List[Dict[str, Any]],
            aggregated_response: Dict[str, Any]
    ):
        """Update MALL with interaction data for learning"""
        interaction_data = {
            "agent_id": self.agent_id,
            "task": task,
            "model_responses": model_responses,
            "aggregated_response": aggregated_response,
            "timestamp": datetime.utcnow().isoformat()
        }

        await self.mall_client.record_interaction(interaction_data)

        # Train model selector with new data
        await self.model_selector.update_from_interaction(interaction_data)

    async def _handle_failure(
            self,
            task: Dict[str, Any],
            context: AgentContext,
            error: Exception
    ) -> Dict[str, Any]:
        """Handle failure with fallback mechanisms"""

        # Try fallback models
        fallback_models = self.config.get("fallback_models", [])

        if fallback_models:
            logger.info(f"Attempting fallback models: {fallback_models}")

            try:
                model_responses = await self._query_models(
                    task=task,
                    models=fallback_models,
                    context=context
                )

                # Use simple averaging for fallback
                return await self.aggregator.aggregate(
                    responses=model_responses,
                    weights={m: 1.0 / len(fallback_models) for m in fallback_models}
                )

            except Exception as fallback_error:
                logger.error(f"Fallback also failed: {fallback_error}")

        # Return error response
        return {
            "success": False,
            "error": str(error),
            "task_id": task.get("id"),
            "timestamp": datetime.utcnow().isoformat()
        }

    def _generate_cache_key(self, task: Dict[str, Any]) -> str:
        """Generate cache key for task"""
        # Use task type, prompt hash, and parameters
        import hashlib

        key_parts = [
            task.get("type", "unknown"),
            hashlib.md5(
                task.get("prompt", "").encode()
            ).hexdigest()[:8]
        ]

        # Add sorted parameters
        params = task.get("parameters", {})
        if params:
            param_str = json.dumps(params, sort_keys=True)
            key_parts.append(
                hashlib.md5(param_str.encode()).hexdigest()[:8]
            )

        return ":".join(key_parts)


class AIAgent:
    """High-level AI Agent interface"""

    def __init__(
            self,
            agent_id: str,
            config: Dict[str, Any],
            maple_clients: Dict[str, Any]
    ):
        self.agent_id = agent_id
        self.config = config

        # Initialize core with MAPLE clients
        self.core = AgentCore(
            agent_id=agent_id,
            config=config,
            map_client=maple_clients["map"],
            ars_client=maple_clients["ars"],
            mall_client=maple_clients["mall"],
            mapleverse_client=maple_clients.get("mapleverse")
        )

        # UAL runtime for command processing
        self.ual_runtime = UALRuntime()

        # Background tasks
        self._background_tasks = []
        self._running = False

    async def start(self):
        """Start the AI agent"""
        self._running = True

        # Start monitoring
        monitor_task = asyncio.create_task(
            self._monitor_performance()
        )
        self._background_tasks.append(monitor_task)

        # Start UAL command listener
        command_task = asyncio.create_task(
            self._listen_for_commands()
        )
        self._background_tasks.append(command_task)

        logger.info(f"AI Agent {self.agent_id} started")

    async def stop(self):
        """Stop the AI agent"""
        self._running = False

        # Cancel background tasks
        for task in self._background_tasks:
            task.cancel()

        await asyncio.gather(*self._background_tasks, return_exceptions=True)

        logger.info(f"AI Agent {self.agent_id} stopped")

    async def query(
            self,
            prompt: str,
            parameters: Optional[Dict[str, Any]] = None,
            context: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """Query the AI agent with a prompt"""

        task = {
            "id": f"query-{int(time.time() * 1000)}",
            "type": "text_generation",
            "prompt": prompt,
            "parameters": parameters or {},
            "requester": context.get("requester") if context else None
        }

        agent_context = None
        if context and context.get("conversation_history"):
            agent_context = AgentContext(
                agent_id=self.agent_id,
                task_id=task["id"],
                conversation_history=context["conversation_history"]
            )

        return await self.core.process_task(task, agent_context)

    async def _monitor_performance(self):
        """Background task to monitor and report performance"""

        while self._running:
            try:
                # Get performance metrics
                metrics = await self.core.monitor.get_metrics()

                # Update ARS with current performance
                await self.core.ars_client.update_agent_state(
                    agent_id=self.agent_id,
                    state={
                        "status": "active",
                        "performance_metrics": metrics,
                        "last_updated": datetime.utcnow().isoformat()
                    }
                )

                # Report to MALL for learning
                await self.core.mall_client.report_metrics(
                    agent_id=self.agent_id,
                    metrics=metrics
                )

                # Wait before next update
                await asyncio.sleep(60)  # Update every minute

            except Exception as e:
                logger.error(f"Error in performance monitoring: {e}")
                await asyncio.sleep(60)

    async def _listen_for_commands(self):
        """Listen for UAL commands via MAP"""

        # Subscribe to agent-specific commands
        await self.core.map_client.subscribe(
            topic=f"agent.{self.agent_id}.commands",
            handler=self._handle_ual_command
        )

        while self._running:
            await asyncio.sleep(1)

    async def _handle_ual_command(self, message: Message):
        """Handle incoming UAL commands"""

        try:
            command_data = json.loads(message.payload.data)
            command = command_data.get("command")

            if command == "REQ":
                # Handle insight request
                task_type = command_data.get("type", "insight")
                model = command_data.get("model")
                query = command_data.get("query")

                task = {
                    "id": message.id,
                    "type": task_type,
                    "prompt": query,
                    "parameters": {
                        "preferred_model": model
                    },
                    "requester": message.source
                }

                response = await self.core.process_task(task)

                # Send response back
                await self.core.map_client.send_message(
                    Message(
                        id=f"{message.id}-response",
                        type=MessageType.RESPONSE,
                        source=self.agent_id,
                        destination=message.source,
                        payload=MessagePayload(
                            content_type="application/json",
                            data=json.dumps(response)
                        )
                    )
                )

        except Exception as e:
            logger.error(f"Error handling UAL command: {e}")