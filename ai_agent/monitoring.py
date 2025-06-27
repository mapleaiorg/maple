# File: maple/ai_agent/monitoring.py
# Description: Performance monitoring and metrics collection for AI agents.
# Tracks model performance, latency, accuracy, and other key metrics.

import time
import asyncio
from typing import Dict, Any, List, Optional
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from collections import defaultdict, deque
import statistics
import logging

logger = logging.getLogger(__name__)


@dataclass
class PerformanceMetrics:
    """Performance metrics for AI agent"""
    total_requests: int = 0
    successful_requests: int = 0
    failed_requests: int = 0
    cache_hits: int = 0
    cache_misses: int = 0

    avg_latency: float = 0.0
    p95_latency: float = 0.0
    p99_latency: float = 0.0

    tokens_used: int = 0
    cost_estimate: float = 0.0

    model_usage: Dict[str, int] = field(default_factory=dict)
    error_counts: Dict[str, int] = field(default_factory=dict)
    task_type_counts: Dict[str, int] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "total_requests": self.total_requests,
            "successful_requests": self.successful_requests,
            "failed_requests": self.failed_requests,
            "success_rate": self.successful_requests / self.total_requests if self.total_requests > 0 else 0,
            "cache_hit_rate": self.cache_hits / (self.cache_hits + self.cache_misses) if (self.cache_hits + self.cache_misses) > 0 else 0,
            "avg_latency": self.avg_latency,
            "p95_latency": self.p95_latency,
            "p99_latency": self.p99_latency,
            "tokens_used": self.tokens_used,
            "cost_estimate": self.cost_estimate,
            "model_usage": self.model_usage,
            "error_counts": self.error_counts,
            "task_type_counts": self.task_type_counts
        }

