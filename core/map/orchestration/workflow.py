# File: maple/core/map/orchestration/workflow.py
# Description: Workflow orchestration engine for MAP protocol that manages
# complex multi-agent workflows, including parallel execution, conditional logic,
# error handling, and state management with compensation/rollback capabilities.

from __future__ import annotations
import asyncio
import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum
from typing import Dict, List, Optional, Any, Set, Callable, Union
from uuid import UUID, uuid4
import json
import pickle
from collections import defaultdict

from maple.core.map.models.message import MAPMessage, MessageType, MessagePayload, MessageHeader, MessageDestination
from maple.core.map.routing.engine import RoutingEngine
from maple.core.map.transport.base import TransportManager, DeliveryReceipt

logger = logging.getLogger(__name__)


class WorkflowState(Enum):
    """Workflow execution states"""
    PENDING = "pending"
    RUNNING = "running"
    SUSPENDED = "suspended"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"
    COMPENSATING = "compensating"
    COMPENSATED = "compensated"


class StepState(Enum):
    """Individual workflow step states"""
    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    SKIPPED = "skipped"
    COMPENSATING = "compensating"
    COMPENSATED = "compensated"


@dataclass
class WorkflowContext:
    """Workflow execution context"""
    workflow_id: UUID
    parent_id: Optional[UUID] = None
    variables: Dict[str, Any] = field(default_factory=dict)
    results: Dict[str, Any] = field(default_factory=dict)
    errors: List[Dict[str, Any]] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)
    created_at: datetime = field(default_factory=datetime.utcnow)
    updated_at: datetime = field(default_factory=datetime.utcnow)

    def get_variable(self, name: str, default: Any = None) -> Any:
        """Get workflow variable with optional default"""
        return self.variables.get(name, default)

    def set_variable(self, name: str, value: Any) -> None:
        """Set workflow variable"""
        self.variables[name] = value
        self.updated_at = datetime.utcnow()

    def add_result(self, step_id: str, result: Any) -> None:
        """Add step execution result"""
        self.results[step_id] = result
        self.updated_at = datetime.utcnow()

    def add_error(self, step_id: str, error: Exception) -> None:
        """Record step execution error"""
        self.errors.append({
            "step_id": step_id,
            "error_type": type(error).__name__,
            "error_message": str(error),
            "timestamp": datetime.utcnow().isoformat()
        })
        self.updated_at = datetime.utcnow()


class WorkflowStep(ABC):
    """Abstract base class for workflow steps"""

    def __init__(self, step_id: str, name: str = None):
        self.step_id = step_id
        self.name = name or step_id
        self.state = StepState.PENDING
        self.started_at: Optional[datetime] = None
        self.completed_at: Optional[datetime] = None
        self.error: Optional[Exception] = None
        self.retry_count = 0
        self.max_retries = 3
        self.timeout = timedelta(minutes=5)

    @abstractmethod
    async def execute(self, context: WorkflowContext) -> Any:
        """Execute the workflow step"""
        pass

    @abstractmethod
    async def compensate(self, context: WorkflowContext) -> None:
        """Compensate/rollback the step if needed"""
        pass

    async def should_retry(self, error: Exception) -> bool:
        """Determine if step should be retried after error"""
        return self.retry_count < self.max_retries and isinstance(error, (TimeoutError, ConnectionError))


