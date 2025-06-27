# File: core/map/orchestration/engine.py
# Description: Core workflow execution engine that manages workflow instances,
# handles execution flow, state management, and compensation logic.

from __future__ import annotations
import asyncio
import logging
from collections import defaultdict
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Set, Any, Callable, Tuple
from uuid import UUID, uuid4
import json
from enum import Enum

from core.map.orchestration.models import (
    WorkflowDefinition, WorkflowContext, WorkflowState, StepState,
    WorkflowMetrics, CompensationStrategy, RetryPolicy, StepResult
)
from core.map.orchestration.persistence import WorkflowPersistence
from core.map.routing.engine import RoutingEngine
from core.map.transport.base import TransportManager

logger = logging.getLogger(__name__)


class ExecutionMode(Enum):
    """Workflow execution modes"""
    NORMAL = "normal"  # Execute all steps
    DEBUG = "debug"  # Execute with detailed logging
    DRY_RUN = "dry_run"  # Validate without execution
    REPLAY = "replay"  # Replay from persistence


class WorkflowEvent:
    """Base class for workflow events"""

    def __init__(self,
                 instance_id: UUID,
                 event_type: str,
                 timestamp: datetime = None,
                 data: Dict[str, Any] = None):
        self.instance_id = instance_id
        self.event_type = event_type
        self.timestamp = timestamp or datetime.utcnow()
        self.data = data or {}


