# File: core/ars/api/server.py
# Description: RESTful API server for the Agent Registry Service.
# Provides HTTP endpoints for agent management and discovery.

from __future__ import annotations
import asyncio
import json
from datetime import datetime
from typing import List, Optional, Dict, Any, Union
import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI, HTTPException, Query, Depends, Request, Response
from fastapi.middleware.cors import CORSMiddleware
from fastapi.middleware.gzip import GZipMiddleware
from fastapi.responses import JSONResponse, StreamingResponse
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from pydantic import BaseModel, Field, validator
import uvicorn

from core.ars.registry import RegistryManager, create_registry_manager
from core.ars.models.registry import (
    AgentRegistration, ServiceQuery, Capability,
    AgentStatus, HealthStatus, RegistryEvent, Endpoint
)
from core.ars.discovery import SearchStrategy
from core.ars.events import EventBus, Event, EventPriority

logger = logging.getLogger(__name__)


# Pydantic models for API

class CapabilityModel(BaseModel):
    """API model for capability"""
    name: str
    version: Optional[str] = "1.0"
    description: Optional[str] = None
    parameters: Optional[Dict[str, Any]] = None

    class Config:
        schema_extra = {
            "example": {
                "name": "image_processing",
                "version": "2.0",
                "description": "Process and analyze images",
                "parameters": {
                    "max_size": "10MB",
                    "formats": ["jpg", "png", "webp"]
                }
            }
        }


class EndpointModel(BaseModel):
    """API model for endpoint"""
    type: str = "http"
    url: str
    protocol: Optional[str] = None
    metadata: Optional[Dict[str, Any]] = None

    class Config:
        schema_extra = {
            "example": {
                "type": "http",
                "url": "http://agent1.example.com:8080",
                "protocol": "REST",
                "metadata": {"region": "us-west"}
            }
        }


class AgentRegistrationRequest(BaseModel):
    """Request model for agent registration"""
    agent_id: Optional[str] = None
    name: str
    version: str
    capabilities: List[CapabilityModel]
    endpoints: List[EndpointModel]
    metadata: Optional[Dict[str, Any]] = Field(default_factory=dict)

    @validator('name')
    def validate_name(cls, v):
        if not v or len(v) < 3:
            raise ValueError('Agent name must be at least 3 characters')
        return v

    @validator('capabilities')
    def validate_capabilities(cls, v):
        if not v:
            raise ValueError('At least one capability is required')
        return v

    class Config:
        schema_extra = {
            "example": {
                "name": "image-processor",
                "version": "1.0.0",
                "capabilities": [
                    {
                        "name": "resize",
                        "version": "1.0",
                        "description": "Resize images"
                    },
                    {
                        "name": "compress",
                        "version": "1.0",
                        "description": "Compress images"
                    }
                ],
                "endpoints": [
                    {
                        "type": "http",
                        "url": "http://localhost:8080"
                    }
                ],
                "metadata": {
                    "environment": "production",
                    "tags": ["image", "processing"]
                }
            }
        }


class AgentDiscoveryRequest(BaseModel):
    """Request model for agent discovery"""
    capabilities: Optional[List[str]] = None
    tags: Optional[List[str]] = None
    status: Optional[AgentStatus] = None
    health_status: Optional[HealthStatus] = None
    metadata_filter: Optional[Dict[str, Any]] = None
    require_all_capabilities: bool = True
    search_strategy: SearchStrategy = SearchStrategy.HYBRID
    sort_by: Optional[str] = None
    limit: Optional[int] = Field(default=50, ge=1, le=1000)
    offset: Optional[int] = Field(default=0, ge=0)

    class Config:
        schema_extra = {
            "example": {
                "capabilities": ["image_processing", "resize"],
                "tags": ["production"],
                "require_all_capabilities": True,
                "search_strategy": "hybrid",
                "limit": 10
            }
        }


class HealthUpdateRequest(BaseModel):
    """Request model for health update"""
    health_status: HealthStatus
    metrics: Optional[Dict[str, Any]] = None

    class Config:
        schema_extra = {
            "example": {
                "health_status": "healthy",
                "metrics": {
                    "cpu_usage": 45.2,
                    "memory_usage": 62.1,
                    "response_time": 0.125
                }
            }
        }


class CapabilityUpdateRequest(BaseModel):
    """Request model for capability update"""
    capabilities: List[CapabilityModel]

    class Config:
        schema_extra = {
            "example": {
                "capabilities": [
                    {
                        "name": "resize",
                        "version": "2.0",
                        "description": "Enhanced resize with AI"
                    }
                ]
            }
        }


