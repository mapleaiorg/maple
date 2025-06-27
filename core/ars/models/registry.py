# File: core/ars/models/registry.py
# Description: Core data models for the Agent Registry Service including
# agent registration, capability definitions, and service metadata.

from __future__ import annotations
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum
from typing import Dict, List, Optional, Set, Any, Tuple
from uuid import UUID, uuid4
import json
import hashlib


class AgentStatus(Enum):
    """Agent availability status"""
    ONLINE = "online"
    OFFLINE = "offline"
    BUSY = "busy"
    MAINTENANCE = "maintenance"
    DEGRADED = "degraded"
    UNKNOWN = "unknown"


class HealthStatus(Enum):
    """Agent health status"""
    HEALTHY = "healthy"
    UNHEALTHY = "unhealthy"
    DEGRADED = "degraded"
    CRITICAL = "critical"


class CapabilityType(Enum):
    """Types of agent capabilities"""
    COMPUTE = "compute"
    STORAGE = "storage"
    ANALYSIS = "analysis"
    GENERATION = "generation"
    COMMUNICATION = "communication"
    COORDINATION = "coordination"
    CUSTOM = "custom"


class DeploymentMode(Enum):
    """Agent deployment modes"""
    CLOUD = "cloud"
    EDGE = "edge"
    HYBRID = "hybrid"
    LOCAL = "local"


@dataclass
class Version:
    """Semantic version representation"""
    major: int
    minor: int
    patch: int
    prerelease: Optional[str] = None

    def __str__(self) -> str:
        version = f"{self.major}.{self.minor}.{self.patch}"
        if self.prerelease:
            version += f"-{self.prerelease}"
        return version

    @classmethod
    def from_string(cls, version_str: str) -> 'Version':
        """Parse version from string"""
        parts = version_str.split("-")
        version_parts = parts[0].split(".")

        return cls(
            major=int(version_parts[0]),
            minor=int(version_parts[1]) if len(version_parts) > 1 else 0,
            patch=int(version_parts[2]) if len(version_parts) > 2 else 0,
            prerelease=parts[1] if len(parts) > 1 else None
        )

    def is_compatible_with(self, other: 'Version') -> bool:
        """Check if version is compatible with another version"""
        # Same major version = compatible
        return self.major == other.major


@dataclass
class CapabilitySchema:
    """Schema definition for a capability"""
    input_schema: Dict[str, Any]
    output_schema: Dict[str, Any]
    error_schema: Optional[Dict[str, Any]] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "input": self.input_schema,
            "output": self.output_schema,
            "errors": self.error_schema
        }


@dataclass
class PerformanceMetrics:
    """Performance metrics for capabilities"""
    avg_response_time_ms: float = 0.0
    p95_response_time_ms: float = 0.0
    p99_response_time_ms: float = 0.0
    success_rate: float = 1.0
    error_rate: float = 0.0
    throughput_per_second: float = 0.0
    last_updated: datetime = field(default_factory=datetime.utcnow)

    def update(self, response_time: float, success: bool):
        """Update metrics with new data point"""
        # Simplified update - in production use proper statistics
        self.avg_response_time_ms = (
                self.avg_response_time_ms * 0.9 + response_time * 0.1
        )
        if success:
            self.success_rate = self.success_rate * 0.99 + 0.01
            self.error_rate = self.error_rate * 0.99
        else:
            self.success_rate = self.success_rate * 0.99
            self.error_rate = self.error_rate * 0.99 + 0.01

        self.last_updated = datetime.utcnow()


@dataclass
class ResourceRequirements:
    """Resource requirements for agent/capability"""
    cpu_cores: Optional[float] = None
    memory_mb: Optional[int] = None
    gpu_required: bool = False
    gpu_memory_mb: Optional[int] = None
    disk_space_mb: Optional[int] = None
    network_bandwidth_mbps: Optional[float] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            k: v for k, v in {
                "cpu_cores": self.cpu_cores,
                "memory_mb": self.memory_mb,
                "gpu_required": self.gpu_required,
                "gpu_memory_mb": self.gpu_memory_mb,
                "disk_space_mb": self.disk_space_mb,
                "network_bandwidth_mbps": self.network_bandwidth_mbps
            }.items() if v is not None
        }