class MessageStep(WorkflowStep):
    """Step that sends a message to an agent"""

    def __init__(self,
                 step_id: str,
                 destination: Union[str, MessageDestination],
                 action: str,
                 data_template: Dict[str, Any] = None,
                 timeout: timedelta = timedelta(minutes=2)):
        super().__init__(step_id)
        self.destination = destination if isinstance(destination, MessageDestination) else MessageDestination(
            agent_id=destination)
        self.action = action
        self.data_template = data_template or {}
        self.timeout = timeout
        self.response: Optional[MAPMessage] = None

    async def execute(self, context: WorkflowContext) -> Any:
        """Send message and wait for response"""
        # Substitute variables in data template
        data = self._substitute_variables(self.data_template, context)

        # Create message
        message = MAPMessage(
            header=MessageHeader(
                destination=self.destination,
                correlation_id=context.workflow_id
            ),
            payload=MessagePayload(
                type=MessageType.REQUEST,
                action=self.action,
                data=data,
                metadata={"workflow_id": str(context.workflow_id), "step_id": self.step_id}
            )
        )

        # This would integrate with actual message sending
        # For now, returning mock response
        logger.info(f"Executing message step {self.step_id}: {self.action}")

        # Simulate async execution
        await asyncio.sleep(0.1)

        return {"status": "success", "data": data}

    async def compensate(self, context: WorkflowContext) -> None:
        """Send compensation message if needed"""
        if self.response:
            # Send compensation message
            comp_message = MAPMessage(
                header=MessageHeader(
                    destination=self.destination,
                    correlation_id=context.workflow_id
                ),
                payload=MessagePayload(
                    type=MessageType.COMMAND,
                    action=f"{self.action}_compensate",
                    data={"original_request": self.data_template},
                    metadata={"workflow_id": str(context.workflow_id), "step_id": self.step_id}
                )
            )
            logger.info(f"Compensating step {self.step_id}")

    def _substitute_variables(self, template: Dict[str, Any], context: WorkflowContext) -> Dict[str, Any]:
        """Substitute workflow variables in template"""
        result = {}
        for key, value in template.items():
            if isinstance(value, str) and value.startswith("${") and value.endswith("}"):
                # Variable reference
                var_name = value[2:-1]
                result[key] = context.get_variable(var_name, value)
            elif isinstance(value, dict):
                result[key] = self._substitute_variables(value, context)
            else:
                result[key] = value
        return result


class ParallelStep(WorkflowStep):
    """Step that executes multiple sub-steps in parallel"""

    def __init__(self, step_id: str, steps: List[WorkflowStep], wait_all: bool = True):
        super().__init__(step_id)
        self.steps = steps
        self.wait_all = wait_all  # Wait for all or any to complete

    async def execute(self, context: WorkflowContext) -> Any:
        """Execute all sub-steps in parallel"""
        tasks = []
        for step in self.steps:
            task = asyncio.create_task(self._execute_step(step, context))
            tasks.append((step.step_id, task))

        if self.wait_all:
            # Wait for all tasks
            results = {}
            for step_id, task in tasks:
                try:
                    result = await task
                    results[step_id] = result
                except Exception as e:
                    results[step_id] = {"error": str(e)}
                    if not self.wait_all:
                        # Cancel remaining tasks
                        for _, t in tasks:
                            if not t.done():
                                t.cancel()
                        raise
            return results
        else:
            # Wait for first to complete
            done, pending = await asyncio.wait(
                [task for _, task in tasks],
                return_when=asyncio.FIRST_COMPLETED
            )

            # Cancel pending tasks
            for task in pending:
                task.cancel()

            # Return first result
            return await done.pop()

    async def _execute_step(self, step: WorkflowStep, context: WorkflowContext) -> Any:
        """Execute a single step with error handling"""
        try:
            step.state = StepState.RUNNING
            step.started_at = datetime.utcnow()

            result = await step.execute(context)

            step.state = StepState.COMPLETED
            step.completed_at = datetime.utcnow()
            context.add_result(step.step_id, result)

            return result
        except Exception as e:
            step.state = StepState.FAILED
            step.error = e
            context.add_error(step.step_id, e)
            raise

    async def compensate(self, context: WorkflowContext) -> None:
        """Compensate all completed sub-steps"""
        compensation_tasks = []
        for step in self.steps:
            if step.state == StepState.COMPLETED:
                task = asyncio.create_task(step.compensate(context))
                compensation_tasks.append(task)

        if compensation_tasks:
            await asyncio.gather(*compensation_tasks, return_exceptions=True)


