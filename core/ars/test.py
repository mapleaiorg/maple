version = agent.version,
status = self._proto_to_status(agent.status),
health_status = self._proto_to_health(agent.health_status),
capabilities = [
    Capability(
        name=cap.name,
        version=cap.version,
        description=cap.description,
        parameters=self._struct_to_dict(cap.parameters)
    )
    for cap in agent.capabilities
],
endpoints = [
    Endpoint(
        type=ep.type,
        url=ep.url,
        protocol=ep.protocol,
        metadata=self._struct_to_dict(ep.metadata)
    )
    for ep in agent.endpoints
],
metadata = self._struct_to_dict(agent.metadata),
metrics = self._struct_to_dict(agent.metrics),
created_at = agent.created_at.ToDatetime(),
last_heartbeat = agent.last_heartbeat.ToDatetime()
)

def _proto_to_event(self, event) -> RegistryEvent:
    """Convert protobuf Event to RegistryEvent"""
    return RegistryEvent(
        event_id=event.event_id,
        event_type=event.event_type,
        timestamp=event.timestamp.ToDatetime(),
        agent_id=event.agent_id if event.agent_id else None,
        data=self._struct_to_dict(event.data)
    )


def _status_to_proto(self, status: AgentStatus):
    """Convert AgentStatus to protobuf"""
    from maple.core.ars.grpc import ars_pb2

    mapping = {
        AgentStatus.ACTIVE: ars_pb2.AGENT_STATUS_ACTIVE,
        AgentStatus.INACTIVE: ars_pb2.AGENT_STATUS_INACTIVE,
        AgentStatus.MAINTENANCE: ars_pb2.AGENT_STATUS_MAINTENANCE,
        AgentStatus.DEPRECATED: ars_pb2.AGENT_STATUS_DEPRECATED
    }
    return mapping.get(status, ars_pb2.AGENT_STATUS_UNKNOWN)


def _proto_to_status(self, status) -> AgentStatus:
    """Convert protobuf to AgentStatus"""
    from maple.core.ars.grpc import ars_pb2

    mapping = {
        ars_pb2.AGENT_STATUS_ACTIVE: AgentStatus.ACTIVE,
        ars_pb2.AGENT_STATUS_INACTIVE: AgentStatus.INACTIVE,
        ars_pb2.AGENT_STATUS_MAINTENANCE: AgentStatus.MAINTENANCE,
        ars_pb2.AGENT_STATUS_DEPRECATED: AgentStatus.DEPRECATED
    }
    return mapping.get(status, AgentStatus.ACTIVE)


def _health_to_proto(self, health: HealthStatus):
    """Convert HealthStatus to protobuf"""
    from maple.core.ars.grpc import ars_pb2

    mapping = {
        HealthStatus.HEALTHY: ars_pb2.HEALTH_STATUS_HEALTHY,
        HealthStatus.DEGRADED: ars_pb2.HEALTH_STATUS_DEGRADED,
        HealthStatus.UNHEALTHY: ars_pb2.HEALTH_STATUS_UNHEALTHY,
        HealthStatus.UNKNOWN: ars_pb2.HEALTH_STATUS_UNKNOWN
    }
    return mapping.get(health, ars_pb2.HEALTH_STATUS_UNKNOWN)


def _proto_to_health(self, health) -> HealthStatus:
    """Convert protobuf to HealthStatus"""
    from maple.core.ars.grpc import ars_pb2

    mapping = {
        ars_pb2.HEALTH_STATUS_HEALTHY: HealthStatus.HEALTHY,
        ars_pb2.HEALTH_STATUS_DEGRADED: HealthStatus.DEGRADED,
        ars_pb2.HEALTH_STATUS_UNHEALTHY: HealthStatus.UNHEALTHY,
        ars_pb2.HEALTH_STATUS_UNKNOWN: HealthStatus.UNKNOWN
    }
    return mapping.get(health, HealthStatus.UNKNOWN)


def _search_strategy_to_proto(self, strategy: SearchStrategy):
    """Convert SearchStrategy to protobuf"""
    from maple.core.ars.grpc import ars_pb2

    mapping = {
        SearchStrategy.EXACT: ars_pb2.SEARCH_STRATEGY_EXACT,
        SearchStrategy.FUZZY: ars_pb2.SEARCH_STRATEGY_FUZZY,
        SearchStrategy.SEMANTIC: ars_pb2.SEARCH_STRATEGY_SEMANTIC,
        SearchStrategy.HYBRID: ars_pb2.SEARCH_STRATEGY_HYBRID
    }
    return mapping.get(strategy, ars_pb2.SEARCH_STRATEGY_HYBRID)


def _dict_to_struct(self, data: Dict[str, Any]):
    """Convert dict to protobuf Struct"""
    from google.protobuf import struct_pb2

    struct = struct_pb2.Struct()
    if data:
        struct.update(data)
    return struct


