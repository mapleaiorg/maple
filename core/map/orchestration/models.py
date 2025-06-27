# File: core/map/orchestration/models.py
# Description: Core data models and types for the workflow orchestration engine.
# Defines workflow states, execution contexts, and base abstractions.

from __future__ import annotations
import asyncio
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum
from typing import Dict, List, Optional, Any, Set, Callable, Union, TypeVar, Generic
from uuid import UUID, uuid4
import json


class WorkflowState(Enum):
    """Workflow execution states"""
    PENDING = "pending"
    RUNNING = "running"
    PAUSED = "paused"
    SUSPENDED = "suspended"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"
    COMPENSATING = "compensating"
    COMPENSATED = "compensated"
    TIMED_OUT = "timed_out"


class StepState(Enum):
    """Individual workflow step states"""
    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    SKIPPED = "skipped"
    CANCELLED = "cancelled"
    COMPENSATING = "compensating"
    COMPENSATED = "compensated"
    TIMED_OUT = "timed_out"


class StepType(Enum):
    """Types of workflow steps"""
    MESSAGE = "message"
    PARALLEL = "parallel"
    SEQUENTIAL = "sequential"
    CONDITIONAL = "conditional"
    LOOP = "loop"
    SUBWORKFLOW = "subworkflow"
    WAIT = "wait"
    TRANSFORM = "transform"
    AGGREGATE = "aggregate"
    CUSTOM = "custom"


class CompensationStrategy(Enum):
    """Compensation strategies for failure handling"""
    NONE = "none"  # No compensation
    BACKWARD = "backward"  # Compensate in reverse order
    FORWARD = "forward"  # Compensate in forward order
    PARALLEL = "parallel"  # Compensate all at once
    CUSTOM = "custom"  # Custom compensation logic


class RetryStrategy(Enum):
    """Retry strategies for failed steps"""
    NONE = "none"  # No retry
    IMMEDIATE = "immediate"  # Retry immediately
    EXPONENTIAL = "exponential"  # Exponential backoff
    LINEAR = "linear"  # Linear backoff
    CUSTOM = "custom"  # Custom retry logic


@dataclass
class RetryPolicy:
    """Retry policy configuration"""
    strategy: RetryStrategy = RetryStrategy.EXPONENTIAL
    max_attempts: int = 3
    initial_delay: float = 1.0  # seconds
    max_delay: float = 60.0  # seconds
    backoff_factor: float = 2.0
    retry_on: List[type[Exception]] = field(default_factory=lambda: [Exception])

    def calculate_delay(self, attempt: int) -> float:
        """Calculate delay for next retry attempt"""
        if self.strategy == RetryStrategy.NONE:
            return 0
        elif self.strategy == RetryStrategy.IMMEDIATE:
            return 0
        elif self.strategy == RetryStrategy.LINEAR:
            return min(self.initial_delay * attempt, self.max_delay)
        elif self.strategy == RetryStrategy.EXPONENTIAL:
            return min(self.initial_delay * (self.backoff_factor ** (attempt - 1)), self.max_delay)
        else:
            return self.initial_delay

    def should_retry(self, error: Exception, attempt: int) -> bool:
        """Check if should retry based on error and attempt count"""
        if attempt >= self.max_attempts:
            return False

        # Check if error type is retryable
        return any(isinstance(error, retry_type) for retry_type in self.retry_on)


@dataclass
class WorkflowMetrics:
    """Metrics collected during workflow execution"""
    start_time: Optional[datetime] = None
    end_time: Optional[datetime] = None
    total_steps: int = 0
    completed_steps: int = 0
    failed_steps: int = 0
    skipped_steps: int = 0
    retry_count: int = 0
    compensation_count: int = 0

    @property
    def duration(self) -> Optional[timedelta]:
        if self.start_time and self.end_time:
            return self.end_time - self.start_time
        return None

    @property
    def success_rate(self) -> float:
        if self.total_steps == 0:
            return 0.0
        return self.completed_steps / self.total_steps

    def to_dict(self) -> Dict[str, Any]:
        return {
            "start_time": self.start_time.isoformat() if self.start_time else None,
            "end_time": self.end_time.isoformat() if self.end_time else None,
            "duration": self.duration.total_seconds() if self.duration else None,
            "total_steps": self.total_steps,
            "completed_steps": self.completed_steps,
            "failed_steps": self.failed_steps,
            "skipped_steps": self.skipped_steps,
            "retry_count": self.retry_count,
            "compensation_count": self.compensation_count,
            "success_rate": self.success_rate
        }


