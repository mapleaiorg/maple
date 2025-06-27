# File: maple/mall/server/mall_server.py
# Description: Main MALL server implementation with REST API endpoints.

from __future__ import annotations
from typing import Dict, List, Optional, Any
import asyncio
import logging
from datetime import datetime
from aiohttp import web
import aiohttp_cors

from maple.mall.core.learning_node import LearningNode, NodeConfig
from maple.mall.core.federated import FederatedLearningManager, FederatedConfig
from maple.mall.core.environment import EnvironmentMonitor
from maple.mall.spawn.auto_spawner import AutoSpawner, SpawnConfig
from maple.mall.spawn.templates import TemplateRegistry
from maple.mall.spawn.predictor import SpawnPredictor
from maple.mall.strategies.gan_strategy import StrategyGAN, StrategyConfig

logger = logging.getLogger(__name__)


class MALLServer:
    """
    Main MALL server providing REST API for learning operations.
    Manages learning nodes, federated training, and auto-spawning.
    """

    def __init__(self, config: Dict[str, Any]):
        self.config = config
        self.app = web.Application()

        # Core components
        self.learning_nodes: Dict[str, LearningNode] = {}
        self.federated_manager = FederatedLearningManager(
            FederatedConfig(**config.get("federated", {}))
        )
        self.environment_monitors: Dict[str, EnvironmentMonitor] = {}
        self.auto_spawners: Dict[str, AutoSpawner] = {}
        self.template_registry = TemplateRegistry()
        self.strategy_gan = StrategyGAN(
            StrategyConfig(**config.get("strategy_gan", {}))
        )

        # Setup routes
        self._setup_routes()
        self._setup_cors()

        logger.info("MALL server initialized")

    def _setup_routes(self):
        """Setup API routes"""
        router = self.app.router

        # Health
        router.add_get("/health", self.health_check)

        # Training
        router.add_post("/training/federated", self.start_federated_training)
        router.add_get("/training/federated/{round_id}", self.get_training_status)
        router.add_get("/training/federated/{round_id}/model", self.get_trained_model)
        router.add_get("/training/federated/{round_id}/logs/stream", self.stream_training_logs)
        router.add_post("/training/batch", self.batch_train)

        # Environment
        router.add_get("/environment/{shard_id}/sense", self.sense_environment)
        router.add_get("/environment/{shard_id}/predict", self.predict_environment)
        router.add_get("/environment/{shard_id}/stream", self.stream_environment)

        # Spawning
        router.add_post("/spawn/predict", self.predict_spawn)
        router.add_post("/spawn/agent", self.spawn_agent)
        router.add_delete("/spawn/agent/{agent_id}", self.despawn_agent)
        router.add_post("/spawn/batch", self.batch_spawn)

        # Strategies
        router.add_post("/strategies/generate", self.generate_strategy)
        router.add_post("/strategies/evaluate", self.evaluate_strategy)

        # Models
        router.add_get("/models", self.list_models)
        router.add_get("/models/{model_id}", self.get_model)
        router.add_post("/models", self.upload_model)
        router.add_get("/models/{model_id}/download", self.download_model)

        # Metrics
        router.add_get("/metrics/learning", self.get_learning_metrics)
        router.add_get("/metrics/spawn", self.get_spawn_metrics)

        # Admin
        router.add_post("/admin/nodes", self.add_learning_node)
        router.add_delete("/admin/nodes/{node_id}", self.remove_learning_node)
        router.add_get("/admin/status", self.get_system_status)

    def _setup_cors(self):
        """Setup CORS for browser access"""
        cors = aiohttp_cors.setup(self.app, defaults={
            "*": aiohttp_cors.ResourceOptions(
                allow_credentials=True,
                expose_headers="*",
                allow_headers="*",
                allow_methods="*"
            )
        })

        for route in list(self.app.router.routes()):
            cors.add(route)

    async def start(self, host: str = "0.0.0.0", port: int = 8080):
        """Start the server"""
        # Initialize components
        await self._initialize_components()

        # Start server
        runner = web.AppRunner(self.app)
        await runner.setup()
        site = web.TCPSite(runner, host, port)
        await site.start()

        logger.info(f"MALL server started on {host}:{port}")

        # Keep running
        try:
            await asyncio.Event().wait()
        except KeyboardInterrupt:
            pass
        finally:
            await self._shutdown_components()
            await runner.cleanup()

    async def _initialize_components(self):
        """Initialize all components"""
        # Start federated manager
        await self.federated_manager.start()

        # Create default learning nodes
        for i in range(self.config.get("default_nodes", 3)):
            node_config = NodeConfig(
                node_id=f"node-{i}",
                shard_id="default"
            )
            node = LearningNode(node_config)
            await node.start()
            self.learning_nodes[node.node_id] = node
            await self.federated_manager.register_node(
                node,
                {"general", "compute", "train"}
            )

        # Create default environment monitor
        monitor = EnvironmentMonitor("default")
        await monitor.start()
        self.environment_monitors["default"] = monitor

        # Create default auto-spawner
        spawner = AutoSpawner(
            SpawnConfig(),
            monitor,
            self.template_registry
        )
        await spawner.start()
        self.auto_spawners["default"] = spawner

        logger.info("All components initialized")

    async def _shutdown_components(self):
        """Shutdown all components"""
        # Stop spawners
        for spawner in self.auto_spawners.values():
            await spawner.stop()

        # Stop monitors
        for monitor in self.environment_monitors.values():
            await monitor.stop()

        # Stop nodes
        for node in self.learning_nodes.values():
            await node.stop()

        # Stop federated manager
        await self.federated_manager.stop()

        logger.info("All components shut down")

    # API Handlers

    async def health_check(self, request: web.Request) -> web.Response:
        """Health check endpoint"""
        return web.json_response({
            "status": "healthy",
            "timestamp": datetime.utcnow().isoformat(),
            "nodes": len(self.learning_nodes),
            "monitors": len(self.environment_monitors),
            "spawners": len(self.auto_spawners)
        })

    async def start_federated_training(self, request: web.Request) -> web.Response:
        """Start federated training round"""
        data = await request.json()

        # Extract parameters
        model_id = data["model_id"]
        task_type = data["task_type"]
        config = data.get("config", {})

        # Create model (would deserialize from data)
        from maple.mall.models.agent_model import AgentModel, ModelType
        model = AgentModel(
            model_id=model_id,
            model_type=ModelType.DQN,
            input_size=10,
            output_size=4
        )

        # Start training
        round_id = await self.federated_manager.start_federated_round(
            model_id,
            model,
            task_type,
            config
        )

        return web.json_response({
            "round_id": round_id,
            "status": "started",
            "nodes": len(self.federated_manager.nodes)
        })

    async def get_training_status(self, request: web.Request) -> web.Response:
        """Get training round status"""
        round_id = request.match_info["round_id"]

        # Check active rounds
        if round_id in self.federated_manager.active_rounds:
            round_data = self.federated_manager.active_rounds[round_id]
            status = "active"
        else:
            # Check completed rounds
            completed = [
                r for r in self.federated_manager.completed_rounds
                if r.round_id == round_id
            ]
            if completed:
                round_data = completed[0]
                status = "completed"
            else:
                return web.json_response(
                    {"error": "Round not found"},
                    status=404
                )

        return web.json_response({
            "round_id": round_id,
            "status": status,
            "epoch": round_data.epoch,
            "nodes": len(round_data.selected_nodes),
            "start_time": round_data.start_time.isoformat(),
            "end_time": round_data.end_time.isoformat() if round_data.end_time else None
        })

    async def sense_environment(self, request: web.Request) -> web.Response:
        """Sense environment data"""
        shard_id = request.match_info["shard_id"]

        if shard_id not in self.environment_monitors:
            return web.json_response(
                {"error": "Shard not found"},
                status=404
            )

        monitor = self.environment_monitors[shard_id]
        env_data = await monitor.sense_environment()

        return web.json_response({
            "timestamp": env_data.timestamp.isoformat(),
            "shard_id": env_data.shard_id,
            "task_backlog": env_data.task_backlog,
            "active_agents": env_data.active_agents,
            "resource_utilization": env_data.resource_utilization,
            "performance_metrics": env_data.performance_metrics
        })

    async def spawn_agent(self, request: web.Request) -> web.Response:
        """Spawn a new agent"""
        data = await request.json()

        # Get spawner (default for now)
        spawner = self.auto_spawners.get("default")
        if not spawner:
            return web.json_response(
                {"error": "Spawner not available"},
                status=503
            )

        # Create spawn request
        from maple.mall.spawn.auto_spawner import SpawnRequest
        spawn_request = SpawnRequest(
            agent_id=data["agent_id"],
            template_name=data["template_name"],
            capabilities=data["capabilities"],
            configuration=data["configuration"],
            parent_agent=data.get("parent_agent"),
            priority=data.get("priority", "normal"),
            metadata=data.get("metadata", {})
        )

        # Execute spawn (would integrate with ARS/UAL)
        # For now, just track it
        spawner.active_agents[spawn_request.agent_id] = {
            "template": spawn_request.template_name,
            "spawned_at": datetime.utcnow(),
            "capabilities": spawn_request.capabilities
        }

        return web.json_response({
            "agent_id": spawn_request.agent_id,
            "status": "spawned",
            "ual_command": spawn_request.to_ual_command()
        })

    async def get_system_status(self, request: web.Request) -> web.Response:
        """Get overall system status"""
        return web.json_response({
            "timestamp": datetime.utcnow().isoformat(),
            "learning_nodes": {
                node_id: node.get_metrics()
                for node_id, node in self.learning_nodes.items()
            },
            "federated_manager": self.federated_manager.get_metrics(),
            "environment_monitors": {
                shard_id: monitor.get_metrics()
                for shard_id, monitor in self.environment_monitors.items()
            },
            "auto_spawners": {
                shard_id: spawner.get_metrics()
                for shard_id, spawner in self.auto_spawners.items()
            }
        })

    # Placeholder implementations for remaining endpoints

    async def get_trained_model(self, request: web.Request) -> web.Response:
        """Get trained model"""
        return web.json_response({"status": "not_implemented"})

    async def stream_training_logs(self, request: web.Request) -> web.Response:
        """Stream training logs"""
        return web.json_response({"status": "not_implemented"})

    async def batch_train(self, request: web.Request) -> web.Response:
        """Batch training"""
        return web.json_response({"status": "not_implemented"})

    async def predict_environment(self, request: web.Request) -> web.Response:
        """Predict environment"""
        return web.json_response({"status": "not_implemented"})

    async def stream_environment(self, request: web.Request) -> web.Response:
        """Stream environment updates"""
        return web.json_response({"status": "not_implemented"})

    async def predict_spawn(self, request: web.Request) -> web.Response:
        """Predict spawn need"""
        return web.json_response({"status": "not_implemented"})

    async def despawn_agent(self, request: web.Request) -> web.Response:
        """Despawn agent"""
        return web.json_response({"status": "not_implemented"})

    async def batch_spawn(self, request: web.Request) -> web.Response:
        """Batch spawn agents"""
        return web.json_response({"status": "not_implemented"})

    async def generate_strategy(self, request: web.Request) -> web.Response:
        """Generate strategy"""
        return web.json_response({"status": "not_implemented"})

    async def evaluate_strategy(self, request: web.Request) -> web.Response:
        """Evaluate strategy"""
        return web.json_response({"status": "not_implemented"})

    async def list_models(self, request: web.Request) -> web.Response:
        """List models"""
        return web.json_response({"status": "not_implemented"})

    async def get_model(self, request: web.Request) -> web.Response:
        """Get model info"""
        return web.json_response({"status": "not_implemented"})

    async def upload_model(self, request: web.Request) -> web.Response:
        """Upload model"""
        return web.json_response({"status": "not_implemented"})

    async def download_model(self, request: web.Request) -> web.Response:
        """Download model"""
        return web.json_response({"status": "not_implemented"})

    async def get_learning_metrics(self, request: web.Request) -> web.Response:
        """Get learning metrics"""
        return web.json_response({"status": "not_implemented"})

    async def get_spawn_metrics(self, request: web.Request) -> web.Response:
        """Get spawn metrics"""
        return web.json_response({"status": "not_implemented"})

    async def add_learning_node(self, request: web.Request) -> web.Response:
        """Add learning node"""
        return web.json_response({"status": "not_implemented"})

    async def remove_learning_node(self, request: web.Request) -> web.Response:
        """Remove learning node"""
        return web.json_response({"status": "not_implemented"})


# Main entry point
async def main():
    """Main entry point"""
    import yaml

    # Load configuration
    try:
        with open("config/mall-server.yaml", "r") as f:
            config = yaml.safe_load(f)
    except FileNotFoundError:
        config = {
            "default_nodes": 3,
            "federated": {
                "min_nodes_per_round": 2,
                "aggregation_strategy": "fedavg"
            },
            "strategy_gan": {
                "latent_dim": 100,
                "strategy_dim": 50
            }
        }

    # Create and start server
    server = MALLServer(config)
    await server.start()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())