class AgentMonitor:
    """Monitor for tracking AI agent performance"""

    def __init__(
            self,
            agent_id: str,
            window_size: int = 1000,
            metrics_interval: int = 60
    ):
        self.agent_id = agent_id
        self.window_size = window_size
        self.metrics_interval = metrics_interval

        # Current metrics
        self.metrics = PerformanceMetrics()

        # Time series data
        self.latency_history = deque(maxlen=window_size)
        self.request_history = deque(maxlen=window_size)

        # Hourly aggregates
        self.hourly_metrics = defaultdict(lambda: PerformanceMetrics())

        # Background task
        self._metrics_task = None
        self._running = False

    async def start(self):
        """Start monitoring"""
        self._running = True
        self._metrics_task = asyncio.create_task(
            self._aggregate_metrics_periodically()
        )
        logger.info(f"Started monitoring for agent {self.agent_id}")

    async def stop(self):
        """Stop monitoring"""
        self._running = False
        if self._metrics_task:
            self._metrics_task.cancel()
            await asyncio.gather(self._metrics_task, return_exceptions=True)
        logger.info(f"Stopped monitoring for agent {self.agent_id}")

    def record_request_start(self, task_id: str) -> float:
        """Record start of request"""
        start_time = time.time()
        self.request_history.append({
            "task_id": task_id,
            "start_time": start_time,
            "timestamp": datetime.utcnow()
        })
        return start_time

    def record_task_completion(
            self,
            task_id: str,
            duration: float,
            models_used: List[str],
            success: bool,
            error: Optional[str] = None,
            tokens_used: Optional[int] = None,
            task_type: Optional[str] = None
    ):
        """Record task completion"""

        # Update totals
        self.metrics.total_requests += 1
        if success:
            self.metrics.successful_requests += 1
        else:
            self.metrics.failed_requests += 1
            if error:
                self.metrics.error_counts[error] = \
                    self.metrics.error_counts.get(error, 0) + 1

        # Update latency
        self.latency_history.append(duration)

        # Update model usage
        for model in models_used:
            self.metrics.model_usage[model] = \
                self.metrics.model_usage.get(model, 0) + 1

        # Update token usage
        if tokens_used:
            self.metrics.tokens_used += tokens_used

        # Update task type counts
        if task_type:
            self.metrics.task_type_counts[task_type] = \
                self.metrics.task_type_counts.get(task_type, 0) + 1

        # Update hourly metrics
        hour_key = datetime.utcnow().strftime("%Y-%m-%d-%H")
        hourly = self.hourly_metrics[hour_key]
        hourly.total_requests += 1
        if success:
            hourly.successful_requests += 1
        else:
            hourly.failed_requests += 1

    def record_cache_hit(self):
        """Record cache hit"""
        self.metrics.cache_hits += 1

    def record_cache_miss(self):
        """Record cache miss"""
        self.metrics.cache_misses += 1

    def update_cost_estimate(self, cost: float):
        """Update cost estimate"""
        self.metrics.cost_estimate += cost

    async def get_metrics(self) -> Dict[str, Any]:
        """Get current metrics"""

        # Calculate latency percentiles
        if self.latency_history:
            latencies = list(self.latency_history)
            self.metrics.avg_latency = statistics.mean(latencies)
            self.metrics.p95_latency = self._percentile(latencies, 0.95)
            self.metrics.p99_latency = self._percentile(latencies, 0.99)

        return self.metrics.to_dict()

    async def get_summary(self) -> Dict[str, Any]:
        """Get metrics summary"""

        metrics = await self.get_metrics()

        # Add time-based analysis
        current_hour = datetime.utcnow().strftime("%Y-%m-%d-%H")
        last_hour = (datetime.utcnow() - timedelta(hours=1)).strftime("%Y-%m-%d-%H")

        current_hour_metrics = self.hourly_metrics[current_hour]
        last_hour_metrics = self.hourly_metrics[last_hour]

        # Calculate trends
        request_trend = "stable"
        if current_hour_metrics.total_requests > last_hour_metrics.total_requests * 1.2:
            request_trend = "increasing"
        elif current_hour_metrics.total_requests < last_hour_metrics.total_requests * 0.8:
            request_trend = "decreasing"

        return {
            **metrics,
            "current_hour_requests": current_hour_metrics.total_requests,
            "last_hour_requests": last_hour_metrics.total_requests,
            "request_trend": request_trend,
            "monitoring_window_size": len(self.latency_history),
            "uptime_hours": len(self.hourly_metrics)
        }

    def _percentile(self, data: List[float], percentile: float) -> float:
        """Calculate percentile"""
        if not data:
            return 0.0

        sorted_data = sorted(data)
        index = int(len(sorted_data) * percentile)
        return sorted_data[min(index, len(sorted_data) - 1)]

    async def _aggregate_metrics_periodically(self):
        """Periodically aggregate and clean up metrics"""

        while self._running:
            try:
                # Clean up old hourly metrics (keep last 24 hours)
                cutoff = datetime.utcnow() - timedelta(hours=24)
                cutoff_key = cutoff.strftime("%Y-%m-%d-%H")

                keys_to_remove = [
                    key for key in self.hourly_metrics.keys()
                    if key < cutoff_key
                ]

                for key in keys_to_remove:
                    del self.hourly_metrics[key]

                # Log current metrics
                metrics = await self.get_metrics()
                logger.info(
                    f"Agent {self.agent_id} metrics: "
                    f"requests={metrics['total_requests']}, "
                    f"success_rate={metrics['success_rate']:.2f}, "
                    f"avg_latency={metrics['avg_latency']:.0f}ms"
                )

                # Wait for next interval
                await asyncio.sleep(self.metrics_interval)

            except Exception as e:
                logger.error(f"Error in metrics aggregation: {e}")
                await asyncio.sleep(self.metrics_interval)