@dataclass
class WorkflowContext:
    """Workflow execution context"""
    workflow_id: UUID
    instance_id: UUID = field(default_factory=uuid4)
    parent_id: Optional[UUID] = None
    correlation_id: Optional[UUID] = None
    variables: Dict[str, Any] = field(default_factory=dict)
    results: Dict[str, Any] = field(default_factory=dict)
    errors: List[Dict[str, Any]] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)
    checkpoints: Dict[str, Any] = field(default_factory=dict)
    created_at: datetime = field(default_factory=datetime.utcnow)
    updated_at: datetime = field(default_factory=datetime.utcnow)

    def get(self, key: str, default: Any = None) -> Any:
        """Get variable with dot notation support"""
        keys = key.split('.')
        value = self.variables

        for k in keys:
            if isinstance(value, dict) and k in value:
                value = value[k]
            else:
                return default

        return value

    def set(self, key: str, value: Any) -> None:
        """Set variable with dot notation support"""
        keys = key.split('.')
        target = self.variables

        for k in keys[:-1]:
            if k not in target:
                target[k] = {}
            target = target[k]

        target[keys[-1]] = value
        self.updated_at = datetime.utcnow()

    def add_result(self, step_id: str, result: Any) -> None:
        """Add step execution result"""
        self.results[step_id] = {
            "value": result,
            "timestamp": datetime.utcnow().isoformat()
        }
        self.updated_at = datetime.utcnow()

    def add_error(self, step_id: str, error: Exception, details: Dict[str, Any] = None) -> None:
        """Record step execution error"""
        error_entry = {
            "step_id": step_id,
            "error_type": type(error).__name__,
            "error_message": str(error),
            "timestamp": datetime.utcnow().isoformat(),
            "details": details or {}
        }

        # Add traceback if available
        import traceback
        if hasattr(error, '__traceback__'):
            error_entry["traceback"] = traceback.format_tb(error.__traceback__)

        self.errors.append(error_entry)
        self.updated_at = datetime.utcnow()

    def create_checkpoint(self, checkpoint_id: str) -> None:
        """Create a checkpoint of current state"""
        self.checkpoints[checkpoint_id] = {
            "variables": self.variables.copy(),
            "results": self.results.copy(),
            "timestamp": datetime.utcnow().isoformat()
        }

    def restore_checkpoint(self, checkpoint_id: str) -> bool:
        """Restore state from checkpoint"""
        if checkpoint_id in self.checkpoints:
            checkpoint = self.checkpoints[checkpoint_id]
            self.variables = checkpoint["variables"].copy()
            self.results = checkpoint["results"].copy()
            self.updated_at = datetime.utcnow()
            return True
        return False

    def merge(self, other: 'WorkflowContext') -> None:
        """Merge another context into this one"""
        self.variables.update(other.variables)
        self.results.update(other.results)
        self.errors.extend(other.errors)
        self.metadata.update(other.metadata)
        self.updated_at = datetime.utcnow()

    def to_dict(self) -> Dict[str, Any]:
        """Serialize context to dictionary"""
        return {
            "workflow_id": str(self.workflow_id),
            "instance_id": str(self.instance_id),
            "parent_id": str(self.parent_id) if self.parent_id else None,
            "correlation_id": str(self.correlation_id) if self.correlation_id else None,
            "variables": self.variables,
            "results": self.results,
            "errors": self.errors,
            "metadata": self.metadata,
            "checkpoints": self.checkpoints,
            "created_at": self.created_at.isoformat(),
            "updated_at": self.updated_at.isoformat()
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'WorkflowContext':
        """Deserialize context from dictionary"""
        return cls(
            workflow_id=UUID(data["workflow_id"]),
            instance_id=UUID(data["instance_id"]),
            parent_id=UUID(data["parent_id"]) if data.get("parent_id") else None,
            correlation_id=UUID(data["correlation_id"]) if data.get("correlation_id") else None,
            variables=data.get("variables", {}),
            results=data.get("results", {}),
            errors=data.get("errors", []),
            metadata=data.get("metadata", {}),
            checkpoints=data.get("checkpoints", {}),
            created_at=datetime.fromisoformat(data["created_at"]),
            updated_at=datetime.fromisoformat(data["updated_at"])
        )


T = TypeVar('T')


@dataclass
class StepResult(Generic[T]):
    """Result of a step execution"""
    step_id: str
    state: StepState
    value: Optional[T] = None
    error: Optional[Exception] = None
    start_time: Optional[datetime] = None
    end_time: Optional[datetime] = None
    retry_count: int = 0
    metadata: Dict[str, Any] = field(default_factory=dict)

    @property
    def duration(self) -> Optional[timedelta]:
        if self.start_time and self.end_time:
            return self.end_time - self.start_time
        return None

    @property
    def is_success(self) -> bool:
        return self.state == StepState.COMPLETED

    @property
    def is_failure(self) -> bool:
        return self.state in [StepState.FAILED, StepState.TIMED_OUT]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "step_id": self.step_id,
            "state": self.state.value,
            "value": self.value,
            "error": str(self.error) if self.error else None,
            "start_time": self.start_time.isoformat() if self.start_time else None,
            "end_time": self.end_time.isoformat() if self.end_time else None,
            "duration": self.duration.total_seconds() if self.duration else None,
            "retry_count": self.retry_count,
            "metadata": self.metadata
        }