class ConditionalStep(WorkflowStep):
    """Step that executes based on condition"""

    def __init__(self,
                 step_id: str,
                 condition: Callable[[WorkflowContext], bool],
                 if_true: WorkflowStep,
                 if_false: Optional[WorkflowStep] = None):
        super().__init__(step_id)
        self.condition = condition
        self.if_true = if_true
        self.if_false = if_false
        self.executed_branch: Optional[WorkflowStep] = None

    async def execute(self, context: WorkflowContext) -> Any:
        """Evaluate condition and execute appropriate branch"""
        if self.condition(context):
            self.executed_branch = self.if_true
            return await self.if_true.execute(context)
        elif self.if_false:
            self.executed_branch = self.if_false
            return await self.if_false.execute(context)
        else:
            # No else branch, skip
            self.state = StepState.SKIPPED
            return None

    async def compensate(self, context: WorkflowContext) -> None:
        """Compensate the executed branch"""
        if self.executed_branch:
            await self.executed_branch.compensate(context)


class LoopStep(WorkflowStep):
    """Step that executes in a loop"""

    def __init__(self,
                 step_id: str,
                 items_provider: Callable[[WorkflowContext], List[Any]],
                 step_template: Callable[[Any], WorkflowStep],
                 max_concurrent: int = 5):
        super().__init__(step_id)
        self.items_provider = items_provider
        self.step_template = step_template
        self.max_concurrent = max_concurrent
        self.executed_steps: List[WorkflowStep] = []

    async def execute(self, context: WorkflowContext) -> Any:
        """Execute step for each item with concurrency control"""
        items = self.items_provider(context)
        results = []

        # Process items with limited concurrency
        semaphore = asyncio.Semaphore(self.max_concurrent)

        async def process_item(item: Any, index: int):
            async with semaphore:
                step = self.step_template(item)
                step.step_id = f"{self.step_id}[{index}]"
                self.executed_steps.append(step)

                result = await step.execute(context)
                return result

        tasks = [process_item(item, i) for i, item in enumerate(items)]
        results = await asyncio.gather(*tasks, return_exceptions=True)

        return results

    async def compensate(self, context: WorkflowContext) -> None:
        """Compensate all executed iterations"""
        compensation_tasks = [
            step.compensate(context)
            for step in self.executed_steps
            if step.state == StepState.COMPLETED
        ]

        if compensation_tasks:
            await asyncio.gather(*compensation_tasks, return_exceptions=True)


@dataclass
class WorkflowDefinition:
    """Defines a workflow structure"""
    workflow_id: str
    name: str
    description: str = ""
    steps: List[WorkflowStep] = field(default_factory=list)
    timeout: timedelta = timedelta(hours=1)
    retry_policy: Dict[str, Any] = field(default_factory=dict)
    compensation_enabled: bool = True

    def add_step(self, step: WorkflowStep) -> None:
        """Add step to workflow"""
        self.steps.append(step)

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

        return errors