class AgentResponse(BaseModel):
    """Response model for agent data"""
    agent_id: str
    name: str
    version: str
    status: str
    health_status: str
    capabilities: List[Dict[str, Any]]
    endpoints: List[Dict[str, Any]]
    metadata: Dict[str, Any]
    created_at: str
    last_heartbeat: str

    @classmethod
    def from_registration(cls, reg: AgentRegistration) -> 'AgentResponse':
        return cls(
            agent_id=reg.agent_id,
            name=reg.name,
            version=reg.version,
            status=reg.status,
            health_status=reg.health_status,
            capabilities=[cap.model_dump() for cap in reg.capabilities],
            endpoints=[ep.model_dump() for ep in reg.endpoints],
            metadata=reg.metadata,
            created_at=reg.created_at.isoformat(),
            last_heartbeat=reg.last_heartbeat.isoformat()
        )


class EventResponse(BaseModel):
    """Response model for events"""
    event_id: str
    event_type: str
    timestamp: str
    agent_id: Optional[str]
    data: Dict[str, Any]

    @classmethod
    def from_event(cls, event: RegistryEvent) -> 'EventResponse':
        return cls(
            event_id=event.event_id,
            event_type=event.event_type,
            timestamp=event.timestamp.isoformat(),
            agent_id=event.agent_id,
            data=event.data
        )


# API Server

