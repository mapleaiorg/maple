# File: core/ars/health.py
# Description: Health monitoring system for registered agents.
# Provides heartbeat tracking, health checks, and automated recovery.

from __future__ import annotations
import asyncio
import aiohttp
from datetime import datetime, timedelta
from typing import List, Optional, Dict, Any, Set, Callable
import logging
from dataclasses import dataclass, field
from enum import Enum
import json
from collections import defaultdict, deque

from core.ars.models.registry import (
    AgentRegistration, AgentStatus, HealthStatus
)
from core.ars.storage.interface import RegistryStorage
from core.ars.events import EventBus

logger = logging.getLogger(__name__)


class HealthCheckType(str, Enum):
    """Types of health checks"""
    PING = "ping"
    HTTP = "http"
    TCP = "tcp"
    GRPC = "grpc"
    CUSTOM = "custom"


@dataclass
class HealthCheckConfig:
    """Configuration for health checks"""
    check_type: HealthCheckType = HealthCheckType.HTTP
    endpoint: Optional[str] = None
    timeout: int = 5
    retries: int = 3
    interval: int = 30
    success_threshold: int = 2
    failure_threshold: int = 3
    custom_check: Optional[Callable] = None


@dataclass
class HealthMetrics:
    """Health metrics for an agent"""
    agent_id: str
    last_check: Optional[datetime] = None
    last_success: Optional[datetime] = None
    consecutive_successes: int = 0
    consecutive_failures: int = 0
    total_checks: int = 0
    total_successes: int = 0
    total_failures: int = 0
    average_response_time: float = 0.0
    response_times: deque = field(default_factory=lambda: deque(maxlen=100))
    error_counts: Dict[str, int] = field(default_factory=lambda: defaultdict(int))

    def update_success(self, response_time: float) -> None:
        """Update metrics for successful check"""
        self.last_check = datetime.utcnow()
        self.last_success = self.last_check
        self.consecutive_successes += 1
        self.consecutive_failures = 0
        self.total_checks += 1
        self.total_successes += 1

        # Update response time metrics
        self.response_times.append(response_time)
        self.average_response_time = sum(self.response_times) / len(self.response_times)

    def update_failure(self, error_type: str) -> None:
        """Update metrics for failed check"""
        self.last_check = datetime.utcnow()
        self.consecutive_successes = 0
        self.consecutive_failures += 1
        self.total_checks += 1
        self.total_failures += 1
        self.error_counts[error_type] += 1

    @property
    def success_rate(self) -> float:
        """Calculate success rate"""
        if self.total_checks == 0:
            return 0.0
        return self.total_successes / self.total_checks

    @property
    def is_healthy(self) -> bool:
        """Check if agent is considered healthy"""
        if self.last_success is None:
            return False

        # Check if last success was recent
        time_since_success = datetime.utcnow() - self.last_success
        return time_since_success < timedelta(minutes=5)