class WorkflowInstance:
    """Runtime instance of a workflow"""

    def __init__(self,
                 definition: WorkflowDefinition,
                 context: Optional[WorkflowContext] = None):
        self.instance_id = uuid4()
        self.definition = definition
        self.context = context or WorkflowContext(workflow_id=self.instance_id)
        self.state = WorkflowState.PENDING
        self.current_step_index = 0
        self.started_at: Optional[datetime] = None
        self.completed_at: Optional[datetime] = None
        self.error: Optional[Exception] = None
        self._state_lock = asyncio.Lock()

    async def execute(self) -> Any:
        """Execute the workflow"""
        async with self._state_lock:
            if self.state != WorkflowState.PENDING:
                raise RuntimeError(f"Cannot execute workflow in state {self.state}")

            self.state = WorkflowState.RUNNING
            self.started_at = datetime.utcnow()

        try:
            # Execute steps sequentially
            for i, step in enumerate(self.definition.steps):
                self.current_step_index = i

                # Check if workflow should continue
                if self.state != WorkflowState.RUNNING:
                    break

                # Execute step with timeout
                try:
                    result = await asyncio.wait_for(
                        self._execute_step(step),
                        timeout=step.timeout.total_seconds()
                    )

                    self.context.add_result(step.step_id, result)

                except asyncio.TimeoutError:
                    raise TimeoutError(f"Step {step.step_id} timed out")

            # Workflow completed successfully
            async with self._state_lock:
                self.state = WorkflowState.COMPLETED
                self.completed_at = datetime.utcnow()

            return self.context.results

        except Exception as e:
            self.error = e
            async with self._state_lock:
                self.state = WorkflowState.FAILED

            # Trigger compensation if enabled
            if self.definition.compensation_enabled:
                await self._compensate()

            raise

    async def _execute_step(self, step: WorkflowStep) -> Any:
        """Execute a single step with retry logic"""
        attempt = 0
        last_error = None

        while attempt <= step.max_retries:
            try:
                step.state = StepState.RUNNING
                step.started_at = datetime.utcnow()

                result = await step.execute(self.context)

                step.state = StepState.COMPLETED
                step.completed_at = datetime.utcnow()

                return result

            except Exception as e:
                last_error = e
                step.error = e
                step.retry_count = attempt

                if await step.should_retry(e) and attempt < step.max_retries:
                    attempt += 1
                    await asyncio.sleep(2 ** attempt)  # Exponential backoff
                    logger.warning(f"Retrying step {step.step_id}, attempt {attempt}")
                else:
                    step.state = StepState.FAILED
                    self.context.add_error(step.step_id, e)
                    raise

        if last_error:
            raise last_error

    async def _compensate(self) -> None:
        """Execute compensation for completed steps in reverse order"""
        async with self._state_lock:
            self.state = WorkflowState.COMPENSATING

        try:
            # Compensate in reverse order
            for i in range(self.current_step_index, -1, -1):
                step = self.definition.steps[i]

                if step.state == StepState.COMPLETED:
                    try:
                        step.state = StepState.COMPENSATING
                        await step.compensate(self.context)
                        step.state = StepState.COMPENSATED
                    except Exception as e:
                        logger.error(f"Compensation failed for step {step.step_id}: {str(e)}")
                        self.context.add_error(f"{step.step_id}_compensation", e)

            async with self._state_lock:
                self.state = WorkflowState.COMPENSATED

        except Exception as e:
            logger.error(f"Compensation process failed: {str(e)}")
            async with self._state_lock:
                self.state = WorkflowState.FAILED

    async def cancel(self) -> None:
        """Cancel workflow execution"""
        async with self._state_lock:
            if self.state not in [WorkflowState.RUNNING, WorkflowState.SUSPENDED]:
                raise RuntimeError(f"Cannot cancel workflow in state {self.state}")

            self.state = WorkflowState.CANCELLED

        # Trigger compensation for completed steps
        if self.definition.compensation_enabled:
            await self._compensate()

    async def suspend(self) -> None:
        """Suspend workflow execution"""
        async with self._state_lock:
            if self.state != WorkflowState.RUNNING:
                raise RuntimeError(f"Cannot suspend workflow in state {self.state}")

            self.state = WorkflowState.SUSPENDED

    async def resume(self) -> Any:
        """Resume suspended workflow"""
        async with self._state_lock:
            if self.state != WorkflowState.SUSPENDED:
                raise RuntimeError(f"Cannot resume workflow in state {self.state}")

            self.state = WorkflowState.RUNNING

        # Continue execution from current step
        return await self.execute()

    def to_dict(self) -> Dict[str, Any]:
        """Serialize workflow instance state"""
        return {
            "instance_id": str(self.instance_id),
            "workflow_id": self.definition.workflow_id,
            "state": self.state.value,
            "current_step": self.current_step_index,
            "context": {
                "variables": self.context.variables,
                "results": self.context.results,
                "errors": self.context.errors
            },
            "started_at": self.started_at.isoformat() if self.started_at else None,
            "completed_at": self.completed_at.isoformat() if self.completed_at else None,
            "error": str(self.error) if self.error else None
        }


