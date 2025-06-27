# File: maple/core/map/routing/engine.py
# Description: Core routing engine for the Multi-Agent Protocol that handles
# intelligent message routing, load balancing, and agent selection based on
# capabilities, performance metrics, and availability.

from __future__ import annotations
import asyncio
import logging
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Set, Tuple, Any
from uuid import UUID
import random
import heapq

from maple.core.map.models.message import (
    MAPMessage, MessageDestination, AgentIdentifier,
    MessagePriority, MessageType
)

logger = logging.getLogger(__name__)


@dataclass
class RouteMetrics:
    """Performance metrics for a specific route"""
    success_count: int = 0
    failure_count: int = 0
    total_latency: float = 0.0
    last_success: Optional[datetime] = None
    last_failure: Optional[datetime] = None

    @property
    def success_rate(self) -> float:
        total = self.success_count + self.failure_count
        return self.success_count / total if total > 0 else 0.0

    @property
    def average_latency(self) -> float:
        return self.total_latency / self.success_count if self.success_count > 0 else 0.0

    def record_success(self, latency: float):
        self.success_count += 1
        self.total_latency += latency
        self.last_success = datetime.utcnow()

    def record_failure(self):
        self.failure_count += 1
        self.last_failure = datetime.utcnow()


@dataclass
class AgentRoute:
    """Represents a route to a specific agent"""
    agent: AgentIdentifier
    endpoint: str
    capabilities: Set[str] = field(default_factory=set)
    metrics: RouteMetrics = field(default_factory=RouteMetrics)
    health_score: float = 1.0
    max_concurrent: int = 100
    current_load: int = 0
    last_heartbeat: datetime = field(default_factory=datetime.utcnow)

    @property
    def is_healthy(self) -> bool:
        # Consider agent unhealthy if no heartbeat for 60 seconds
        return (datetime.utcnow() - self.last_heartbeat).seconds < 60

    @property
    def is_overloaded(self) -> bool:
        return self.current_load >= self.max_concurrent

    def calculate_score(self, required_capabilities: List[str] = None) -> float:
        """Calculate routing score based on multiple factors"""
        score = self.health_score

        # Factor in success rate
        score *= (0.5 + 0.5 * self.metrics.success_rate)

        # Factor in current load
        load_factor = 1.0 - (self.current_load / self.max_concurrent)
        score *= (0.7 + 0.3 * load_factor)

        # Factor in capability match
        if required_capabilities:
            matched = sum(1 for cap in required_capabilities if cap in self.capabilities)
            match_ratio = matched / len(required_capabilities) if required_capabilities else 1.0
            score *= match_ratio

        # Penalize if unhealthy
        if not self.is_healthy:
            score *= 0.1

        return score


class RoutingStrategy:
    """Base class for routing strategies"""

    async def select_route(self,
                           routes: List[AgentRoute],
                           message: MAPMessage) -> Optional[AgentRoute]:
        raise NotImplementedError


class LoadBalancedRouting(RoutingStrategy):
    """Load-balanced routing with capability matching"""

    async def select_route(self,
                           routes: List[AgentRoute],
                           message: MAPMessage) -> Optional[AgentRoute]:
        if not routes:
            return None

        # Filter healthy and non-overloaded routes
        available_routes = [r for r in routes if r.is_healthy and not r.is_overloaded]

        if not available_routes:
            # Fallback to any healthy route
            available_routes = [r for r in routes if r.is_healthy]

        if not available_routes:
            logger.warning("No healthy routes available")
            return None

        # Get required capabilities from message
        required_caps = []
        if message.header.destination and message.header.destination.requirements:
            required_caps = message.header.destination.requirements

        # Score and sort routes
        scored_routes = [
            (route.calculate_score(required_caps), route)
            for route in available_routes
        ]
        scored_routes.sort(key=lambda x: x[0], reverse=True)

        # Use weighted random selection from top candidates
        top_n = min(3, len(scored_routes))
        candidates = scored_routes[:top_n]

        if not candidates:
            return None

        # Weighted random selection
        total_score = sum(score for score, _ in candidates)
        if total_score == 0:
            return candidates[0][1]

        rand = random.uniform(0, total_score)
        cumulative = 0

        for score, route in candidates:
            cumulative += score
            if rand <= cumulative:
                return route

        return candidates[-1][1]