def _struct_to_dict(self, struct) -> Dict[str, Any]:
    """Convert protobuf Struct to dict"""
    from google.protobuf.json_format import MessageToDict

    if struct:
        return MessageToDict(struct)
    return {}


# Convenience functions and utilities

class AgentManager:
    """
    High-level agent management class.
    Provides simplified interface for common agent operations.
    """

    def __init__(self, client: ARSClient):
        self.client = client
        self._heartbeat_tasks: Dict[str, asyncio.Task] = {}
        self._event_handlers: Dict[str, List[Callable]] = defaultdict(list)
        self._running = False

    async def start(self) -> None:
        """Start agent manager"""
        self._running = True

        # Start event monitoring
        asyncio.create_task(self._monitor_events())

    async def stop(self) -> None:
        """Stop agent manager"""
        self._running = False

        # Cancel heartbeat tasks
        for task in self._heartbeat_tasks.values():
            task.cancel()

        # Wait for tasks to complete
        if self._heartbeat_tasks:
            await asyncio.gather(
                *self._heartbeat_tasks.values(),
                return_exceptions=True
            )

    async def register_and_maintain(
            self,
            name: str,
            version: str,
            capabilities: List[Capability],
            endpoints: List[Endpoint],
            metadata: Optional[Dict[str, Any]] = None,
            heartbeat_interval: int = 30,
            heartbeat_callback: Optional[Callable] = None
    ) -> AgentRegistration:
        """
        Register an agent and automatically maintain its heartbeat.

        Args:
            name: Agent name
            version: Agent version
            capabilities: Agent capabilities
            endpoints: Agent endpoints
            metadata: Agent metadata
            heartbeat_interval: Heartbeat interval in seconds
            heartbeat_callback: Optional callback to get metrics

        Returns:
            Agent registration
        """
        # Register agent
        registration = await self.client.register_agent(
            name=name,
            version=version,
            capabilities=capabilities,
            endpoints=endpoints,
            metadata=metadata
        )

        # Start heartbeat
        self._heartbeat_tasks[registration.agent_id] = asyncio.create_task(
            self._heartbeat_loop(
                registration.agent_id,
                heartbeat_interval,
                heartbeat_callback
            )
        )

        return registration

    async def deregister_and_cleanup(self, agent_id: str) -> bool:
        """Deregister agent and cleanup resources"""
        # Stop heartbeat
        if agent_id in self._heartbeat_tasks:
            self._heartbeat_tasks[agent_id].cancel()
            await asyncio.gather(
                self._heartbeat_tasks[agent_id],
                return_exceptions=True
            )
            del self._heartbeat_tasks[agent_id]

        # Deregister agent
        return await self.client.deregister_agent(agent_id)

    def on_event(self, event_type: str, handler: Callable) -> None:
        """Register event handler"""
        self._event_handlers[event_type].append(handler)

    def off_event(self, event_type: str, handler: Callable) -> None:
        """Unregister event handler"""
        if handler in self._event_handlers[event_type]:
            self._event_handlers[event_type].remove(handler)

    async def find_best_agent(
            self,
            capabilities: List[str],
            prefer_healthy: bool = True,
            max_results: int = 5
    ) -> List[AgentRegistration]:
        """
        Find the best agents matching capabilities.

        Args:
            capabilities: Required capabilities
            prefer_healthy: Prefer healthy agents
            max_results: Maximum results to return

        Returns:
            List of best matching agents
        """
        # Discover agents
        agents = await self.client.discover_agents(
            capabilities=capabilities,
            require_all_capabilities=True,
            search_strategy=SearchStrategy.HYBRID,
            limit=max_results * 3  # Get more to filter
        )

        if not agents:
            return []

        # Score agents
        scored_agents = []
        for agent in agents:
            score = 0.0

            # Health score
            if agent.health_status == HealthStatus.HEALTHY:
                score += 1.0 if prefer_healthy else 0.5
            elif agent.health_status == HealthStatus.DEGRADED:
                score += 0.5 if prefer_healthy else 0.3

            # Capability match score
            agent_caps = {cap.name for cap in agent.capabilities}
            matched = len(set(capabilities) & agent_caps)
            score += matched / len(capabilities)

            # Recency score (prefer recently updated)
            time_since_heartbeat = datetime.utcnow() - agent.last_heartbeat
            if time_since_heartbeat < timedelta(minutes=1):
                score += 0.5
            elif time_since_heartbeat < timedelta(minutes=5):
                score += 0.3

            scored_agents.append((agent, score))

        # Sort by score
        scored_agents.sort(key=lambda x: x[1], reverse=True)

        return [agent for agent, _ in scored_agents[:max_results]]

    async def _heartbeat_loop(
            self,
            agent_id: str,
            interval: int,
            callback: Optional[Callable] = None
    ) -> None:
        """Background heartbeat loop"""
        while self._running:
            try:
                # Get metrics if callback provided
                metrics = None
                if callback:
                    if asyncio.iscoroutinefunction(callback):
                        metrics = await callback()
                    else:
                        metrics = callback()

                # Send heartbeat
                success = await self.client.heartbeat(agent_id, metrics)

                if not success:
                    logger.warning(f"Heartbeat failed for agent {agent_id}")

                # Wait for next interval
                await asyncio.sleep(interval)

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Error in heartbeat loop for {agent_id}: {e}")
                await asyncio.sleep(interval)

    async def _monitor_events(self) -> None:
        """Monitor and dispatch events"""
        while self._running:
            try:
                async for event in self.client.stream_events():
                    # Dispatch to handlers
                    handlers = self._event_handlers.get(event.event_type, [])
                    handlers.extend(self._event_handlers.get("*", []))  # Wildcard

                    for handler in handlers:
                        try:
                            if asyncio.iscoroutinefunction(handler):
                                await handler(event)
                            else:
                                handler(event)
                        except Exception as e:
                            logger.error(f"Error in event handler: {e}")

            except Exception as e:
                logger.error(f"Error in event monitor: {e}")
                await asyncio.sleep(5)  # Retry after delay