class WorkflowEngine:
    """Main workflow orchestration engine"""

    def __init__(self,
                 routing_engine: RoutingEngine,
                 transport_manager: TransportManager):
        self.routing_engine = routing_engine
        self.transport_manager = transport_manager
        self.definitions: Dict[str, WorkflowDefinition] = {}
        self.instances: Dict[UUID, WorkflowInstance] = {}
        self.pending_responses: Dict[UUID, asyncio.Future] = {}
        self._persistence_enabled = True
        self._cleanup_task: Optional[asyncio.Task] = None

    def register_workflow(self, definition: WorkflowDefinition) -> None:
        """Register a workflow definition"""
        errors = definition.validate()
        if errors:
            raise ValueError(f"Invalid workflow definition: {', '.join(errors)}")

        self.definitions[definition.workflow_id] = definition
        logger.info(f"Registered workflow: {definition.name}")

    async def start_workflow(self,
                             workflow_id: str,
                             input_data: Dict[str, Any] = None) -> UUID:
        """Start a new workflow instance"""
        if workflow_id not in self.definitions:
            raise ValueError(f"Unknown workflow: {workflow_id}")

        definition = self.definitions[workflow_id]
        context = WorkflowContext(workflow_id=uuid4())

        # Set input variables
        if input_data:
            for key, value in input_data.items():
                context.set_variable(key, value)

        instance = WorkflowInstance(definition, context)
        self.instances[instance.instance_id] = instance

        # Start execution asynchronously
        asyncio.create_task(self._execute_workflow(instance))

        logger.info(f"Started workflow instance: {instance.instance_id}")
        return instance.instance_id

    async def _execute_workflow(self, instance: WorkflowInstance) -> None:
        """Execute workflow instance with error handling"""
        try:
            await instance.execute()

            # Persist final state
            if self._persistence_enabled:
                await self._persist_instance(instance)

        except Exception as e:
            logger.error(f"Workflow {instance.instance_id} failed: {str(e)}")

            # Persist error state
            if self._persistence_enabled:
                await self._persist_instance(instance)

    async def get_workflow_status(self, instance_id: UUID) -> Dict[str, Any]:
        """Get workflow instance status"""
        if instance_id not in self.instances:
            # Try to load from persistence
            instance = await self._load_instance(instance_id)
            if not instance:
                raise ValueError(f"Unknown workflow instance: {instance_id}")
        else:
            instance = self.instances[instance_id]

        return instance.to_dict()

    async def cancel_workflow(self, instance_id: UUID) -> None:
        """Cancel a running workflow"""
        if instance_id not in self.instances:
            raise ValueError(f"Unknown workflow instance: {instance_id}")

        instance = self.instances[instance_id]
        await instance.cancel()

    async def handle_message_response(self, message: MAPMessage) -> None:
        """Handle response messages for workflow steps"""
        correlation_id = message.header.correlation_id

        if correlation_id and correlation_id in self.pending_responses:
            future = self.pending_responses[correlation_id]
            future.set_result(message)
            del self.pending_responses[correlation_id]

    async def _persist_instance(self, instance: WorkflowInstance) -> None:
        """Persist workflow instance state"""
        # This would save to a database
        # For now, just log
        logger.debug(f"Persisting workflow instance {instance.instance_id}")

    async def _load_instance(self, instance_id: UUID) -> Optional[WorkflowInstance]:
        """Load workflow instance from persistence"""
        # This would load from a database
        # For now, return None
        return None

    async def cleanup_completed_instances(self, retention_period: timedelta) -> None:
        """Remove completed workflow instances older than retention period"""
        cutoff_time = datetime.utcnow() - retention_period
        instances_to_remove = []

        for instance_id, instance in self.instances.items():
            if instance.state in [WorkflowState.COMPLETED, WorkflowState.FAILED, WorkflowState.COMPENSATED]:
                if instance.completed_at and instance.completed_at < cutoff_time:
                    instances_to_remove.append(instance_id)

        for instance_id in instances_to_remove:
            del self.instances[instance_id]
            logger.info(f"Cleaned up workflow instance: {instance_id}")

    async def start_cleanup_task(self, interval: timedelta = timedelta(hours=1)) -> None:
        """Start periodic cleanup task"""

        async def cleanup_loop():
            while True:
                await asyncio.sleep(interval.total_seconds())
                await self.cleanup_completed_instances(timedelta(days=7))

        self._cleanup_task = asyncio.create_task(cleanup_loop())

    async def shutdown(self) -> None:
        """Shutdown workflow engine"""
        if self._cleanup_task:
            self._cleanup_task.cancel()

        # Cancel all running workflows
        for instance in self.instances.values():
            if instance.state == WorkflowState.RUNNING:
                await instance.cancel()


