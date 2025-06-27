# File: mall/core/environment.py
# Description: Environment monitoring and analysis for MALL. Collects real-time
# data about system state, agent performance, and resource utilization.

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, Set
from datetime import datetime, timedelta
import asyncio
import logging
import numpy as np
from collections import defaultdict, deque

logger = logging.getLogger(__name__)


@dataclass
class EnvironmentData:
    """Environmental data snapshot"""
    timestamp: datetime
    shard_id: str
    task_backlog: int
    active_agents: int
    resource_utilization: Dict[str, float]  # cpu, memory, network
    performance_metrics: Dict[str, float]  # latency, throughput, error_rate
    agent_capabilities: Dict[str, Set[str]]
    custom_metrics: Dict[str, Any] = field(default_factory=dict)


@dataclass
class ResourceMetrics:
    """Resource utilization metrics"""
    cpu_percent: float
    memory_percent: float
    network_in_mbps: float
    network_out_mbps: float
    disk_io_percent: float
    gpu_percent: Optional[float] = None


class EnvironmentMonitor:
    """
    Monitors environment state and collects metrics for auto-spawning decisions.
    Integrates with UAL's SNS (sense) verb for real-time data collection.
    """

    def __init__(
            self,
            shard_id: str,
            sample_interval: float = 1.0,
            history_size: int = 1000
    ):
        self.shard_id = shard_id
        self.sample_interval = sample_interval
        self.history_size = history_size

        # Metrics history
        self.task_backlog_history: deque = deque(maxlen=history_size)
        self.agent_count_history: deque = deque(maxlen=history_size)
        self.resource_history: deque = deque(maxlen=history_size)
        self.performance_history: deque = deque(maxlen=history_size)

        # Current state
        self.current_agents: Dict[str, Dict[str, Any]] = {}
        self.task_queue_sizes: Dict[str, int] = defaultdict(int)
        self.agent_capabilities: Dict[str, Set[str]] = defaultdict(set)

        # Monitoring task
        self._monitoring_task: Optional[asyncio.Task] = None
        self._running = False

        logger.info(f"Environment monitor initialized for shard {shard_id}")

    async def start(self) -> None:
        """Start environment monitoring"""
        logger.info(f"Starting environment monitor for shard {self.shard_id}")
        self._running = True
        self._monitoring_task = asyncio.create_task(self._monitoring_loop())

    async def stop(self) -> None:
        """Stop environment monitoring"""
        logger.info(f"Stopping environment monitor for shard {self.shard_id}")
        self._running = False

        if self._monitoring_task:
            self._monitoring_task.cancel()
            await asyncio.gather(self._monitoring_task, return_exceptions=True)

    async def sense_environment(self) -> EnvironmentData:
        """
        Collect current environment data.
        This implements the UAL SNS (sense) verb functionality.
        """
        # Collect resource metrics
        resources = await self._collect_resource_metrics()

        # Collect performance metrics
        performance = await self._collect_performance_metrics()

        # Calculate task backlog
        task_backlog = sum(self.task_queue_sizes.values())

        # Create snapshot
        env_data = EnvironmentData(
            timestamp=datetime.utcnow(),
            shard_id=self.shard_id,
            task_backlog=task_backlog,
            active_agents=len(self.current_agents),
            resource_utilization={
                "cpu": resources.cpu_percent,
                "memory": resources.memory_percent,
                "network_in": resources.network_in_mbps,
                "network_out": resources.network_out_mbps,
                "disk_io": resources.disk_io_percent,
            },
            performance_metrics=performance,
            agent_capabilities=dict(self.agent_capabilities)
        )

        # Add GPU if available
        if resources.gpu_percent is not None:
            env_data.resource_utilization["gpu"] = resources.gpu_percent

        # Update history
        self._update_history(env_data)

        return env_data

    async def register_agent(
            self,
            agent_id: str,
            capabilities: Set[str],
            metadata: Optional[Dict[str, Any]] = None
    ) -> None:
        """Register an agent with the monitor"""
        self.current_agents[agent_id] = {
            "capabilities": capabilities,
            "metadata": metadata or {},
            "registered_at": datetime.utcnow(),
            "last_seen": datetime.utcnow(),
        }
        self.agent_capabilities[agent_id] = capabilities
        logger.debug(f"Registered agent {agent_id} with capabilities: {capabilities}")

    async def unregister_agent(self, agent_id: str) -> None:
        """Unregister an agent"""
        if agent_id in self.current_agents:
            del self.current_agents[agent_id]
            del self.agent_capabilities[agent_id]
            logger.debug(f"Unregistered agent {agent_id}")

    async def update_task_queue(self, queue_name: str, size: int) -> None:
        """Update task queue size"""
        self.task_queue_sizes[queue_name] = size

    async def get_predictions(self) -> Dict[str, Any]:
        """Get environment predictions based on historical data"""
        if len(self.task_backlog_history) < 10:
            return {"insufficient_data": True}

        # Calculate trends
        recent_backlog = list(self.task_backlog_history)[-100:]
        backlog_trend = np.polyfit(range(len(recent_backlog)), recent_backlog, 1)[0]

        recent_agents = list(self.agent_count_history)[-100:]
        agent_trend = np.polyfit(range(len(recent_agents)), recent_agents, 1)[0]

        # Calculate averages
        avg_cpu = np.mean([r["cpu"] for r in self.resource_history])
        avg_memory = np.mean([r["memory"] for r in self.resource_history])

        # Predict future load
        future_backlog = recent_backlog[-1] + backlog_trend * 10  # 10 steps ahead

        return {
            "backlog_trend": backlog_trend,
            "agent_trend": agent_trend,
            "predicted_backlog": future_backlog,
            "average_cpu": avg_cpu,
            "average_memory": avg_memory,
            "recommendation": self._get_recommendation(
                future_backlog,
                avg_cpu,
                avg_memory,
                len(self.current_agents)
            )
        }

    def _get_recommendation(
            self,
            predicted_backlog: float,
            avg_cpu: float,
            avg_memory: float,
            current_agents: int
    ) -> str:
        """Get spawn recommendation based on predictions"""
        if predicted_backlog > current_agents * 10 and avg_cpu < 80:
            return "spawn_agents"
        elif predicted_backlog < current_agents * 2 and current_agents > 1:
            return "reduce_agents"
        else:
            return "maintain"

    async def _monitoring_loop(self) -> None:
        """Background monitoring loop"""
        while self._running:
            try:
                # Collect environment data
                await self.sense_environment()

                # Clean up stale agents
                await self._cleanup_stale_agents()

                await asyncio.sleep(self.sample_interval)
            except Exception as e:
                logger.error(f"Monitoring error: {e}")
                await asyncio.sleep(1)

    async def _collect_resource_metrics(self) -> ResourceMetrics:
        """Collect system resource metrics"""
        # In production, this would use psutil or similar
        # For now, return simulated metrics
        return ResourceMetrics(
            cpu_percent=np.random.uniform(20, 80),
            memory_percent=np.random.uniform(30, 70),
            network_in_mbps=np.random.uniform(10, 100),
            network_out_mbps=np.random.uniform(10, 100),
            disk_io_percent=np.random.uniform(5, 50),
            gpu_percent=np.random.uniform(0, 100) if np.random.random() > 0.5 else None
        )

    async def _collect_performance_metrics(self) -> Dict[str, float]:
        """Collect performance metrics"""
        # In production, this would aggregate from actual services
        return {
            "avg_latency_ms": np.random.uniform(10, 100),
            "throughput_rps": np.random.uniform(100, 1000),
            "error_rate": np.random.uniform(0, 0.05),
            "success_rate": np.random.uniform(0.95, 1.0),
        }

    def _update_history(self, env_data: EnvironmentData) -> None:
        """Update metrics history"""
        self.task_backlog_history.append(env_data.task_backlog)
        self.agent_count_history.append(env_data.active_agents)
        self.resource_history.append(env_data.resource_utilization)
        self.performance_history.append(env_data.performance_metrics)

    async def _cleanup_stale_agents(self) -> None:
        """Remove agents that haven't been seen recently"""
        stale_threshold = datetime.utcnow() - timedelta(minutes=5)
        stale_agents = [
            agent_id for agent_id, info in self.current_agents.items()
            if info["last_seen"] < stale_threshold
        ]

        for agent_id in stale_agents:
            await self.unregister_agent(agent_id)
            logger.warning(f"Removed stale agent {agent_id}")

    def get_metrics(self) -> Dict[str, Any]:
        """Get monitor metrics"""
        return {
            "shard_id": self.shard_id,
            "active_agents": len(self.current_agents),
            "total_capabilities": len(set().union(*self.agent_capabilities.values())),
            "task_backlog": sum(self.task_queue_sizes.values()),
            "history_size": len(self.task_backlog_history),
        }