# Batch operations helper

class BatchOperations:
    """Helper for batch operations on ARS"""

    def __init__(self, client: ARSClient):
        self.client = client

    async def register_multiple(
            self,
            agents: List[Dict[str, Any]],
            parallel: int = 10
    ) -> List[Union[AgentRegistration, Exception]]:
        """Register multiple agents in parallel"""
        semaphore = asyncio.Semaphore(parallel)

        async def register_with_limit(agent_data):
            async with semaphore:
                try:
                    return await self.client.register_agent(**agent_data)
                except Exception as e:
                    return e

        tasks = [register_with_limit(agent) for agent in agents]
        return await asyncio.gather(*tasks)

    async def update_health_multiple(
            self,
            updates: List[Dict[str, Any]],
            parallel: int = 20
    ) -> List[bool]:
        """Update health for multiple agents"""
        semaphore = asyncio.Semaphore(parallel)

        async def update_with_limit(update):
            async with semaphore:
                try:
                    return await self.client.update_health(**update)
                except:
                    return False

        tasks = [update_with_limit(update) for update in updates]
        return await asyncio.gather(*tasks)

    async def discover_by_patterns(
            self,
            patterns: List[Dict[str, Any]]
    ) -> Dict[str, List[AgentRegistration]]:
        """Discover agents using multiple patterns"""
        results = {}

        for idx, pattern in enumerate(patterns):
            key = pattern.get("name", f"pattern_{idx}")
            agents = await self.client.discover_agents(**pattern)
            results[key] = agents

        return results


# Testing utilities

class MockARSClient(ARSClient):
    """Mock ARS client for testing"""

    def __init__(self):
        super().__init__(ClientConfig(protocol=ClientProtocol.REST))
        self._agents: Dict[str, AgentRegistration] = {}
        self._events: List[RegistryEvent] = []

    async def connect(self) -> None:
        pass

    async def close(self) -> None:
        pass

    async def register_agent(
            self,
            name: str,
            version: str,
            capabilities: List[Capability],
            endpoints: List[Endpoint],
            metadata: Optional[Dict[str, Any]] = None,
            agent_id: Optional[str] = None
    ) -> AgentRegistration:
        agent_id = agent_id or f"{name}-{len(self._agents)}"

        registration = AgentRegistration(
            agent_id=agent_id,
            name=name,
            version=version,
            status=AgentStatus.ACTIVE,
            health_status=HealthStatus.HEALTHY,
            capabilities=capabilities,
            endpoints=endpoints,
            metadata=metadata or {},
            metrics={},
            created_at=datetime.utcnow(),
            last_heartbeat=datetime.utcnow()
        )

        self._agents[agent_id] = registration
        return registration

    async def get_agent(self, agent_id: str) -> Optional[AgentRegistration]:
        return self._agents.get(agent_id)

    async def discover_agents(self, **kwargs) -> List[AgentRegistration]:
        # Simple filtering
        results = list(self._agents.values())

        if kwargs.get("capabilities"):
            required_caps = set(kwargs["capabilities"])
            results = [
                agent for agent in results
                if required_caps.issubset({cap.name for cap in agent.capabilities})
            ]

        return results


# Export public API
__all__ = [
    "ARSClient",
    "ARSRestClient",
    "ARSGrpcClient",
    "ClientConfig",
    "ClientProtocol",
    "AgentManager",
    "BatchOperations",
    "MockARSClient"
]