# File: maple/ai_agent/server.py
# Description: Main server implementation for AI Agent Service.
# Provides HTTP/gRPC endpoints for agent interaction and management.

import asyncio
from typing import Dict, Any, Optional
from fastapi import FastAPI, HTTPException, BackgroundTasks
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn
import logging

from .core.agent import AIAgent
from .config import AIAgentConfig
from .monitoring import AgentMonitor
from core.map.client import MAPClient
from core.ars.client import ARSClient
from mall.client import MALLClient
from mapleverse.client import MapleverseClient

logger = logging.getLogger(__name__)

# FastAPI app
app = FastAPI(
    title="MAPLE AI Agent Service",
    description="AI Agent Service for MAPLE Framework",
    version="0.1.0"
)

# CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Global agent instance
agent: Optional[AIAgent] = None
config: Optional[AIAgentConfig] = None
monitor: Optional[AgentMonitor] = None


# Request/Response models
class QueryRequest(BaseModel):
    prompt: str
    parameters: Optional[Dict[str, Any]] = None
    context: Optional[Dict[str, Any]] = None
    preferred_models: Optional[List[str]] = None


class QueryResponse(BaseModel):
    success: bool
    text: str
    models_used: List[str]
    metadata: Dict[str, Any]
    error: Optional[str] = None


class ConfigUpdateRequest(BaseModel):
    config: Dict[str, Any]
    restart: bool = False


# API Endpoints
@app.on_event("startup")
async def startup_event():
    """Initialize AI Agent on startup"""
    global agent, config, monitor

    try:
        # Load configuration
        config = AIAgentConfig.from_env()

        # Validate config
        errors = config.validate()
        if errors:
            logger.error(f"Configuration errors: {errors}")
            raise ValueError(f"Invalid configuration: {', '.join(errors)}")

        # Initialize MAPLE clients
        maple_clients = {
            "map": MAPClient(config.map_endpoint),
            "ars": ARSClient(config.ars_endpoint),
            "mall": MALLClient(config.mall_endpoint)
        }

        if config.mapleverse_endpoint:
            maple_clients["mapleverse"] = MapleverseClient(
                config.mapleverse_endpoint
            )

        # Initialize agent
        agent = AIAgent(
            agent_id=config.agent_id,
            config=config.to_dict(),
            maple_clients=maple_clients
        )

        # Start agent
        await agent.start()

        # Initialize monitor
        monitor = agent.core.monitor

        logger.info(f"AI Agent {config.agent_id} initialized successfully")

    except Exception as e:
        logger.error(f"Failed to initialize AI Agent: {e}")
        raise


@app.on_event("shutdown")
async def shutdown_event():
    """Cleanup on shutdown"""
    global agent

    if agent:
        await agent.stop()
        logger.info("AI Agent shut down successfully")


@app.get("/health")
async def health_check():
    """Health check endpoint"""

    if not agent:
        raise HTTPException(status_code=503, detail="Agent not initialized")

    # Check adapter health
    adapter_health = {}
    for name, adapter in agent.core.adapter_registry.adapters.items():
        adapter_health[name] = await adapter.health_check()

    return {
        "status": "healthy" if all(adapter_health.values()) else "degraded",
        "agent_id": config.agent_id,
        "adapters": adapter_health
    }


@app.post("/query", response_model=QueryResponse)
async def query_agent(request: QueryRequest):
    """Query the AI agent"""

    if not agent:
        raise HTTPException(status_code=503, detail="Agent not initialized")

    try:
        # Override model selection if preferred models specified
        context = request.context or {}
        if request.preferred_models:
            context["preferred_models"] = request.preferred_models

        # Query agent
        result = await agent.query(
            prompt=request.prompt,
            parameters=request.parameters,
            context=context
        )

        return QueryResponse(
            success=result.get("success", True),
            text=result.get("text", ""),
            models_used=result.get("models_used", []),
            metadata={
                "aggregation_method": result.get("aggregation_method"),
                "latency": result.get("avg_latency"),
                "tokens": result.get("total_tokens"),
                "from_cache": result.get("_from_cache", False)
            }
        )

    except Exception as e:
        logger.error(f"Query error: {e}")
        return QueryResponse(
            success=False,
            text="",
            models_used=[],
            metadata={},
            error=str(e)
        )


@app.get("/metrics")
async def get_metrics():
    """Get agent performance metrics"""

    if not monitor:
        raise HTTPException(status_code=503, detail="Monitor not initialized")

    return await monitor.get_summary()


@app.get("/models")
async def list_models():
    """List available models"""

    if not agent:
        raise HTTPException(status_code=503, detail="Agent not initialized")

    models = []
    for name, adapter in agent.core.adapter_registry.adapters.items():
        model_info = adapter.get_model_info()
        models.append({
            "name": name,
            **model_info
        })

    return {"models": models}


@app.post("/config/update")
async def update_config(
        request: ConfigUpdateRequest,
        background_tasks: BackgroundTasks
):
    """Update agent configuration"""

    global config

    try:
        # Create new config
        new_config = AIAgentConfig(**request.config)

        # Validate
        errors = new_config.validate()
        if errors:
            raise HTTPException(
                status_code=400,
                detail=f"Invalid configuration: {', '.join(errors)}"
            )

        # Update config
        config = new_config

        # Restart agent if requested
        if request.restart:
            background_tasks.add_task(restart_agent)

        return {"success": True, "message": "Configuration updated"}

    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


@app.get("/config")
async def get_config():
    """Get current configuration"""

    if not config:
        raise HTTPException(status_code=503, detail="Config not initialized")

    return config.to_dict()


@app.post("/cache/clear")
async def clear_cache():
    """Clear response cache"""

    if not agent:
        raise HTTPException(status_code=503, detail="Agent not initialized")

    await agent.core.cache.backend.clear()

    return {"success": True, "message": "Cache cleared"}


@app.get("/cache/stats")
async def get_cache_stats():
    """Get cache statistics"""

    if not agent:
        raise HTTPException(status_code=503, detail="Agent not initialized")

    return agent.core.cache.get_stats()


async def restart_agent():
    """Restart the agent with new configuration"""

    global agent

    logger.info("Restarting AI Agent...")

    # Stop current agent
    if agent:
        await agent.stop()

    # Start with new config
    await startup_event()


def create_app(config_path: Optional[str] = None) -> FastAPI:
    """Create FastAPI app with configuration"""

    if config_path:
        global config
        config = AIAgentConfig.from_file(config_path)

    return app


def run_server(
        host: str = "0.0.0.0",
        port: int = 8003,
        config_path: Optional[str] = None
):
    """Run the AI Agent server"""

    # Set up logging
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    )

    # Create app
    app = create_app(config_path)

    # Run server
    uvicorn.run(
        app,
        host=host,
        port=port,
        log_level="info"
    )


if __name__ == "__main__":
    run_server()