class WorkflowInstance:
    """Runtime instance of a workflow"""

    def __init__(self,
                 definition: WorkflowDefinition,
                 context: Optional[WorkflowContext] = None,
                 execution_mode: ExecutionMode = ExecutionMode.NORMAL):
        self.instance_id = context.instance_id if context else uuid4()
        self.definition = definition
        self.context = context or WorkflowContext(
            workflow_id=definition.workflow_id,
            instance_id=self.instance_id
        )
        self.execution_mode = execution_mode
        self.state = WorkflowState.PENDING
        self.metrics = WorkflowMetrics()

        # Execution state
        self.current_step_index = 0
        self.step_results: Dict[str, StepResult] = {}
        self.compensation_stack: List[str] = []

        # Event handlers
        self.event_handlers: Dict[str, List[Callable]] = defaultdict(list)

        # Async control
        self._execution_task: Optional[asyncio.Task] = None
        self._pause_event = asyncio.Event()
        self._pause_event.set()  # Not paused by default
        self._cancel_event = asyncio.Event()
        self._state_lock = asyncio.Lock()

    async def start(self) -> UUID:
        """Start workflow execution"""
        async with self._state_lock:
            if self.state != WorkflowState.PENDING:
                raise RuntimeError(f"Cannot start workflow in state {self.state}")

            self.state = WorkflowState.RUNNING
            self.metrics.start_time = datetime.utcnow()
            self.metrics.total_steps = len(self.definition.steps)

        # Emit start event
        await self._emit_event("workflow_started", {
            "workflow_id": self.definition.workflow_id,
            "instance_id": str(self.instance_id)
        })

        # Start execution in background
        self._execution_task = asyncio.create_task(self._execute())

        return self.instance_id

    async def wait_for_completion(self, timeout: Optional[timedelta] = None) -> Dict[str, Any]:
        """Wait for workflow to complete"""
        if not self._execution_task:
            raise RuntimeError("Workflow not started")

        try:
            if timeout:
                await asyncio.wait_for(self._execution_task, timeout.total_seconds())
            else:
                await self._execution_task
        except asyncio.TimeoutError:
            await self.cancel()
            raise TimeoutError("Workflow execution timed out")

        return self.get_results()

    async def pause(self) -> None:
        """Pause workflow execution"""
        async with self._state_lock:
            if self.state != WorkflowState.RUNNING:
                raise RuntimeError(f"Cannot pause workflow in state {self.state}")

            self.state = WorkflowState.PAUSED
            self._pause_event.clear()

        await self._emit_event("workflow_paused", {
            "current_step": self.current_step_index
        })

    async def resume(self) -> None:
        """Resume paused workflow"""
        async with self._state_lock:
            if self.state != WorkflowState.PAUSED:
                raise RuntimeError(f"Cannot resume workflow in state {self.state}")

            self.state = WorkflowState.RUNNING
            self._pause_event.set()

        await self._emit_event("workflow_resumed", {
            "current_step": self.current_step_index
        })

    async def cancel(self) -> None:
        """Cancel workflow execution"""
        async with self._state_lock:
            if self.state not in [WorkflowState.RUNNING, WorkflowState.PAUSED]:
                return

            self.state = WorkflowState.CANCELLED
            self._cancel_event.set()
            self._pause_event.set()  # Unpause if paused

        # Cancel execution task
        if self._execution_task and not self._execution_task.done():
            self._execution_task.cancel()
            try:
                await self._execution_task
            except asyncio.CancelledError:
                pass

        # Trigger compensation if enabled
        if self.definition.compensation_strategy != CompensationStrategy.NONE:
            await self._compensate()

        await self._emit_event("workflow_cancelled", {
            "reason": "user_requested"
        })

    async def _execute(self) -> None:
        """Main workflow execution loop"""
        try:
            # Validate workflow
            if self.execution_mode == ExecutionMode.DRY_RUN:
                await self._validate_workflow()
                self.state = WorkflowState.COMPLETED
                return

            # Execute steps
            for i, step in enumerate(self.definition.steps):
                self.current_step_index = i

                # Check for pause
                await self._pause_event.wait()

                # Check for cancellation
                if self._cancel_event.is_set():
                    raise asyncio.CancelledError()

                # Execute step
                await self._execute_step(step)

                # Check if workflow should continue
                if self.state != WorkflowState.RUNNING:
                    break

            # Workflow completed successfully
            async with self._state_lock:
                self.state = WorkflowState.COMPLETED
                self.metrics.end_time = datetime.utcnow()

            await self._emit_event("workflow_completed", {
                "results": self.context.results
            })

        except asyncio.CancelledError:
            logger.info(f"Workflow {self.instance_id} cancelled")

        except Exception as e:
            logger.error(f"Compensation failed for workflow {self.instance_id}: {str(e)}")
            self.context.add_error("compensation", e)

            await self._emit_event("compensation_failed", {
                "error": str(e)
            })

    async def _compensate_backward(self) -> None:
        """Compensate steps in reverse order"""
        for step_id in reversed(self.compensation_stack):
            await self._compensate_step(step_id)

    async def _compensate_forward(self) -> None:
        """Compensate steps in forward order"""
        for step_id in self.compensation_stack:
            await self._compensate_step(step_id)

    async def _compensate_parallel(self) -> None:
        """Compensate all steps in parallel"""
        tasks = [
            self._compensate_step(step_id)
            for step_id in self.compensation_stack
        ]

        if tasks:
            await asyncio.gather(*tasks, return_exceptions=True)

    async def _compensate_step(self, step_id: str) -> None:
        """Compensate a single step"""
        step = self.definition.get_step(step_id)
        if not step:
            return

        try:
            step.state = StepState.COMPENSATING

            await self._emit_event("step_compensation_started", {
                "step_id": step_id
            })

            await step.compensate(self.context)

            step.state = StepState.COMPENSATED
            self.metrics.compensation_count += 1

            await self._emit_event("step_compensation_completed", {
                "step_id": step_id
            })

        except Exception as e:
            logger.error(f"Failed to compensate step {step_id}: {str(e)}")
            self.context.add_error(f"{step_id}_compensation", e)

            await self._emit_event("step_compensation_failed", {
                "step_id": step_id,
                "error": str(e)
            })

    async def _validate_workflow(self) -> None:
        """Validate workflow without executing"""
        errors = self.definition.validate()

        if errors:
            raise ValueError(f"Workflow validation failed: {', '.join(errors)}")

        # Validate each step
        for step in self.definition.steps:
            error = await step.validate(self.context)
            if error:
                raise ValueError(f"Step {step.step_id} validation failed: {error}")

    async def _call_handler(self, handler: Callable, *args) -> Any:
        """Call a handler function (sync or async)"""
        if asyncio.iscoroutinefunction(handler):
            return await handler(*args)
        else:
            return handler(*args)

    async def _emit_event(self, event_type: str, data: Dict[str, Any] = None) -> None:
        """Emit workflow event"""
        event = WorkflowEvent(
            instance_id=self.instance_id,
            event_type=event_type,
            data=data
        )

        # Call registered handlers
        handlers = self.event_handlers.get(event_type, [])
        for handler in handlers:
            try:
                await self._call_handler(handler, event)
            except Exception as e:
                logger.error(f"Event handler error: {str(e)}")

    def on_event(self, event_type: str, handler: Callable) -> None:
        """Register event handler"""
        self.event_handlers[event_type].append(handler)

    def get_state(self) -> WorkflowState:
        """Get current workflow state"""
        return self.state

    def get_results(self) -> Dict[str, Any]:
        """Get workflow results"""
        return {
            "state": self.state.value,
            "results": self.context.results,
            "errors": self.context.errors,
            "metrics": self.metrics.to_dict()
        }

    def get_progress(self) -> Dict[str, Any]:
        """Get workflow progress information"""
        return {
            "current_step": self.current_step_index,
            "total_steps": self.metrics.total_steps,
            "completed_steps": self.metrics.completed_steps,
            "failed_steps": self.metrics.failed_steps,
            "progress_percentage": (
                        self.metrics.completed_steps / self.metrics.total_steps * 100) if self.metrics.total_steps > 0 else 0
        }

    def to_dict(self) -> Dict[str, Any]:
        """Serialize workflow instance"""
        return {
            "instance_id": str(self.instance_id),
            "workflow_id": self.definition.workflow_id,
            "state": self.state.value,
            "execution_mode": self.execution_mode.value,
            "context": self.context.to_dict(),
            "metrics": self.metrics.to_dict(),
            "current_step_index": self.current_step_index,
            "step_results": {
                step_id: result.to_dict()
                for step_id, result in self.step_results.items()
            }
        }