class ModelPerformanceTracker:
    """Track individual model performance"""

    def __init__(self):
        self.model_metrics: Dict[str, Dict[str, Any]] = defaultdict(
            lambda: {
                "total_requests": 0,
                "successful_requests": 0,
                "failed_requests": 0,
                "total_latency": 0.0,
                "total_tokens": 0,
                "accuracy_scores": deque(maxlen=100),
                "task_performance": defaultdict(lambda: {
                    "success": 0,
                    "total": 0
                })
            }
        )

    def record_model_performance(
            self,
            model: str,
            success: bool,
            latency: float,
            tokens: int,
            task_type: str,
            accuracy_score: Optional[float] = None
    ):
        """Record performance for a specific model"""

        metrics = self.model_metrics[model]

        # Update counts
        metrics["total_requests"] += 1
        if success:
            metrics["successful_requests"] += 1
        else:
            metrics["failed_requests"] += 1

        # Update latency and tokens
        metrics["total_latency"] += latency
        metrics["total_tokens"] += tokens

        # Update task-specific performance
        task_perf = metrics["task_performance"][task_type]
        task_perf["total"] += 1
        if success:
            task_perf["success"] += 1

        # Record accuracy if provided
        if accuracy_score is not None:
            metrics["accuracy_scores"].append(accuracy_score)

    def get_model_report(self, model: str) -> Dict[str, Any]:
        """Get performance report for a model"""

        metrics = self.model_metrics.get(model)
        if not metrics:
            return {}

        total = metrics["total_requests"]
        if total == 0:
            return {}

        # Calculate averages
        avg_latency = metrics["total_latency"] / total
        success_rate = metrics["successful_requests"] / total

        # Calculate accuracy
        accuracy_scores = list(metrics["accuracy_scores"])
        avg_accuracy = statistics.mean(accuracy_scores) if accuracy_scores else None

        # Task breakdown
        task_breakdown = {}
        for task_type, perf in metrics["task_performance"].items():
            if perf["total"] > 0:
                task_breakdown[task_type] = {
                    "success_rate": perf["success"] / perf["total"],
                    "total_requests": perf["total"]
                }

        return {
            "model": model,
            "total_requests": total,
            "success_rate": success_rate,
            "avg_latency": avg_latency,
            "avg_tokens_per_request": metrics["total_tokens"] / total,
            "avg_accuracy": avg_accuracy,
            "task_breakdown": task_breakdown
        }

    def get_all_models_report(self) -> List[Dict[str, Any]]:
        """Get performance report for all models"""

        reports = []
        for model in self.model_metrics.keys():
            report = self.get_model_report(model)
            if report:
                reports.append(report)

        # Sort by total requests
        reports.sort(key=lambda x: x["total_requests"], reverse=True)

        return reports


class AlertManager:
    """Manage alerts based on performance thresholds"""

    def __init__(self, thresholds: Optional[Dict[str, Any]] = None):
        self.thresholds = thresholds or {
            "error_rate": 0.1,  # 10% error rate
            "latency_p95": 5000,  # 5 seconds
            "cache_hit_rate": 0.3,  # 30% minimum
            "cost_per_hour": 10.0  # $10/hour
        }

        self.active_alerts: Dict[str, Dict[str, Any]] = {}

    def check_metrics(self, metrics: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Check metrics against thresholds and generate alerts"""

        alerts = []
        timestamp = datetime.utcnow()

        # Check error rate
        error_rate = 1 - metrics.get("success_rate", 1)
        if error_rate > self.thresholds["error_rate"]:
            alerts.append({
                "type": "high_error_rate",
                "severity": "critical",
                "message": f"Error rate {error_rate:.1%} exceeds threshold",
                "value": error_rate,
                "threshold": self.thresholds["error_rate"],
                "timestamp": timestamp
            })

        # Check latency
        p95_latency = metrics.get("p95_latency", 0)
        if p95_latency > self.thresholds["latency_p95"]:
            alerts.append({
                "type": "high_latency",
                "severity": "warning",
                "message": f"P95 latency {p95_latency:.0f}ms exceeds threshold",
                "value": p95_latency,
                "threshold": self.thresholds["latency_p95"],
                "timestamp": timestamp
            })

        # Check cache hit rate
        cache_hit_rate = metrics.get("cache_hit_rate", 1)
        if cache_hit_rate < self.thresholds["cache_hit_rate"]:
            alerts.append({
                "type": "low_cache_hit_rate",
                "severity": "info",
                "message": f"Cache hit rate {cache_hit_rate:.1%} below threshold",
                "value": cache_hit_rate,
                "threshold": self.thresholds["cache_hit_rate"],
                "timestamp": timestamp
            })

        # Update active alerts
        for alert in alerts:
            alert_key = alert["type"]
            self.active_alerts[alert_key] = alert

        # Clear resolved alerts
        for alert_type in list(self.active_alerts.keys()):
            if not any(a["type"] == alert_type for a in alerts):
                del self.active_alerts[alert_type]

        return alerts

    def get_active_alerts(self) -> List[Dict[str, Any]]:
        """Get all active alerts"""
        return list(self.active_alerts.values())