class ARSAPIServer:
    """RESTful API server for Agent Registry Service"""

    def __init__(
            self,
            registry: Optional[RegistryManager] = None,
            config: Optional[Dict[str, Any]] = None
    ):
        self.config = config or {}
        self.registry = registry
        self.app = FastAPI(
            title="MAPLE Agent Registry Service API",
            version="1.0.0",
            description="API for managing and discovering MAPLE agents",
            docs_url="/docs",
            redoc_url="/redoc"
        )

        # Security
        self.security = HTTPBearer() if self.config.get('enable_auth', False) else None

        # Configure middleware
        self._configure_middleware()

        # Configure routes
        self._configure_routes()

        # WebSocket connections for real-time events
        self._websocket_connections: List[Any] = []

    def _configure_middleware(self):
        """Configure API middleware"""
        # CORS
        if self.config.get('enable_cors', True):
            self.app.add_middleware(
                CORSMiddleware,
                allow_origins=self.config.get('cors_origins', ["*"]),
                allow_credentials=True,
                allow_methods=["*"],
                allow_headers=["*"],
            )

        # Compression
        self.app.add_middleware(GZipMiddleware, minimum_size=1000)

        # Request ID middleware
        @self.app.middleware("http")
        async def add_request_id(request: Request, call_next):
            import uuid
            request_id = str(uuid.uuid4())
            request.state.request_id = request_id
            response = await call_next(request)
            response.headers["X-Request-ID"] = request_id
            return response

        # Logging middleware
        @self.app.middleware("http")
        async def log_requests(request: Request, call_next):
            start_time = datetime.utcnow()
            response = await call_next(request)
            duration = (datetime.utcnow() - start_time).total_seconds()

            logger.info(
                f"{request.method} {request.url.path} "
                f"status={response.status_code} "
                f"duration={duration:.3f}s"
            )

            return response

    def _configure_routes(self):
        """Configure API routes"""

        # Health check
        @self.app.get("/health", tags=["System"])
        async def health_check():
            """Health check endpoint"""
            return {
                "status": "healthy",
                "timestamp": datetime.utcnow().isoformat(),
                "service": "agent-registry-service"
            }

        # Agent registration
        @self.app.post(
            "/v1/agents",
            response_model=AgentResponse,
            tags=["Agents"],
            status_code=201
        )
        async def register_agent(
                request: AgentRegistrationRequest,
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Register a new agent"""
            try:
                # Convert to internal models
                capabilities = [
                    Capability(
                        name=cap.name,
                        version=cap.version,
                        description=cap.description,
                        parameters=cap.parameters
                    )
                    for cap in request.capabilities
                ]

                endpoints = [
                    Endpoint(
                        type=ep.type,
                        url=ep.url,
                        protocol=ep.protocol,
                        metadata=ep.metadata
                    )
                    for ep in request.endpoints
                ]

                # Register agent
                registration = await self.registry.register_agent(
                    name=request.name,
                    version=request.version,
                    capabilities=capabilities,
                    endpoints=endpoints,
                    metadata=request.metadata,
                    agent_id=request.agent_id
                )

                return AgentResponse.from_registration(registration)

            except ValueError as e:
                raise HTTPException(status_code=400, detail=str(e))
            except Exception as e:
                logger.error(f"Failed to register agent: {e}")
                raise HTTPException(status_code=500, detail="Internal server error")

        # Get agent
        @self.app.get(
            "/v1/agents/{agent_id}",
            response_model=AgentResponse,
            tags=["Agents"]
        )
        async def get_agent(
                agent_id: str,
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Get agent by ID"""
            agent = await self.registry.get_agent(agent_id)
            if not agent:
                raise HTTPException(status_code=404, detail="Agent not found")

            return AgentResponse.from_registration(agent)

        # Deregister agent
        @self.app.delete(
            "/v1/agents/{agent_id}",
            tags=["Agents"],
            status_code=204
        )
        async def deregister_agent(
                agent_id: str,
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Deregister an agent"""
            success = await self.registry.deregister_agent(agent_id)
            if not success:
                raise HTTPException(status_code=404, detail="Agent not found")

            return Response(status_code=204)

        # Discover agents
        @self.app.post(
            "/v1/agents/discover",
            response_model=List[AgentResponse],
            tags=["Discovery"]
        )
        async def discover_agents(
                request: AgentDiscoveryRequest,
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Discover agents matching criteria"""
            try:
                agents = await self.registry.discover_agents(
                    capabilities=request.capabilities,
                    tags=request.tags,
                    status=request.status,
                    health_status=request.health_status,
                    metadata_filter=request.metadata_filter,
                    require_all_capabilities=request.require_all_capabilities,
                    sort_by=request.sort_by,
                    limit=request.limit,
                    offset=request.offset
                )

                return [AgentResponse.from_registration(agent) for agent in agents]

            except Exception as e:
                logger.error(f"Failed to discover agents: {e}")
                raise HTTPException(status_code=500, detail="Internal server error")

        # Update agent health
        @self.app.put(
            "/v1/agents/{agent_id}/health",
            tags=["Health"],
            status_code=204
        )
        async def update_health(
                agent_id: str,
                request: HealthUpdateRequest,
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Update agent health status"""
            success = await self.registry.update_agent_health(
                agent_id,
                request.health_status,
                request.metrics
            )

            if not success:
                raise HTTPException(status_code=404, detail="Agent not found")

            return Response(status_code=204)

        # Heartbeat
        @self.app.post(
            "/v1/agents/{agent_id}/heartbeat",
            tags=["Health"],
            status_code=204
        )
        async def heartbeat(
                agent_id: str,
                metrics: Optional[Dict[str, Any]] = None,
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Send agent heartbeat"""
            success = await self.registry.heartbeat(agent_id, metrics)

            if not success:
                raise HTTPException(status_code=404, detail="Agent not found")

            return Response(status_code=204)

        # Update capabilities
        @self.app.put(
            "/v1/agents/{agent_id}/capabilities",
            tags=["Agents"],
            status_code=204
        )
        async def update_capabilities(
                agent_id: str,
                request: CapabilityUpdateRequest,
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Update agent capabilities"""
            capabilities = [
                Capability(
                    name=cap.name,
                    version=cap.version,
                    description=cap.description,
                    parameters=cap.parameters
                )
                for cap in request.capabilities
            ]

            success = await self.registry.update_agent_capabilities(
                agent_id,
                capabilities
            )

            if not success:
                raise HTTPException(status_code=404, detail="Agent not found")

            return Response(status_code=204)

        # Get events
        @self.app.get(
            "/v1/events",
            response_model=List[EventResponse],
            tags=["Events"]
        )
        async def get_events(
                agent_id: Optional[str] = Query(None, description="Filter by agent ID"),
                event_type: Optional[str] = Query(None, description="Filter by event type"),
                since: Optional[datetime] = Query(None, description="Filter events since timestamp"),
                limit: int = Query(100, ge=1, le=1000, description="Maximum events to return"),
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Get registry events"""
            events = await self.registry.get_agent_events(
                agent_id=agent_id,
                event_type=event_type,
                since=since,
                limit=limit
            )

            return [EventResponse.from_event(event) for event in events]

        # Get statistics
        @self.app.get(
            "/v1/statistics",
            tags=["System"]
        )
        async def get_statistics(
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Get registry statistics"""
            return await self.registry.get_statistics()

        # WebSocket endpoint for real-time events
        from fastapi import WebSocket, WebSocketDisconnect

        @self.app.websocket("/v1/events/stream")
        async def websocket_events(websocket: WebSocket):
            """Stream events via WebSocket"""
            await websocket.accept()
            self._websocket_connections.append(websocket)

            try:
                # Subscribe to events
                def event_handler(event: Event):
                    asyncio.create_task(
                        websocket.send_json(event.to_dict())
                    )

                self.registry.subscribe("*", event_handler)

                # Keep connection alive
                while True:
                    await websocket.receive_text()

            except WebSocketDisconnect:
                self._websocket_connections.remove(websocket)
            except Exception as e:
                logger.error(f"WebSocket error: {e}")
                if websocket in self._websocket_connections:
                    self._websocket_connections.remove(websocket)

        # Batch operations
        @self.app.post(
            "/v1/agents/batch/discover",
            response_model=Dict[str, List[AgentResponse]],
            tags=["Batch"]
        )
        async def batch_discover(
                requests: List[AgentDiscoveryRequest],
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Batch discovery of agents"""
            results = {}

            for idx, request in enumerate(requests):
                try:
                    agents = await self.registry.discover_agents(
                        capabilities=request.capabilities,
                        tags=request.tags,
                        status=request.status,
                        health_status=request.health_status,
                        metadata_filter=request.metadata_filter,
                        require_all_capabilities=request.require_all_capabilities,
                        sort_by=request.sort_by,
                        limit=request.limit,
                        offset=request.offset
                    )

                    results[f"request_{idx}"] = [
                        AgentResponse.from_registration(agent)
                        for agent in agents
                    ]

                except Exception as e:
                    logger.error(f"Batch discovery error for request {idx}: {e}")
                    results[f"request_{idx}"] = []

            return results

        # Admin endpoints
        @self.app.post(
            "/v1/admin/cleanup",
            tags=["Admin"]
        )
        async def cleanup_expired(
                ttl_hours: int = Query(24, description="TTL in hours"),
                auth: Optional[HTTPAuthorizationCredentials] = Depends(self._get_auth)
        ):
            """Clean up expired agents"""
            from datetime import timedelta

            count = await self.registry._storage.clean_expired(
                timedelta(hours=ttl_hours)
            )

            return {
                "cleaned": count,
                "timestamp": datetime.utcnow().isoformat()
            }

    def _get_auth(self):
        """Get authentication dependency"""
        if self.security and self.config.get('enable_auth', False):
            return self.security
        return None

    async def start(self, host: str = "0.0.0.0", port: int = 8080):
        """Start the API server"""
        # Initialize registry if not provided
        if not self.registry:
            self.registry = create_registry_manager(
                backend=self.config.get('storage_backend', 'memory'),
                **self.config.get('registry_config', {})
            )
            await self.registry.start()

        # Configure server
        config = uvicorn.Config(
            self.app,
            host=host,
            port=port,
            log_level=self.config.get('log_level', 'info'),
            access_log=self.config.get('access_log', True),
            use_colors=True
        )

        server = uvicorn.Server(config)
        await server.serve()

    async def stop(self):
        """Stop the API server"""
        if self.registry:
            await self.registry.stop()


# Lifespan management for FastAPI

@asynccontextmanager
async def lifespan(app: FastAPI):
    """Manage application lifespan"""
    # Startup
    registry = create_registry_manager()
    await registry.start()
    app.state.registry = registry

    yield

    # Shutdown
    await registry.stop()


# Create default app instance

def create_app(config: Optional[Dict[str, Any]] = None) -> FastAPI:
    """Create FastAPI application"""
    server = ARSAPIServer(config=config)
    return server.app


# CLI entry point

def main():
    """Main entry point for running the server"""
    import argparse

    parser = argparse.ArgumentParser(description="MAPLE Agent Registry Service API")
    parser.add_argument("--host", default="0.0.0.0", help="Host to bind to")
    parser.add_argument("--port", type=int, default=8080, help="Port to bind to")
    parser.add_argument("--backend", default="memory", help="Storage backend")
    parser.add_argument("--log-level", default="info", help="Log level")

    args = parser.parse_args()

    # Configure logging
    logging.basicConfig(
        level=getattr(logging, args.log_level.upper()),
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    )

    # Create and run server
    config = {
        'storage_backend': args.backend,
        'log_level': args.log_level
    }

    server = ARSAPIServer(config=config)

    # Run server
    asyncio.run(server.start(host=args.host, port=args.port))


if __name__ == "__main__":
    main()

# Export public API
__all__ = [
    "ARSAPIServer",
    "create_app",
    "AgentRegistrationRequest",
    "AgentDiscoveryRequest",
    "HealthUpdateRequest",
    "CapabilityUpdateRequest",
    "AgentResponse",
    "EventResponse"
]