class WorkflowEngine:
    """Main workflow orchestration engine"""

    def __init__(self,
                 routing_engine: Optional[RoutingEngine] = None,
                 transport_manager: Optional[TransportManager] = None,
                 persistence: Optional[WorkflowPersistence] = None,
                 max_concurrent_workflows: int = 100):
        self.routing_engine = routing_engine
        self.transport_manager = transport_manager
        self.persistence = persistence
        self.max_concurrent_workflows = max_concurrent_workflows

        # Workflow storage
        self.definitions: Dict[str, WorkflowDefinition] = {}
        self.instances: Dict[UUID, WorkflowInstance] = {}
        self.pending_messages: Dict[UUID, asyncio.Future] = {}

        # Execution control
        self._semaphore = asyncio.Semaphore(max_concurrent_workflows)
        self._cleanup_task: Optional[asyncio.Task] = None
        self._message_handler_task: Optional[asyncio.Task] = None

        # Metrics
        self.total_workflows_started = 0
        self.total_workflows_completed = 0
        self.total_workflows_failed = 0

    def register_workflow(self, definition: WorkflowDefinition) -> None:
        """Register a workflow definition"""
        errors = definition.validate()
        if errors:
            raise ValueError(f"Invalid workflow: {', '.join(errors)}")

        self.definitions[definition.workflow_id] = definition
        logger.info(f"Registered workflow: {definition.name} (v{definition.version})")

    def unregister_workflow(self, workflow_id: str) -> None:
        """Unregister a workflow definition"""
        if workflow_id in self.definitions:
            del self.definitions[workflow_id]
            logger.info(f"Unregistered workflow: {workflow_id}")

    async def start_workflow(self,
                             workflow_id: str,
                             input_data: Optional[Dict[str, Any]] = None,
                             execution_mode: ExecutionMode = ExecutionMode.NORMAL,
                             parent_instance_id: Optional[UUID] = None) -> UUID:
        """Start a new workflow instance"""
        if workflow_id not in self.definitions:
            raise ValueError(f"Unknown workflow: {workflow_id}")

        definition = self.definitions[workflow_id]

        # Create context
        context = WorkflowContext(
            workflow_id=workflow_id,
            parent_id=parent_instance_id
        )

        # Set input variables
        if input_data:
            for key, value in input_data.items():
                context.set(key, value)

        # Create instance
        instance = WorkflowInstance(definition, context, execution_mode)

        # Register event handlers
        instance.on_event("workflow_completed", self._on_workflow_completed)
        instance.on_event("workflow_failed", self._on_workflow_failed)

        # Store instance
        self.instances[instance.instance_id] = instance

        # Start execution with semaphore
        async with self._semaphore:
            instance_id = await instance.start()
            self.total_workflows_started += 1

        # Persist if enabled
        if self.persistence:
            await self.persistence.save_instance(instance)

        logger.info(f"Started workflow instance: {instance_id}")
        return instance_id

    async def get_instance(self, instance_id: UUID) -> Optional[WorkflowInstance]:
        """Get workflow instance"""
        if instance_id in self.instances:
            return self.instances[instance_id]

        # Try to load from persistence
        if self.persistence:
            instance_data = await self.persistence.load_instance(instance_id)
            if instance_data:
                # Reconstruct instance
                context = WorkflowContext.from_dict(instance_data["context"])
                definition = self.definitions.get(context.workflow_id)

                if definition:
                    instance = WorkflowInstance(definition, context)
                    # Restore state
                    instance.state = WorkflowState(instance_data["state"])
                    instance.current_step_index = instance_data["current_step_index"]

                    self.instances[instance_id] = instance
                    return instance

        return None

    async def cancel_workflow(self, instance_id: UUID) -> None:
        """Cancel a running workflow"""
        instance = await self.get_instance(instance_id)
        if instance:
            await instance.cancel()

    async def pause_workflow(self, instance_id: UUID) -> None:
        """Pause a running workflow"""
        instance = await self.get_instance(instance_id)
        if instance:
            await instance.pause()

    async def resume_workflow(self, instance_id: UUID) -> None:
        """Resume a paused workflow"""
        instance = await self.get_instance(instance_id)
        if instance:
            await instance.resume()

    async def get_workflow_status(self, instance_id: UUID) -> Optional[Dict[str, Any]]:
        """Get workflow status"""
        instance = await self.get_instance(instance_id)
        if instance:
            return {
                "instance_id": str(instance_id),
                "workflow_id": instance.definition.workflow_id,
                "state": instance.get_state().value,
                "progress": instance.get_progress(),
                "results": instance.get_results() if instance.state in [WorkflowState.COMPLETED,
                                                                        WorkflowState.FAILED] else None
            }
        return None

    async def list_workflows(self,
                             state_filter: Optional[WorkflowState] = None,
                             limit: int = 100) -> List[Dict[str, Any]]:
        """List workflow instances"""
        workflows = []

        for instance in self.instances.values():
            if state_filter and instance.state != state_filter:
                continue

            workflows.append({
                "instance_id": str(instance.instance_id),
                "workflow_id": instance.definition.workflow_id,
                "workflow_name": instance.definition.name,
                "state": instance.state.value,
                "created_at": instance.context.created_at.isoformat(),
                "progress": instance.get_progress()
            })

            if len(workflows) >= limit:
                break

        return workflows

    async def handle_message_response(self, message) -> None:
        """Handle response message for workflow steps"""
        correlation_id = message.header.correlation_id

        if correlation_id and correlation_id in self.pending_messages:
            future = self.pending_messages[correlation_id]
            future.set_result(message)
            del self.pending_messages[correlation_id]

    async def _on_workflow_completed(self, event: WorkflowEvent) -> None:
        """Handle workflow completion"""
        self.total_workflows_completed += 1

        # Persist final state
        if self.persistence:
            instance = self.instances.get(event.instance_id)
            if instance:
                await self.persistence.save_instance(instance)

    async def _on_workflow_failed(self, event: WorkflowEvent) -> None:
        """Handle workflow failure"""
        self.total_workflows_failed += 1

        # Persist failure state
        if self.persistence:
            instance = self.instances.get(event.instance_id)
            if instance:
                await self.persistence.save_instance(instance)

    async def cleanup_completed_instances(self, retention_period: timedelta) -> None:
        """Remove completed workflow instances older than retention period"""
        cutoff_time = datetime.utcnow() - retention_period
        instances_to_remove = []

        for instance_id, instance in self.instances.items():
            if instance.state in [WorkflowState.COMPLETED, WorkflowState.FAILED, WorkflowState.COMPENSATED]:
                if instance.metrics.end_time and instance.metrics.end_time < cutoff_time:
                    instances_to_remove.append(instance_id)

        for instance_id in instances_to_remove:
            del self.instances[instance_id]

            # Archive in persistence if enabled
            if self.persistence:
                await self.persistence.archive_instance(instance_id)

        if instances_to_remove:
            logger.info(f"Cleaned up {len(instances_to_remove)} workflow instances")

    async def start_background_tasks(self) -> None:
        """Start background maintenance tasks"""

        # Cleanup task
        async def cleanup_loop():
            while True:
                await asyncio.sleep(3600)  # Run hourly
                await self.cleanup_completed_instances(timedelta(days=7))

        self._cleanup_task = asyncio.create_task(cleanup_loop())

    async def shutdown(self) -> None:
        """Shutdown workflow engine"""
        # Cancel background tasks
        if self._cleanup_task:
            self._cleanup_task.cancel()

        # Cancel all running workflows
        for instance in self.instances.values():
            if instance.state in [WorkflowState.RUNNING, WorkflowState.PAUSED]:
                await instance.cancel()

        logger.info("Workflow engine shutdown complete")

    def get_metrics(self) -> Dict[str, Any]:
        """Get engine metrics"""
        return {
            "total_started": self.total_workflows_started,
            "total_completed": self.total_workflows_completed,
            "total_failed": self.total_workflows_failed,
            "active_instances": len([i for i in self.instances.values() if i.state == WorkflowState.RUNNING]),
            "registered_workflows": len(self.definitions)
        }(f"Workflow {self.instance_id} failed: {str(e)}")

        async with self._state_lock:
            self.state = WorkflowState.FAILED
            self.metrics.end_time = datetime.utcnow()

        self.context.add_error("workflow", e, {
            "step_index": self.current_step_index
        })

        # Trigger compensation
        if self.definition.compensation_strategy != CompensationStrategy.NONE:
            await self._compensate()

        await self._emit_event("workflow_failed", {
            "error": str(e),
            "step_index": self.current_step_index
        })

        raise