class HealthMonitor:
    """
    Health monitoring system for registered agents.
    Performs periodic health checks and updates agent status.
    """

    def __init__(
            self,
            storage: RegistryStorage,
            config: Optional[Dict[str, Any]] = None,
            event_bus: Optional[EventBus] = None
    ):
        self._storage = storage
        self._config = config or {}
        self._event_bus = event_bus

        # Health check configurations per agent
        self._health_configs: Dict[str, HealthCheckConfig] = {}

        # Health metrics per agent
        self._health_metrics: Dict[str, HealthMetrics] = {}

        # Active health check tasks
        self._check_tasks: Dict[str, asyncio.Task] = {}

        # Global configuration
        self.default_check_interval = self._config.get('health_check_interval', 30)
        self.max_concurrent_checks = self._config.get('max_concurrent_checks', 100)
        self.enable_auto_recovery = self._config.get('enable_auto_recovery', True)

        # HTTP session for health checks
        self._session: Optional[aiohttp.ClientSession] = None

        # Semaphore to limit concurrent checks
        self._check_semaphore = asyncio.Semaphore(self.max_concurrent_checks)

        self._running = False

    async def start(self) -> None:
        """Start health monitor"""
        if self._running:
            return

        self._running = True

        # Create HTTP session
        self._session = aiohttp.ClientSession(
            timeout=aiohttp.ClientTimeout(total=10)
        )

        logger.info("Health monitor started")

    async def stop(self) -> None:
        """Stop health monitor"""
        if not self._running:
            return

        self._running = False

        # Cancel all health check tasks
        for task in self._check_tasks.values():
            task.cancel()

        # Wait for tasks to complete
        if self._check_tasks:
            await asyncio.gather(
                *self._check_tasks.values(),
                return_exceptions=True
            )

        self._check_tasks.clear()

        # Close HTTP session
        if self._session:
            await self._session.close()

        logger.info("Health monitor stopped")

    async def register_agent_health_check(
            self,
            agent_id: str,
            config: Optional[HealthCheckConfig] = None
    ) -> None:
        """Register health check for an agent"""
        # Use provided config or create default
        if config:
            self._health_configs[agent_id] = config
        else:
            # Get agent to determine health check endpoint
            agent = await self._storage.get_agent(agent_id)
            if not agent:
                raise ValueError(f"Agent {agent_id} not found")

            # Create default config based on agent endpoints
            health_endpoint = None
            for endpoint in agent.endpoints:
                if endpoint.type == "health":
                    health_endpoint = endpoint.url
                    break

            if not health_endpoint:
                # Use first HTTP endpoint
                for endpoint in agent.endpoints:
                    if endpoint.type == "http":
                        health_endpoint = f"{endpoint.url}/health"
                        break

            self._health_configs[agent_id] = HealthCheckConfig(
                check_type=HealthCheckType.HTTP,
                endpoint=health_endpoint,
                interval=self.default_check_interval
            )

        # Initialize metrics
        self._health_metrics[agent_id] = HealthMetrics(agent_id=agent_id)

        # Start health check task
        if self._running:
            await self._start_health_check_task(agent_id)

        logger.info(f"Registered health check for agent {agent_id}")

    async def unregister_agent_health_check(self, agent_id: str) -> None:
        """Unregister health check for an agent"""
        # Cancel health check task
        if agent_id in self._check_tasks:
            self._check_tasks[agent_id].cancel()
            await asyncio.gather(
                self._check_tasks[agent_id],
                return_exceptions=True
            )
            del self._check_tasks[agent_id]

        # Remove configuration and metrics
        self._health_configs.pop(agent_id, None)
        self._health_metrics.pop(agent_id, None)

        logger.info(f"Unregistered health check for agent {agent_id}")

    async def check_agent_health(
            self,
            agent_id: str,
            force: bool = False
    ) -> HealthStatus:
        """
        Perform health check for a specific agent.

        Args:
            agent_id: Agent to check
            force: Force check even if recently checked

        Returns:
            Current health status
        """
        if agent_id not in self._health_configs:
            raise ValueError(f"No health check configured for agent {agent_id}")

        config = self._health_configs[agent_id]
        metrics = self._health_metrics[agent_id]

        # Check if we should skip (unless forced)
        if not force and metrics.last_check:
            time_since_check = datetime.utcnow() - metrics.last_check
            if time_since_check < timedelta(seconds=config.interval / 2):
                # Return current status
                return await self._get_current_health_status(agent_id)

        # Perform health check
        async with self._check_semaphore:
            success, response_time, error = await self._perform_health_check(
                agent_id,
                config
            )

        # Update metrics
        if success:
            metrics.update_success(response_time)
        else:
            metrics.update_failure(error or "unknown")

        # Determine health status
        health_status = self._determine_health_status(metrics, config)

        # Update agent health in storage
        await self._storage.update_health(
            agent_id,
            health_status,
            {
                "response_time": response_time if success else None,
                "success_rate": metrics.success_rate,
                "last_error": error if not success else None
            }
        )

        # Emit event if status changed
        await self._emit_health_event(agent_id, health_status, metrics)

        return health_status

    async def check_all_agents(self) -> Dict[str, HealthStatus]:
        """Perform health checks for all registered agents"""
        # Get all active agents
        from core.ars.models.registry import ServiceQuery

        query = ServiceQuery(status=AgentStatus.ACTIVE)
        agents = await self._storage.query_agents(query)

        # Register health checks for new agents
        for agent in agents:
            if agent.agent_id not in self._health_configs:
                await self.register_agent_health_check(agent.agent_id)

        # Perform health checks concurrently
        tasks = []
        for agent in agents:
            task = asyncio.create_task(
                self.check_agent_health(agent.agent_id)
            )
            tasks.append((agent.agent_id, task))

        # Wait for all checks to complete
        results = {}
        for agent_id, task in tasks:
            try:
                health_status = await task
                results[agent_id] = health_status
            except Exception as e:
                logger.error(f"Health check failed for {agent_id}: {e}")
                results[agent_id] = HealthStatus.UNKNOWN

        return results

    async def get_health_metrics(
            self,
            agent_id: str
    ) -> Optional[HealthMetrics]:
        """Get health metrics for an agent"""
        return self._health_metrics.get(agent_id)

    async def get_unhealthy_agents(self) -> List[str]:
        """Get list of unhealthy agents"""
        unhealthy = []

        for agent_id, metrics in self._health_metrics.items():
            if not metrics.is_healthy:
                unhealthy.append(agent_id)

        return unhealthy

    # Private methods

    async def _start_health_check_task(self, agent_id: str) -> None:
        """Start background health check task for an agent"""
        if agent_id in self._check_tasks:
            # Cancel existing task
            self._check_tasks[agent_id].cancel()

        # Create new task
        task = asyncio.create_task(
            self._health_check_loop(agent_id)
        )
        self._check_tasks[agent_id] = task

    async def _health_check_loop(self, agent_id: str) -> None:
        """Background loop for periodic health checks"""
        config = self._health_configs[agent_id]

        while self._running:
            try:
                # Perform health check
                await self.check_agent_health(agent_id)

                # Wait for next interval
                await asyncio.sleep(config.interval)

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Error in health check loop for {agent_id}: {e}")
                # Wait before retry
                await asyncio.sleep(config.interval)

    async def _perform_health_check(
            self,
            agent_id: str,
            config: HealthCheckConfig
    ) -> tuple[bool, float, Optional[str]]:
        """
        Perform actual health check.

        Returns:
            Tuple of (success, response_time, error_message)
        """
        start_time = datetime.utcnow()

        try:
            if config.check_type == HealthCheckType.HTTP:
                success = await self._http_health_check(agent_id, config)
            elif config.check_type == HealthCheckType.TCP:
                success = await self._tcp_health_check(agent_id, config)
            elif config.check_type == HealthCheckType.GRPC:
                success = await self._grpc_health_check(agent_id, config)
            elif config.check_type == HealthCheckType.CUSTOM:
                success = await self._custom_health_check(agent_id, config)
            else:
                success = await self._ping_health_check(agent_id, config)

            response_time = (datetime.utcnow() - start_time).total_seconds()

            return success, response_time, None

        except asyncio.TimeoutError:
            response_time = (datetime.utcnow() - start_time).total_seconds()
            return False, response_time, "timeout"
        except Exception as e:
            response_time = (datetime.utcnow() - start_time).total_seconds()
            return False, response_time, str(e)

    async def _http_health_check(
            self,
            agent_id: str,
            config: HealthCheckConfig
    ) -> bool:
        """Perform HTTP health check"""
        if not config.endpoint:
            raise ValueError(f"No health endpoint configured for {agent_id}")

        if not self._session:
            raise RuntimeError("HTTP session not initialized")

        # Perform HTTP request with retries
        for attempt in range(config.retries):
            try:
                async with self._session.get(
                        config.endpoint,
                        timeout=aiohttp.ClientTimeout(total=config.timeout)
                ) as response:
                    # Check if response indicates health
                    if response.status == 200:
                        # Try to parse JSON response
                        try:
                            data = await response.json()
                            # Check for health status in response
                            if isinstance(data, dict):
                                status = data.get('status', 'ok')
                                return status in ['ok', 'healthy', 'up']
                        except:
                            # Non-JSON response, consider healthy if 200
                            pass

                        return True
                    elif response.status == 503:
                        # Service unavailable
                        return False
                    elif 200 <= response.status < 300:
                        # Other 2xx responses considered healthy
                        return True
                    else:
                        # Non-2xx response
                        if attempt < config.retries - 1:
                            await asyncio.sleep(1)  # Wait before retry
                            continue
                        return False

            except (aiohttp.ClientError, asyncio.TimeoutError) as e:
                if attempt < config.retries - 1:
                    await asyncio.sleep(1)  # Wait before retry
                    continue
                raise

        return False

    async def _tcp_health_check(
            self,
            agent_id: str,
            config: HealthCheckConfig
    ) -> bool:
        """Perform TCP health check"""
        if not config.endpoint:
            raise ValueError(f"No endpoint configured for {agent_id}")

        # Parse host and port from endpoint
        import urllib.parse
        parsed = urllib.parse.urlparse(f"//{config.endpoint}")
        host = parsed.hostname or 'localhost'
        port = parsed.port or 80

        try:
            # Try to establish TCP connection
            reader, writer = await asyncio.wait_for(
                asyncio.open_connection(host, port),
                timeout=config.timeout
            )

            # Connection successful
            writer.close()
            await writer.wait_closed()

            return True

        except (OSError, asyncio.TimeoutError):
            return False

    async def _grpc_health_check(
            self,
            agent_id: str,
            config: HealthCheckConfig
    ) -> bool:
        """Perform gRPC health check"""
        # Simplified gRPC health check
        # In production, would use grpc.health.v1
        return await self._tcp_health_check(agent_id, config)

    async def _custom_health_check(
            self,
            agent_id: str,
            config: HealthCheckConfig
    ) -> bool:
        """Perform custom health check"""
        if not config.custom_check:
            raise ValueError(f"No custom check function for {agent_id}")

        return await config.custom_check(agent_id)

    async def _ping_health_check(
            self,
            agent_id: str,
            config: HealthCheckConfig
    ) -> bool:
        """Simple ping health check (always returns True)"""
        # This is used when agent doesn't have specific health endpoint
        # Just checks if agent exists in registry
        agent = await self._storage.get_agent(agent_id)
        return agent is not None

    def _determine_health_status(
            self,
            metrics: HealthMetrics,
            config: HealthCheckConfig
    ) -> HealthStatus:
        """Determine health status based on metrics"""
        # Check consecutive failures
        if metrics.consecutive_failures >= config.failure_threshold:
            return HealthStatus.UNHEALTHY

        # Check consecutive successes
        if metrics.consecutive_successes >= config.success_threshold:
            return HealthStatus.HEALTHY

        # Check overall success rate
        if metrics.total_checks >= 10:
            if metrics.success_rate < 0.5:
                return HealthStatus.UNHEALTHY
            elif metrics.success_rate < 0.8:
                return HealthStatus.DEGRADED

        # Check last success time
        if metrics.last_success:
            time_since_success = datetime.utcnow() - metrics.last_success
            if time_since_success > timedelta(minutes=5):
                return HealthStatus.UNHEALTHY
            elif time_since_success > timedelta(minutes=2):
                return HealthStatus.DEGRADED

        # Default to current state or degraded
        return HealthStatus.DEGRADED

    async def _get_current_health_status(self, agent_id: str) -> HealthStatus:
        """Get current health status from storage"""
        agent = await self._storage.get_agent(agent_id)
        if agent:
            return agent.health_status
        return HealthStatus.UNKNOWN

    async def _emit_health_event(
            self,
            agent_id: str,
            health_status: HealthStatus,
            metrics: HealthMetrics
    ) -> None:
        """Emit health status change event"""
        if self._event_bus:
            await self._event_bus.emit(
                "agent.health_changed",
                {
                    "agent_id": agent_id,
                    "health_status": health_status,
                    "metrics": {
                        "success_rate": metrics.success_rate,
                        "consecutive_failures": metrics.consecutive_failures,
                        "average_response_time": metrics.average_response_time
                    }
                }
            )

    async def _attempt_recovery(self, agent_id: str) -> bool:
        """Attempt to recover unhealthy agent"""
        if not self.enable_auto_recovery:
            return False

        logger.info(f"Attempting recovery for agent {agent_id}")

        # Get agent information
        agent = await self._storage.get_agent(agent_id)
        if not agent:
            return False

        # Emit recovery attempt event
        if self._event_bus:
            await self._event_bus.emit(
                "agent.recovery_attempt",
                {"agent_id": agent_id}
            )

        # Recovery strategies would be implemented here
        # For example:
        # - Send restart command to agent
        # - Update agent configuration
        # - Notify administrators

        return False  # Placeholder