@dataclass
class Capability:
    """Agent capability definition"""
    name: str
    version: Version
    type: CapabilityType
    description: str
    schema: CapabilitySchema
    tags: Set[str] = field(default_factory=set)
    requirements: ResourceRequirements = field(default_factory=ResourceRequirements)
    performance: PerformanceMetrics = field(default_factory=PerformanceMetrics)
    cost_per_call: Optional[float] = None
    rate_limit: Optional[int] = None  # calls per minute
    timeout_seconds: int = 60
    is_async: bool = True
    metadata: Dict[str, Any] = field(default_factory=dict)

    @property
    def capability_id(self) -> str:
        """Generate unique capability ID"""
        return f"{self.name}:{self.version}"

    def matches_requirements(self, required_tags: Set[str]) -> bool:
        """Check if capability matches required tags"""
        return required_tags.issubset(self.tags)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "version": str(self.version),
            "type": self.type.value,
            "description": self.description,
            "schema": self.schema.to_dict(),
            "tags": list(self.tags),
            "requirements": self.requirements.to_dict(),
            "performance": {
                "avg_response_time_ms": self.performance.avg_response_time_ms,
                "success_rate": self.performance.success_rate,
                "error_rate": self.performance.error_rate,
                "throughput_per_second": self.performance.throughput_per_second
            },
            "cost_per_call": self.cost_per_call,
            "rate_limit": self.rate_limit,
            "timeout_seconds": self.timeout_seconds,
            "is_async": self.is_async,
            "metadata": self.metadata
        }


@dataclass
class AgentEndpoint:
    """Agent connection endpoint"""
    protocol: str  # http, grpc, websocket
    host: str
    port: int
    path: Optional[str] = None
    tls_enabled: bool = True
    auth_required: bool = True

    @property
    def url(self) -> str:
        """Get full endpoint URL"""
        scheme = f"{self.protocol}s" if self.tls_enabled else self.protocol
        base_url = f"{scheme}://{self.host}:{self.port}"
        if self.path:
            base_url += f"/{self.path.lstrip('/')}"
        return base_url


@dataclass
class AgentLocation:
    """Physical/logical location of agent"""
    region: str
    zone: Optional[str] = None
    datacenter: Optional[str] = None
    latitude: Optional[float] = None
    longitude: Optional[float] = None

    def distance_to(self, other: 'AgentLocation') -> Optional[float]:
        """Calculate distance to another location (km)"""
        if self.latitude and self.longitude and other.latitude and other.longitude:
            # Simplified haversine formula
            import math
            R = 6371  # Earth radius in km

            lat1, lon1 = math.radians(self.latitude), math.radians(self.longitude)
            lat2, lon2 = math.radians(other.latitude), math.radians(other.longitude)

            dlat = lat2 - lat1
            dlon = lon2 - lon1

            a = math.sin(dlat / 2) ** 2 + math.cos(lat1) * math.cos(lat2) * math.sin(dlon / 2) ** 2
            c = 2 * math.asin(math.sqrt(a))

            return R * c
        return None


@dataclass
class AgentRegistration:
    """Complete agent registration information"""
    agent_id: str
    name: str
    version: Version
    description: str
    capabilities: List[Capability]
    endpoints: List[AgentEndpoint]
    location: AgentLocation
    deployment_mode: DeploymentMode
    status: AgentStatus = AgentStatus.ONLINE
    health: HealthStatus = HealthStatus.HEALTHY
    owner: Optional[str] = None
    organization: Optional[str] = None
    tags: Set[str] = field(default_factory=set)
    metadata: Dict[str, Any] = field(default_factory=dict)
    registered_at: datetime = field(default_factory=datetime.utcnow)
    last_heartbeat: datetime = field(default_factory=datetime.utcnow)
    last_updated: datetime = field(default_factory=datetime.utcnow)

    @property
    def is_available(self) -> bool:
        """Check if agent is available for requests"""
        return (
                self.status == AgentStatus.ONLINE and
                self.health in [HealthStatus.HEALTHY, HealthStatus.DEGRADED]
        )

    @property
    def heartbeat_age(self) -> timedelta:
        """Get time since last heartbeat"""
        return datetime.utcnow() - self.last_heartbeat

    def get_capability(self, name: str, version: Optional[str] = None) -> Optional[Capability]:
        """Get specific capability by name and optional version"""
        for cap in self.capabilities:
            if cap.name == name:
                if version is None or str(cap.version) == version:
                    return cap
        return None

    def has_capability(self, name: str, min_version: Optional[str] = None) -> bool:
        """Check if agent has a capability"""
        cap = self.get_capability(name)
        if not cap:
            return False

        if min_version:
            min_ver = Version.from_string(min_version)
            return cap.version.major >= min_ver.major

        return True

    def to_dict(self) -> Dict[str, Any]:
        return {
            "agent_id": self.agent_id,
            "name": self.name,
            "version": str(self.version),
            "description": self.description,
            "capabilities": [cap.to_dict() for cap in self.capabilities],
            "endpoints": [
                {
                    "protocol": ep.protocol,
                    "host": ep.host,
                    "port": ep.port,
                    "path": ep.path,
                    "tls_enabled": ep.tls_enabled
                }
                for ep in self.endpoints
            ],
            "location": {
                "region": self.location.region,
                "zone": self.location.zone,
                "datacenter": self.location.datacenter
            },
            "deployment_mode": self.deployment_mode.value,
            "status": self.status.value,
            "health": self.health.value,
            "owner": self.owner,
            "organization": self.organization,
            "tags": list(self.tags),
            "metadata": self.metadata,
            "registered_at": self.registered_at.isoformat(),
            "last_heartbeat": self.last_heartbeat.isoformat(),
            "last_updated": self.last_updated.isoformat()
        }