async def _execute_step(self, step) -> None:
    """Execute a single workflow step with retry and error handling"""
    step_start = datetime.utcnow()
    attempt = 0
    last_error = None

    # Create step result
    result = StepResult(
        step_id=step.step_id,
        state=StepState.PENDING,
        start_time=step_start
    )

    while attempt <= step.retry_policy.max_attempts:
        try:
            # Update step state
            step.state = StepState.RUNNING
            result.state = StepState.RUNNING

            # Emit step started event
            await self._emit_event("step_started", {
                "step_id": step.step_id,
                "step_type": step.get_type().value,
                "attempt": attempt
            })

            # Validate step
            validation_error = await step.validate(self.context)
            if validation_error:
                raise ValueError(f"Step validation failed: {validation_error}")

            # Execute step with timeout
            step_result = await asyncio.wait_for(
                step.execute(self.context),
                timeout=step.timeout.total_seconds()
            )

            # Success
            step.state = StepState.COMPLETED
            result.state = StepState.COMPLETED
            result.value = step_result
            result.end_time = datetime.utcnow()

            # Store result
            self.context.add_result(step.step_id, step_result)
            self.step_results[step.step_id] = result

            # Add to compensation stack
            if step.compensation_handler or hasattr(step, 'compensate'):
                self.compensation_stack.append(step.step_id)

            # Update metrics
            self.metrics.completed_steps += 1

            # Call success handler
            if step.on_success:
                await self._call_handler(step.on_success, self.context)

            # Emit step completed event
            await self._emit_event("step_completed", {
                "step_id": step.step_id,
                "duration": result.duration.total_seconds() if result.duration else 0,
                "result": step_result
            })

            return

        except asyncio.TimeoutError:
            last_error = TimeoutError(f"Step {step.step_id} timed out")
            step.state = StepState.TIMED_OUT
            result.state = StepState.TIMED_OUT

        except Exception as e:
            last_error = e
            step.state = StepState.FAILED
            result.state = StepState.FAILED
            result.error = e

            # Check if should retry
            if not step.retry_policy.should_retry(e, attempt + 1):
                break

        # Update retry count
        attempt += 1
        result.retry_count = attempt
        self.metrics.retry_count += 1

        if attempt <= step.retry_policy.max_attempts:
            # Calculate retry delay
            delay = step.retry_policy.calculate_delay(attempt)

            # Emit retry event
            await self._emit_event("step_retry", {
                "step_id": step.step_id,
                "attempt": attempt,
                "delay": delay,
                "error": str(last_error)
            })

            await asyncio.sleep(delay)

    # Step failed after all retries
    result.end_time = datetime.utcnow()
    result.error = last_error
    self.step_results[step.step_id] = result
    self.context.add_error(step.step_id, last_error)
    self.metrics.failed_steps += 1

    # Call failure handler
    if step.on_failure:
        await self._call_handler(step.on_failure, self.context)

    # Emit step failed event
    await self._emit_event("step_failed", {
        "step_id": step.step_id,
        "error": str(last_error),
        "attempts": attempt
    })

    raise last_error


async def _compensate(self) -> None:
    """Execute compensation logic"""
    async with self._state_lock:
        self.state = WorkflowState.COMPENSATING

    await self._emit_event("compensation_started", {
        "strategy": self.definition.compensation_strategy.value,
        "steps_to_compensate": len(self.compensation_stack)
    })

    try:
        if self.definition.compensation_strategy == CompensationStrategy.BACKWARD:
            await self._compensate_backward()
        elif self.definition.compensation_strategy == CompensationStrategy.FORWARD:
            await self._compensate_forward()
        elif self.definition.compensation_strategy == CompensationStrategy.PARALLEL:
            await self._compensate_parallel()

        async with self._state_lock:
            self.state = WorkflowState.COMPENSATED

        await self._emit_event("compensation_completed", {
            "compensated_steps": self.metrics.compensation_count
        })

    except Exception as e:
        logger.error