# Workflow DSL Builder for easier workflow creation
class WorkflowBuilder:
    """Fluent API for building workflows"""

    def __init__(self, workflow_id: str, name: str):
        self.definition = WorkflowDefinition(workflow_id=workflow_id, name=name)
        self._current_steps: List[WorkflowStep] = []

    def with_description(self, description: str) -> 'WorkflowBuilder':
        """Set workflow description"""
        self.definition.description = description
        return self

    def with_timeout(self, timeout: timedelta) -> 'WorkflowBuilder':
        """Set workflow timeout"""
        self.definition.timeout = timeout
        return self

    def with_compensation(self, enabled: bool = True) -> 'WorkflowBuilder':
        """Enable/disable compensation"""
        self.definition.compensation_enabled = enabled
        return self

    def send_message(self,
                     step_id: str,
                     destination: str,
                     action: str,
                     data: Dict[str, Any] = None) -> 'WorkflowBuilder':
        """Add message step"""
        step = MessageStep(step_id, destination, action, data)
        self.definition.add_step(step)
        return self

    def parallel(self, *builders: 'WorkflowBuilder') -> 'WorkflowBuilder':
        """Add parallel execution block"""
        steps = []
        for builder in builders:
            steps.extend(builder.definition.steps)

        parallel_step = ParallelStep(f"parallel_{len(self.definition.steps)}", steps)
        self.definition.add_step(parallel_step)
        return self

    def if_condition(self,
                     condition: Callable[[WorkflowContext], bool],
                     if_true: 'WorkflowBuilder',
                     if_false: Optional['WorkflowBuilder'] = None) -> 'WorkflowBuilder':
        """Add conditional execution"""
        true_steps = if_true.definition.steps[0] if if_true.definition.steps else None
        false_steps = if_false.definition.steps[0] if if_false and if_false.definition.steps else None

        conditional = ConditionalStep(
            f"conditional_{len(self.definition.steps)}",
            condition,
            true_steps,
            false_steps
        )
        self.definition.add_step(conditional)
        return self

    def for_each(self,
                 items_provider: Callable[[WorkflowContext], List[Any]],
                 step_builder: Callable[[Any], WorkflowStep],
                 max_concurrent: int = 5) -> 'WorkflowBuilder':
        """Add loop execution"""
        loop = LoopStep(
            f"loop_{len(self.definition.steps)}",
            items_provider,
            step_builder,
            max_concurrent
        )
        self.definition.add_step(loop)
        return self

    def build(self) -> WorkflowDefinition:
        """Build the workflow definition"""
        return self.definition


# Example workflow creation
def create_data_processing_workflow() -> WorkflowDefinition:
    """Example: Create a data processing workflow"""

    return (WorkflowBuilder("data_processing", "Data Processing Workflow")
            .with_description("Process data through multiple agents")
            .with_timeout(timedelta(minutes=30))
            .with_compensation(True)

            # Step 1: Validate input
            .send_message("validate", "validator_agent", "validate_data", {
                "data": "${input_data}",
                "schema": "${validation_schema}"
            })

            # Step 2: Process in parallel
            .parallel(
                WorkflowBuilder("", "")
                .send_message("transform", "transformer_agent", "transform_data", {
                    "data": "${input_data}",
                    "rules": "${transformation_rules}"
                }),

                WorkflowBuilder("", "")
                .send_message("analyze", "analyzer_agent", "analyze_data", {
                    "data": "${input_data}",
                    "metrics": ["mean", "median", "std"]
                })
            )

            # Step 3: Conditional processing
            .if_condition(
                lambda ctx: ctx.get_variable("data_quality_score", 0) > 0.8,
                WorkflowBuilder("", "")
                .send_message("ml_process", "ml_agent", "train_model", {
                    "data": "${transformed_data}"
                }),
                WorkflowBuilder("", "")
                .send_message("manual_review", "review_agent", "flag_for_review", {
                    "data": "${input_data}",
                    "reason": "Low quality score"
                })
            )

            # Step 4: Store results
            .send_message("store", "storage_agent", "save_results", {
                "results": "${processing_results}",
                "metadata": "${processing_metadata}"
            })

            .build()
    )