# Advanced health monitor with predictive capabilities

class PredictiveHealthMonitor(HealthMonitor):
    """
    Health monitor with predictive failure detection.
    Uses historical data to predict potential failures.
    """

    def __init__(
            self,
            storage: RegistryStorage,
            config: Optional[Dict[str, Any]] = None,
            event_bus: Optional[EventBus] = None
    ):
        super().__init__(storage, config, event_bus)

        # Prediction configuration
        self.enable_predictions = config.get('enable_predictions', True)
        self.prediction_window = config.get('prediction_window', 3600)  # 1 hour
        self.anomaly_threshold = config.get('anomaly_threshold', 2.0)  # std devs

        # Historical data for predictions
        self._historical_data: Dict[str, deque] = defaultdict(
            lambda: deque(maxlen=1000)
        )

    async def check_agent_health(
            self,
            agent_id: str,
            force: bool = False
    ) -> HealthStatus:
        """Enhanced health check with predictive analysis"""
        # Perform regular health check
        health_status = await super().check_agent_health(agent_id, force)

        # Perform predictive analysis
        if self.enable_predictions and agent_id in self._health_metrics:
            metrics = self._health_metrics[agent_id]

            # Store historical data
            self._historical_data[agent_id].append({
                'timestamp': datetime.utcnow(),
                'response_time': metrics.average_response_time,
                'success_rate': metrics.success_rate,
                'consecutive_failures': metrics.consecutive_failures
            })

            # Predict potential failures
            prediction = self._predict_failure(agent_id)

            if prediction['likely_failure']:
                logger.warning(
                    f"Potential failure predicted for agent {agent_id}: "
                    f"{prediction['reason']}"
                )

                # Emit prediction event
                if self._event_bus:
                    await self._event_bus.emit(
                        "agent.failure_predicted",
                        {
                            "agent_id": agent_id,
                            "prediction": prediction,
                            "current_health": health_status
                        }
                    )

        return health_status

    def _predict_failure(self, agent_id: str) -> Dict[str, Any]:
        """Predict potential agent failure"""
        historical = list(self._historical_data[agent_id])

        if len(historical) < 10:
            return {'likely_failure': False, 'reason': 'Insufficient data'}

        # Analyze trends
        recent_data = historical[-10:]

        # Check response time trend
        response_times = [d['response_time'] for d in recent_data]
        if self._is_trending_up(response_times, threshold=1.5):
            return {
                'likely_failure': True,
                'reason': 'Response time increasing',
                'confidence': 0.7
            }

        # Check success rate trend
        success_rates = [d['success_rate'] for d in recent_data]
        if self._is_trending_down(success_rates, threshold=0.8):
            return {
                'likely_failure': True,
                'reason': 'Success rate declining',
                'confidence': 0.8
            }

        # Check for anomalies
        if self._detect_anomaly(response_times):
            return {
                'likely_failure': True,
                'reason': 'Anomalous behavior detected',
                'confidence': 0.6
            }

        return {'likely_failure': False, 'reason': 'No issues detected'}

    def _is_trending_up(
            self,
            values: List[float],
            threshold: float = 1.5
    ) -> bool:
        """Check if values are trending upward"""
        if len(values) < 3:
            return False

        # Simple linear regression
        x = list(range(len(values)))
        mean_x = sum(x) / len(x)
        mean_y = sum(values) / len(values)

        numerator = sum((x[i] - mean_x) * (values[i] - mean_y) for i in range(len(values)))
        denominator = sum((x[i] - mean_x) ** 2 for i in range(len(values)))

        if denominator == 0:
            return False

        slope = numerator / denominator

        # Check if slope indicates significant increase
        return slope > (mean_y * 0.1)  # 10% increase per time unit

    def _is_trending_down(
            self,
            values: List[float],
            threshold: float = 0.8
    ) -> bool:
        """Check if values are trending downward"""
        if len(values) < 3:
            return False

        # Invert logic from trending up
        return self._is_trending_up([-v for v in values], threshold)

    def _detect_anomaly(self, values: List[float]) -> bool:
        """Detect anomalies using statistical methods"""
        if len(values) < 5:
            return False

        # Calculate mean and std deviation
        mean = sum(values) / len(values)
        variance = sum((x - mean) ** 2 for x in values) / len(values)
        std_dev = variance ** 0.5

        if std_dev == 0:
            return False

        # Check if latest value is anomalous
        z_score = abs(values[-1] - mean) / std_dev

        return z_score > self.anomaly_threshold


# Export public API
__all__ = [
    "HealthMonitor",
    "PredictiveHealthMonitor",
    "HealthCheckConfig",
    "HealthCheckType",
    "HealthMetrics"
]