@dataclass
class ServiceQuery:
    """Query for finding agents/capabilities"""
    capability_name: Optional[str] = None
    capability_type: Optional[CapabilityType] = None
    required_tags: Set[str] = field(default_factory=set)
    preferred_tags: Set[str] = field(default_factory=set)
    min_version: Optional[str] = None
    max_response_time_ms: Optional[float] = None
    min_success_rate: Optional[float] = None
    location_preference: Optional[AgentLocation] = None
    max_distance_km: Optional[float] = None
    deployment_mode: Optional[DeploymentMode] = None
    exclude_agents: Set[str] = field(default_factory=set)

    def matches_agent(self, agent: AgentRegistration) -> bool:
        """Check if agent matches query criteria"""
        if not agent.is_available:
            return False

        if agent.agent_id in self.exclude_agents:
            return False

        if self.deployment_mode and agent.deployment_mode != self.deployment_mode:
            return False

        # Check location constraints
        if self.location_preference and self.max_distance_km:
            distance = agent.location.distance_to(self.location_preference)
            if distance and distance > self.max_distance_km:
                return False

        # Check capability requirements
        if self.capability_name:
            matching_cap = None
            for cap in agent.capabilities:
                if cap.name == self.capability_name:
                    matching_cap = cap
                    break

            if not matching_cap:
                return False

            # Check capability-specific requirements
            if self.capability_type and matching_cap.type != self.capability_type:
                return False

            if self.min_version:
                min_ver = Version.from_string(self.min_version)
                if not matching_cap.version.is_compatible_with(min_ver):
                    return False

            if not matching_cap.matches_requirements(self.required_tags):
                return False

            if self.max_response_time_ms and \
                    matching_cap.performance.avg_response_time_ms > self.max_response_time_ms:
                return False

            if self.min_success_rate and \
                    matching_cap.performance.success_rate < self.min_success_rate:
                return False

        return True

    def score_agent(self, agent: AgentRegistration) -> float:
        """Score agent based on query preferences"""
        if not self.matches_agent(agent):
            return 0.0

        score = 1.0

        # Boost for preferred tags
        if self.preferred_tags:
            matched_tags = len(agent.tags.intersection(self.preferred_tags))
            score += matched_tags * 0.1

        # Boost for performance
        if self.capability_name:
            cap = agent.get_capability(self.capability_name)
            if cap:
                score += cap.performance.success_rate * 0.5

                # Lower response time is better
                if cap.performance.avg_response_time_ms > 0:
                    score += (1.0 / cap.performance.avg_response_time_ms) * 100

        # Boost for proximity
        if self.location_preference:
            distance = agent.location.distance_to(self.location_preference)
            if distance:
                # Closer is better
                score += (1.0 / (1.0 + distance)) * 0.3

        # Boost for health
        if agent.health == HealthStatus.HEALTHY:
            score += 0.2

        return score


@dataclass
class RegistryEvent:
    """Event in the registry system"""
    event_id: str = field(default_factory=lambda: str(uuid4()))
    event_type: str = ""
    timestamp: datetime = field(default_factory=datetime.utcnow)
    agent_id: Optional[str] = None
    capability_name: Optional[str] = None
    data: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "event_id": self.event_id,
            "event_type": self.event_type,
            "timestamp": self.timestamp.isoformat(),
            "agent_id": self.agent_id,
            "capability_name": self.capability_name,
            "data": self.data
        }