class WorkflowStep(ABC):
    """Abstract base class for workflow steps"""

    def __init__(self,
                 step_id: str,
                 name: Optional[str] = None,
                 description: Optional[str] = None,
                 timeout: Optional[timedelta] = None,
                 retry_policy: Optional[RetryPolicy] = None,
                 compensation_handler: Optional[Callable] = None,
                 on_success: Optional[Callable] = None,
                 on_failure: Optional[Callable] = None,
                 metadata: Optional[Dict[str, Any]] = None):
        self.step_id = step_id
        self.name = name or step_id
        self.description = description
        self.timeout = timeout or timedelta(minutes=5)
        self.retry_policy = retry_policy or RetryPolicy(strategy=RetryStrategy.NONE)
        self.compensation_handler = compensation_handler
        self.on_success = on_success
        self.on_failure = on_failure
        self.metadata = metadata or {}

        # Runtime state
        self.state = StepState.PENDING
        self.result: Optional[StepResult] = None
        self._lock = asyncio.Lock()

    @abstractmethod
    def get_type(self) -> StepType:
        """Get the step type"""
        pass

    @abstractmethod
    async def execute(self, context: WorkflowContext) -> Any:
        """Execute the workflow step"""
        pass

    @abstractmethod
    async def compensate(self, context: WorkflowContext) -> None:
        """Compensate/rollback the step if needed"""
        pass

    async def validate(self, context: WorkflowContext) -> Optional[str]:
        """Validate step configuration and context"""
        return None

    def get_required_permissions(self) -> Set[str]:
        """Get required permissions for this step"""
        return set()

    def get_estimated_duration(self) -> timedelta:
        """Get estimated execution duration"""
        return timedelta(seconds=30)

    def to_dict(self) -> Dict[str, Any]:
        """Serialize step to dictionary"""
        return {
            "step_id": self.step_id,
            "name": self.name,
            "type": self.get_type().value,
            "description": self.description,
            "timeout": self.timeout.total_seconds(),
            "retry_policy": {
                "strategy": self.retry_policy.strategy.value,
                "max_attempts": self.retry_policy.max_attempts
            },
            "metadata": self.metadata,
            "state": self.state.value,
            "result": self.result.to_dict() if self.result else None
        }


@dataclass
class WorkflowDefinition:
    """Defines a workflow structure"""
    workflow_id: str
    name: str
    version: str = "1.0"
    description: str = ""
    steps: List[WorkflowStep] = field(default_factory=list)
    timeout: timedelta = timedelta(hours=1)
    retry_policy: RetryPolicy = field(default_factory=RetryPolicy)
    compensation_strategy: CompensationStrategy = CompensationStrategy.BACKWARD
    variables_schema: Optional[Dict[str, Any]] = None
    required_permissions: Set[str] = field(default_factory=set)
    tags: List[str] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)

    def add_step(self, step: WorkflowStep) -> 'WorkflowDefinition':
        """Add step to workflow"""
        self.steps.append(step)
        return self

    def validate(self) -> List[str]:
        """Validate workflow definition"""
        errors = []

        # Check for duplicate step IDs
        step_ids = [step.step_id for step in self.steps]
        if len(step_ids) != len(set(step_ids)):
            errors.append("Duplicate step IDs found")

        # Check for empty workflow
        if not self.steps:
            errors.append("Workflow has no steps")

        # Validate each step
        for step in self.steps:
            if not step.step_id:
                errors.append("Step missing ID")

        return errors

    def get_step(self, step_id: str) -> Optional[WorkflowStep]:
        """Get step by ID"""
        for step in self.steps:
            if step.step_id == step_id:
                return step
        return None

    def to_dict(self) -> Dict[str, Any]:
        """Serialize workflow definition"""
        return {
            "workflow_id": self.workflow_id,
            "name": self.name,
            "version": self.version,
            "description": self.description,
            "steps": [step.to_dict() for step in self.steps],
            "timeout": self.timeout.total_seconds(),
            "retry_policy": {
                "strategy": self.retry_policy.strategy.value,
                "max_attempts": self.retry_policy.max_attempts
            },
            "compensation_strategy": self.compensation_strategy.value,
            "variables_schema": self.variables_schema,
            "required_permissions": list(self.required_permissions),
            "tags": self.tags,
            "metadata": self.metadata
        }