class PriorityQueueRouting(RoutingStrategy):
    """Priority-based routing for critical messages"""

    def __init__(self):
        self.priority_queues: Dict[str, List[Tuple[float, AgentRoute]]] = defaultdict(list)

    async def select_route(self,
                           routes: List[AgentRoute],
                           message: MAPMessage) -> Optional[AgentRoute]:
        priority_value = {
            MessagePriority.CRITICAL: 0,
            MessagePriority.HIGH: 1,
            MessagePriority.MEDIUM: 2,
            MessagePriority.LOW: 3,
            MessagePriority.BACKGROUND: 4
        }.get(message.header.priority, 2)

        # Find route with lowest load for high priority messages
        if priority_value <= 1:  # Critical or High
            available_routes = [r for r in routes if r.is_healthy]
            if available_routes:
                return min(available_routes, key=lambda r: r.current_load)

        # Use standard load balancing for other priorities
        return await LoadBalancedRouting().select_route(routes, message)


class RoutingEngine:
    """Core routing engine for MAP protocol"""

    def __init__(self, strategy: RoutingStrategy = None):
        self.strategy = strategy or LoadBalancedRouting()
        self.routes: Dict[str, AgentRoute] = {}  # agent_id -> route
        self.service_routes: Dict[str, List[str]] = defaultdict(list)  # service -> [agent_ids]
        self.capability_index: Dict[str, Set[str]] = defaultdict(set)  # capability -> {agent_ids}
        self.multicast_groups: Dict[str, Set[str]] = defaultdict(set)  # group -> {agent_ids}
        self._route_cache: Dict[str, Tuple[AgentRoute, datetime]] = {}
        self._cache_ttl = timedelta(seconds=60)
        self._lock = asyncio.Lock()

    async def register_agent(self,
                             agent: AgentIdentifier,
                             endpoint: str,
                             capabilities: List[str],
                             max_concurrent: int = 100) -> None:
        """Register an agent with the routing engine"""
        async with self._lock:
            route = AgentRoute(
                agent=agent,
                endpoint=endpoint,
                capabilities=set(capabilities),
                max_concurrent=max_concurrent
            )

            self.routes[agent.agent_id] = route
            self.service_routes[agent.service].append(agent.agent_id)

            # Update capability index
            for cap in capabilities:
                self.capability_index[cap].add(agent.agent_id)

            logger.info(f"Registered agent {agent.agent_id} with {len(capabilities)} capabilities")

    async def unregister_agent(self, agent_id: str) -> None:
        """Remove an agent from the routing engine"""
        async with self._lock:
            if agent_id not in self.routes:
                return

            route = self.routes[agent_id]

            # Remove from service routes
            if route.agent.service in self.service_routes:
                self.service_routes[route.agent.service].remove(agent_id)

            # Remove from capability index
            for cap in route.capabilities:
                self.capability_index[cap].discard(agent_id)

            # Remove from multicast groups
            for group, members in self.multicast_groups.items():
                members.discard(agent_id)

            del self.routes[agent_id]

            # Clear cache entries
            self._clear_cache_for_agent(agent_id)

            logger.info(f"Unregistered agent {agent_id}")

    async def update_agent_health(self, agent_id: str, health_score: float) -> None:
        """Update agent health score"""
        async with self._lock:
            if agent_id in self.routes:
                self.routes[agent_id].health_score = health_score
                self.routes[agent_id].last_heartbeat = datetime.utcnow()

    async def record_route_metrics(self,
                                   agent_id: str,
                                   success: bool,
                                   latency: float = 0.0) -> None:
        """Record routing metrics for an agent"""
        async with self._lock:
            if agent_id in self.routes:
                route = self.routes[agent_id]
                if success:
                    route.metrics.record_success(latency)
                else:
                    route.metrics.record_failure()

    async def route_message(self, message: MAPMessage) -> Optional[AgentRoute]:
        """Route a message to the appropriate agent"""
        destination = message.header.destination

        if not destination:
            logger.error("Message has no destination")
            return None

        # Check cache first
        cache_key = self._get_cache_key(message)
        cached_route = self._get_cached_route(cache_key)
        if cached_route:
            return cached_route

        # Handle different routing scenarios
        if destination.is_broadcast():
            return await self._handle_broadcast(message)

        if destination.is_multicast():
            return await self._handle_multicast(message)

        if destination.agent_id:
            return await self._route_to_specific_agent(destination.agent_id, message)

        if destination.service:
            return await self._route_to_service(destination.service, message)

        if destination.requirements:
            return await self._route_by_capabilities(destination.requirements, message)

        logger.error("Unable to determine routing strategy for message")
        return None

    async def _route_to_specific_agent(self,
                                       agent_id: str,
                                       message: MAPMessage) -> Optional[AgentRoute]:
        """Route to a specific agent"""
        route = self.routes.get(agent_id)

        if route and route.is_healthy:
            self._cache_route(self._get_cache_key(message), route)
            return route

        logger.warning(f"Agent {agent_id} not found or unhealthy")
        return None

    async def _route_to_service(self,
                                service: str,
                                message: MAPMessage) -> Optional[AgentRoute]:
        """Route to any agent providing a service"""
        agent_ids = self.service_routes.get(service, [])

        if not agent_ids:
            logger.warning(f"No agents found for service {service}")
            return None

        routes = [self.routes[aid] for aid in agent_ids if aid in self.routes]
        selected = await self.strategy.select_route(routes, message)

        if selected:
            self._cache_route(self._get_cache_key(message), selected)

        return selected

    async def _route_by_capabilities(self,
                                     requirements: List[str],
                                     message: MAPMessage) -> Optional[AgentRoute]:
        """Route based on required capabilities"""
        # Find agents that have all required capabilities
        if not requirements:
            return None

        # Start with agents having the first capability
        candidate_ids = self.capability_index.get(requirements[0], set()).copy()

        # Intersect with agents having other capabilities
        for cap in requirements[1:]:
            candidate_ids &= self.capability_index.get(cap, set())

        if not candidate_ids:
            logger.warning(f"No agents found with capabilities {requirements}")
            return None

        routes = [self.routes[aid] for aid in candidate_ids if aid in self.routes]
        selected = await self.strategy.select_route(routes, message)

        if selected:
            self._cache_route(self._get_cache_key(message), selected)

        return selected

    async def _handle_broadcast(self, message: MAPMessage) -> Optional[AgentRoute]:
        """Handle broadcast messages (returns None as broadcast is handled differently)"""
        # Broadcast messages are handled by the message broker, not individual routing
        return None

    async def _handle_multicast(self, message: MAPMessage) -> Optional[AgentRoute]:
        """Handle multicast messages (returns None as multicast is handled differently)"""
        # Multicast messages are handled by the message broker, not individual routing
        return None

    async def join_multicast_group(self, agent_id: str, group: str) -> None:
        """Add agent to multicast group"""
        async with self._lock:
            self.multicast_groups[group].add(agent_id)

    async def leave_multicast_group(self, agent_id: str, group: str) -> None:
        """Remove agent from multicast group"""
        async with self._lock:
            self.multicast_groups[group].discard(agent_id)

    def get_multicast_members(self, group: str) -> Set[str]:
        """Get all members of a multicast group"""
        return self.multicast_groups.get(group, set()).copy()

    def _get_cache_key(self, message: MAPMessage) -> str:
        """Generate cache key for routing decision"""
        dest = message.header.destination
        parts = []

        if dest.agent_id:
            parts.append(f"agent:{dest.agent_id}")
        if dest.service:
            parts.append(f"service:{dest.service}")
        if dest.requirements:
            parts.append(f"caps:{','.join(sorted(dest.requirements))}")

        return "|".join(parts)

    def _get_cached_route(self, cache_key: str) -> Optional[AgentRoute]:
        """Get cached route if still valid"""
        if cache_key in self._route_cache:
            route, timestamp = self._route_cache[cache_key]
            if datetime.utcnow() - timestamp < self._cache_ttl:
                if route.is_healthy:
                    return route
            del self._route_cache[cache_key]
        return None

    def _cache_route(self, cache_key: str, route: AgentRoute) -> None:
        """Cache routing decision"""
        self._route_cache[cache_key] = (route, datetime.utcnow())

        # Limit cache size
        if len(self._route_cache) > 10000:
            # Remove oldest entries
            sorted_entries = sorted(self._route_cache.items(),
                                    key=lambda x: x[1][1])
            for key, _ in sorted_entries[:1000]:
                del self._route_cache[key]

    def _clear_cache_for_agent(self, agent_id: str) -> None:
        """Clear cache entries for a specific agent"""
        keys_to_remove = []
        for key, (route, _) in self._route_cache.items():
            if route.agent.agent_id == agent_id:
                keys_to_remove.append(key)

        for key in keys_to_remove:
            del self._route_cache[key]

    async def get_routing_stats(self) -> Dict[str, Any]:
        """Get routing statistics"""
        async with self._lock:
            total_agents = len(self.routes)
            healthy_agents = sum(1 for r in self.routes.values() if r.is_healthy)

            capability_stats = {
                cap: len(agents)
                for cap, agents in self.capability_index.items()
            }

            service_stats = {
                service: len(agents)
                for service, agents in self.service_routes.items()
            }

            return {
                "total_agents": total_agents,
                "healthy_agents": healthy_agents,
                "unhealthy_agents": total_agents - healthy_agents,
                "total_capabilities": len(self.capability_index),
                "total_services": len(self.service_routes),
                "capability_distribution": capability_stats,
                "service_distribution": service_stats,
                "cache_size": len(self._route_cache),
                "multicast_groups": len(self.multicast_groups)
            }