# File: maple/core/map/orchestration/steps.py
# Description: Concrete workflow step implementations for the MAP orchestration engine.
# Provides various step types for building complex workflows.

from __future__ import annotations
import asyncio
import json
import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Any, Callable, Union, Set
from uuid import UUID, uuid4
import re
from jinja2 import Template, Environment, meta

from maple.core.map.orchestration.models import (
    WorkflowStep, StepState, StepResult, WorkflowContext,
    RetryPolicy, CompensationStrategy
)
from maple.core.map.models.message import (
    MAPMessage, MessageType, MessagePriority,
    MessagePayload, MessageHeader, MessageDestination
)
from maple.core.map.routing.engine import RoutingEngine
from maple.core.map.transport.base import TransportManager

logger = logging.getLogger(__name__)


class MessageStep(WorkflowStep):
    """Step that sends a message to an agent"""

    def __init__(self,
                 step_id: str,
                 destination: Union[str, Callable[[WorkflowContext], str]],
                 message_template: Dict[str, Any],
                 routing_engine: RoutingEngine,
                 transport_manager: TransportManager,
                 **kwargs):
        super().__init__(step_id, step_type="message", **kwargs)
        self.destination = destination
        self.message_template = message_template
        self.routing_engine = routing_engine
        self.transport_manager = transport_manager

    async def execute(self, context: WorkflowContext) -> StepResult:
        """Send message to agent"""
        try:
            # Resolve destination
            if callable(self.destination):
                dest_agent = self.destination(context)
            else:
                dest_agent = self.destination

            # Build message from template
            message_data = self._expand_template(self.message_template, context)

            # Create MAP message
            message = MAPMessage(
                header=MessageHeader(
                    destination=MessageDestination(agent_id=dest_agent),
                    priority=MessagePriority(message_data.get('priority', 'medium'))
                ),
                payload=MessagePayload(
                    type=MessageType(message_data.get('type', 'request')),
                    action=message_data.get('action', 'process'),
                    data=message_data.get('data', {}),
                    metadata=message_data.get('metadata', {})
                )
            )

            # Route and send
            route = await self.routing_engine.route_message(message)
            if not route:
                raise Exception(f"No route found for agent {dest_agent}")

            receipt = await self.transport_manager.send_message(message, route)

            return StepResult(
                step_id=self.step_id,
                state=StepState.COMPLETED,
                output={"receipt": receipt.to_dict(), "message_id": str(message.header.message_id)},
                end_time=datetime.utcnow()
            )

        except Exception as e:
            logger.error(f"TransformStep {self.step_id} failed: {str(e)}")
            return StepResult(
                step_id=self.step_id,
                state=StepState.FAILED,
                error=str(e),
                end_time=datetime.utcnow()
            )

    def _transform_dict(self, template: Dict, context: WorkflowContext) -> Dict:
        """Recursively transform dictionary values"""
        result = {}
        for key, value in template.items():
            if isinstance(value, str) and '{{' in value:
                template_obj = Template(value)
                result[key] = template_obj.render(**context.variables)
            elif isinstance(value, dict):
                result[key] = self._transform_dict(value, context)
            else:
                result[key] = value
        return result


class AggregateStep(WorkflowStep):
    """Step that aggregates results from previous steps"""

    def __init__(self,
                 step_id: str,
                 source_steps: List[str],
                 aggregation_func: Union[str, Callable[[List[Any]], Any]],
                 output_key: str,
                 **kwargs):
        super().__init__(step_id, step_type="aggregate", **kwargs)
        self.source_steps = source_steps
        self.aggregation_func = aggregation_func
        self.output_key = output_key

    async def execute(self, context: WorkflowContext) -> StepResult:
        """Aggregate results from multiple steps"""
        try:
            # Collect results from source steps
            source_values = []
            for step_id in self.source_steps:
                if step_id in context.results:
                    source_values.append(context.results[step_id])

            if not source_values:
                return StepResult(
                    step_id=self.step_id,
                    state=StepState.FAILED,
                    error="No source values found for aggregation",
                    end_time=datetime.utcnow()
                )

            # Apply aggregation function
            if callable(self.aggregation_func):
                result = self.aggregation_func(source_values)
            else:
                # Built-in aggregation functions
                if self.aggregation_func == "sum":
                    result = sum(source_values)
                elif self.aggregation_func == "avg":
                    result = sum(source_values) / len(source_values)
                elif self.aggregation_func == "min":
                    result = min(source_values)
                elif self.aggregation_func == "max":
                    result = max(source_values)
                elif self.aggregation_func == "count":
                    result = len(source_values)
                elif self.aggregation_func == "concat":
                    result = "".join(str(v) for v in source_values)
                elif self.aggregation_func == "list":
                    result = source_values
                else:
                    raise ValueError(f"Unknown aggregation function: {self.aggregation_func}")

            # Store result in context
            context.set(self.output_key, result)

            return StepResult(
                step_id=self.step_id,
                state=StepState.COMPLETED,
                output={
                    "aggregation_func": str(self.aggregation_func),
                    "source_count": len(source_values),
                    "result": result
                },
                end_time=datetime.utcnow()
            )

        except Exception as e:
            logger.error(f"AggregateStep {self.step_id} failed: {str(e)}")
            return StepResult(
                step_id=self.step_id,
                state=StepState.FAILED,
                error=str(e),
                end_time=datetime.utcnow()
            )


class CustomStep(WorkflowStep):
    """Step that executes custom user-defined logic"""

    def __init__(self,
                 step_id: str,
                 handler: Callable[[WorkflowContext], Any],
                 async_handler: bool = True,
                 **kwargs):
        super().__init__(step_id, step_type="custom", **kwargs)
        self.handler = handler
        self.async_handler = async_handler

    async def execute(self, context: WorkflowContext) -> StepResult:
        """Execute custom handler"""
        try:
            if self.async_handler:
                result = await self.handler(context)
            else:
                # Run sync handler in thread pool
                import asyncio
                loop = asyncio.get_event_loop()
                result = await loop.run_in_executor(None, self.handler, context)

            return StepResult(
                step_id=self.step_id,
                state=StepState.COMPLETED,
                output={"result": result},
                end_time=datetime.utcnow()
            )

        except Exception as e:
            logger.error(f"CustomStep {self.step_id} failed: {str(e)}")
            return StepResult(
                step_id=self.step_id,
                state=StepState.FAILED,
                error=str(e),
                end_time=datetime.utcnow()
            )


# Factory function for creating steps from configuration
def create_step_from_config(config: Dict[str, Any],
                            routing_engine: Optional[RoutingEngine] = None,
                            transport_manager: Optional[TransportManager] = None,
                            workflow_engine: Optional['WorkflowEngine'] = None) -> WorkflowStep:
    """Factory function to create workflow steps from configuration"""

    step_type = config.get('type')
    step_id = config.get('id', f"step_{uuid4().hex[:8]}")

    # Common parameters
    common_params = {
        'name': config.get('name'),
        'description': config.get('description'),
        'timeout': timedelta(seconds=config.get('timeout', 300)),
        'retry_policy': RetryPolicy(**config.get('retry_policy', {})) if config.get('retry_policy') else None,
        'metadata': config.get('metadata', {})
    }

    if step_type == 'message':
        return MessageStep(
            step_id=step_id,
            destination=config['destination'],
            message_template=config['template'],
            routing_engine=routing_engine,
            transport_manager=transport_manager,
            **common_params
        )

    elif step_type == 'parallel':
        sub_steps = [
            create_step_from_config(sub_config, routing_engine, transport_manager, workflow_engine)
            for sub_config in config['steps']
        ]
        return ParallelStep(
            step_id=step_id,
            steps=sub_steps,
            max_concurrent=config.get('max_concurrent'),
            fail_fast=config.get('fail_fast', True),
            **common_params
        )

    elif step_type == 'conditional':
        if_true_config = config.get('if_true')
        if_false_config = config.get('if_false')

        return ConditionalStep(
            step_id=step_id,
            condition=config['condition'],
            if_true=create_step_from_config(if_true_config, routing_engine, transport_manager,
                                            workflow_engine) if if_true_config else None,
            if_false=create_step_from_config(if_false_config, routing_engine, transport_manager,
                                             workflow_engine) if if_false_config else None,
            **common_params
        )

    elif step_type == 'loop':
        body_config = config['body']
        return LoopStep(
            step_id=step_id,
            items=config['items'],
            loop_var=config['loop_var'],
            body=create_step_from_config(body_config, routing_engine, transport_manager, workflow_engine),
            max_concurrent=config.get('max_concurrent', 1),
            **common_params
        )

    elif step_type == 'subworkflow':
        return SubWorkflowStep(
            step_id=step_id,
            workflow_id=config['workflow_id'],
            workflow_engine=workflow_engine,
            input_mapping=config.get('input_mapping'),
            output_mapping=config.get('output_mapping'),
            **common_params
        )

    elif step_type == 'wait':
        return WaitStep(
            step_id=step_id,
            duration=config.get('duration'),
            until=config.get('until'),
            check_interval=config.get('check_interval', 1),
            **common_params
        )

    elif step_type == 'transform':
        return TransformStep(
            step_id=step_id,
            transformations=config['transformations'],
            **common_params
        )

    elif step_type == 'aggregate':
        return AggregateStep(
            step_id=step_id,
            source_steps=config['source_steps'],
            aggregation_func=config['aggregation_func'],
            output_key=config['output_key'],
            **common_params
        )

    else:
        raise ValueError(f"Unknown step type: {step_type}")


# Example step patterns
class CircuitBreakerStep(CustomStep):
    """Circuit breaker pattern implementation"""

    def __init__(self,
                 step_id: str,
                 protected_step: WorkflowStep,
                 failure_threshold: int = 5,
                 timeout: timedelta = timedelta(minutes=1),
                 **kwargs):
        self.protected_step = protected_step
        self.failure_threshold = failure_threshold
        self.timeout = timeout
        self.failure_count = 0
        self.last_failure_time = None
        self.is_open = False

        async def circuit_breaker_handler(context: WorkflowContext):
            # Check if circuit is open
            if self.is_open:
                if datetime.utcnow() - self.last_failure_time < self.timeout:
                    raise Exception("Circuit breaker is open")
                else:
                    # Try to close circuit
                    self.is_open = False
                    self.failure_count = 0

            try:
                result = await self.protected_step.execute(context)
                if result.state == StepState.FAILED:
                    self.failure_count += 1
                    if self.failure_count >= self.failure_threshold:
                        self.is_open = True
                        self.last_failure_time = datetime.utcnow()
                else:
                    self.failure_count = 0
                return result
            except Exception as e:
                self.failure_count += 1
                if self.failure_count >= self.failure_threshold:
                    self.is_open = True
                    self.last_failure_time = datetime.utcnow()
                raise

        super().__init__(step_id, circuit_breaker_handler, **kwargs)


class RetryStep(CustomStep):
    """Retry pattern implementation with exponential backoff"""

    def __init__(self,
                 step_id: str,
                 protected_step: WorkflowStep,
                 max_retries: int = 3,
                 initial_delay: float = 1.0,
                 backoff_factor: float = 2.0,
                 **kwargs):
        self.protected_step = protected_step
        self.max_retries = max_retries
        self.initial_delay = initial_delay
        self.backoff_factor = backoff_factor

        async def retry_handler(context: WorkflowContext):
            delay = self.initial_delay
            last_error = None

            for attempt in range(self.max_retries + 1):
                try:
                    result = await self.protected_step.execute(context)
                    if result.state == StepState.COMPLETED:
                        return result
                    last_error = result.error
                except Exception as e:
                    last_error = str(e)

                if attempt < self.max_retries:
                    await asyncio.sleep(delay)
                    delay *= self.backoff_factor

            raise Exception(f"Max retries exceeded. Last error: {last_error}")

        super().__init__(step_id, retry_handler, **kwargs)
        d_time = datetime.utcnow()
        )

        except Exception as e:
        logger.error(f"MessageStep {self.step_id} failed: {str(e)}")
        return StepResult(
            step_id=self.step_id,
            state=StepState.FAILED,
            error=str(e),
            end_time=datetime.utcnow()
        )


def _expand_template(self, template: Dict[str, Any], context: WorkflowContext) -> Dict[str, Any]:
    """Expand template with context variables"""
    result = {}
    for key, value in template.items():
        if isinstance(value, str) and '{{' in value:
            # Jinja2 template expansion
            tmpl = Template(value)
            result[key] = tmpl.render(**context.variables)
        elif isinstance(value, dict):
            result[key] = self._expand_template(value, context)
        else:
            result[key] = value
    return result


class ParallelStep(WorkflowStep):
    """Step that executes multiple sub-steps in parallel"""

    def __init__(self,
                 step_id: str,
                 steps: List[WorkflowStep],
                 max_concurrent: Optional[int] = None,
                 fail_fast: bool = True,
                 **kwargs):
        super().__init__(step_id, step_type="parallel", **kwargs)
        self.steps = steps
        self.max_concurrent = max_concurrent
        self.fail_fast = fail_fast

    async def execute(self, context: WorkflowContext) -> StepResult:
        """Execute all sub-steps in parallel"""
        try:
            if self.max_concurrent:
                # Limited concurrency
                semaphore = asyncio.Semaphore(self.max_concurrent)

                async def run_with_semaphore(step: WorkflowStep):
                    async with semaphore:
                        return await step.execute(context)

                tasks = [run_with_semaphore(step) for step in self.steps]
            else:
                # Unlimited concurrency
                tasks = [step.execute(context) for step in self.steps]

            # Execute with fail-fast option
            if self.fail_fast:
                results = await asyncio.gather(*tasks, return_exceptions=False)
            else:
                results = await asyncio.gather(*tasks, return_exceptions=True)

            # Aggregate results
            all_completed = all(
                r.state == StepState.COMPLETED
                for r in results
                if isinstance(r, StepResult)
            )

            output = {
                "results": [r.to_dict() if isinstance(r, StepResult) else str(r) for r in results],
                "completed_count": sum(
                    1 for r in results if isinstance(r, StepResult) and r.state == StepState.COMPLETED),
                "total_count": len(self.steps)
            }

            return StepResult(
                step_id=self.step_id,
                state=StepState.COMPLETED if all_completed else StepState.FAILED,
                output=output,
                end_time=datetime.utcnow()
            )

        except Exception as e:
            logger.error(f"ParallelStep {self.step_id} failed: {str(e)}")
            return StepResult(
                step_id=self.step_id,
                state=StepState.FAILED,
                error=str(e),
                end_time=datetime.utcnow()
            )

    async def compensate(self, context: WorkflowContext) -> None:
        """Compensate all completed sub-steps"""
        compensation_tasks = []
        for step in self.steps:
            if hasattr(step, '_result') and step._result.state == StepState.COMPLETED:
                if step.compensation_handler:
                    compensation_tasks.append(step.compensate(context))

        if compensation_tasks:
            await asyncio.gather(*compensation_tasks, return_exceptions=True)


class ConditionalStep(WorkflowStep):
    """Step that executes based on a condition"""

    def __init__(self,
                 step_id: str,
                 condition: Union[str, Callable[[WorkflowContext], bool]],
                 if_true: Optional[WorkflowStep] = None,
                 if_false: Optional[WorkflowStep] = None,
                 **kwargs):
        super().__init__(step_id, step_type="conditional", **kwargs)
        self.condition = condition
        self.if_true = if_true
        self.if_false = if_false
        self.executed_branch: Optional[WorkflowStep] = None

    async def execute(self, context: WorkflowContext) -> StepResult:
        """Evaluate condition and execute appropriate branch"""
        try:
            # Evaluate condition
            if callable(self.condition):
                result = self.condition(context)
            else:
                # String expression evaluation
                result = self._evaluate_expression(self.condition, context)

            # Execute appropriate branch
            if result and self.if_true:
                self.executed_branch = self.if_true
                branch_result = await self.if_true.execute(context)
                branch_name = "if_true"
            elif not result and self.if_false:
                self.executed_branch = self.if_false
                branch_result = await self.if_false.execute(context)
                branch_name = "if_false"
            else:
                # No branch to execute
                return StepResult(
                    step_id=self.step_id,
                    state=StepState.SKIPPED,
                    output={"condition_result": result, "branch_executed": None},
                    end_time=datetime.utcnow()
                )

            return StepResult(
                step_id=self.step_id,
                state=branch_result.state,
                output={
                    "condition_result": result,
                    "branch_executed": branch_name,
                    "branch_result": branch_result.to_dict()
                },
                end_time=datetime.utcnow()
            )

        except Exception as e:
            logger.error(f"ConditionalStep {self.step_id} failed: {str(e)}")
            return StepResult(
                step_id=self.step_id,
                state=StepState.FAILED,
                error=str(e),
                end_time=datetime.utcnow()
            )

    def _evaluate_expression(self, expression: str, context: WorkflowContext) -> bool:
        """Safely evaluate a boolean expression"""
        # Simple expression evaluation (can be enhanced)
        # Supports: ==, !=, <, >, <=, >=, and, or, not
        try:
            # Replace context variables
            env = Environment()
            ast = env.parse(expression)
            variables = meta.find_undeclared_variables(ast)

            local_vars = {}
            for var in variables:
                local_vars[var] = context.get(var)

            template = Template(expression)
            expanded = template.render(**local_vars)

            # Safe evaluation (limited to comparisons and boolean ops)
            # This is a simplified implementation - in production, use a proper expression evaluator
            return eval(expanded, {"__builtins__": {}}, {})

        except Exception as e:
            logger.error(f"Expression evaluation failed: {str(e)}")
            return False

    async def compensate(self, context: WorkflowContext) -> None:
        """Compensate the executed branch"""
        if self.executed_branch and self.executed_branch.compensation_handler:
            await self.executed_branch.compensate(context)


class LoopStep(WorkflowStep):
    """Step that iterates over a collection"""

    def __init__(self,
                 step_id: str,
                 items: Union[str, List[Any], Callable[[WorkflowContext], List[Any]]],
                 loop_var: str,
                 body: WorkflowStep,
                 max_concurrent: int = 1,
                 **kwargs):
        super().__init__(step_id, step_type="loop", **kwargs)
        self.items = items
        self.loop_var = loop_var
        self.body = body
        self.max_concurrent = max_concurrent
        self.executed_iterations: List[WorkflowStep] = []

    async def execute(self, context: WorkflowContext) -> StepResult:
        """Execute loop body for each item"""
        try:
            # Get items to iterate over
            if callable(self.items):
                items_list = self.items(context)
            elif isinstance(self.items, str):
                # Variable name in context
                items_list = context.get(self.items, [])
            else:
                items_list = self.items

            if not items_list:
                return StepResult(
                    step_id=self.step_id,
                    state=StepState.COMPLETED,
                    output={"iterations": 0, "results": []},
                    end_time=datetime.utcnow()
                )

            # Execute iterations with concurrency control
            semaphore = asyncio.Semaphore(self.max_concurrent)

            async def execute_iteration(item: Any, index: int):
                async with semaphore:
                    # Create iteration context
                    iter_context = WorkflowContext(
                        workflow_id=context.workflow_id,
                        instance_id=context.instance_id,
                        variables={**context.variables, self.loop_var: item, f"{self.loop_var}_index": index}
                    )

                    # Clone body step for this iteration
                    import copy
                    iter_step = copy.deepcopy(self.body)
                    iter_step.step_id = f"{self.step_id}[{index}]"
                    self.executed_iterations.append(iter_step)

                    return await iter_step.execute(iter_context)

            tasks = [execute_iteration(item, i) for i, item in enumerate(items_list)]
            results = await asyncio.gather(*tasks, return_exceptions=True)

            # Aggregate results
            successful = sum(1 for r in results if isinstance(r, StepResult) and r.state == StepState.COMPLETED)

            return StepResult(
                step_id=self.step_id,
                state=StepState.COMPLETED if successful == len(items_list) else StepState.FAILED,
                output={
                    "iterations": len(items_list),
                    "successful": successful,
                    "results": [r.to_dict() if isinstance(r, StepResult) else str(r) for r in results]
                },
                end_time=datetime.utcnow()
            )

        except Exception as e:
            logger.error(f"LoopStep {self.step_id} failed: {str(e)}")
            return StepResult(
                step_id=self.step_id,
                state=StepState.FAILED,
                error=str(e),
                end_time=datetime.utcnow()
            )

    async def compensate(self, context: WorkflowContext) -> None:
        """Compensate all executed iterations"""
        compensation_tasks = []
        for step in self.executed_iterations:
            if hasattr(step, '_result') and step._result.state == StepState.COMPLETED:
                if step.compensation_handler:
                    compensation_tasks.append(step.compensate(context))

        if compensation_tasks:
            await asyncio.gather(*compensation_tasks, return_exceptions=True)


class SubWorkflowStep(WorkflowStep):
    """Step that executes another workflow"""

    def __init__(self,
                 step_id: str,
                 workflow_id: str,
                 workflow_engine: 'WorkflowEngine',
                 input_mapping: Optional[Dict[str, str]] = None,
                 output_mapping: Optional[Dict[str, str]] = None,
                 **kwargs):
        super().__init__(step_id, step_type="subworkflow", **kwargs)
        self.workflow_id = workflow_id
        self.workflow_engine = workflow_engine
        self.input_mapping = input_mapping or {}
        self.output_mapping = output_mapping or {}
        self.sub_instance_id: Optional[UUID] = None

    async def execute(self, context: WorkflowContext) -> StepResult:
        """Execute sub-workflow"""
        try:
            # Prepare sub-workflow context
            sub_variables = {}
            for target_key, source_key in self.input_mapping.items():
                sub_variables[target_key] = context.get(source_key)

            # Start sub-workflow
            instance_id = await self.workflow_engine.start_workflow(
                self.workflow_id,
                variables=sub_variables,
                parent_id=context.instance_id
            )
            self.sub_instance_id = instance_id

            # Wait for completion
            result = await self.workflow_engine.wait_for_completion(instance_id)

            # Map outputs back
            if self.output_mapping and result.get('results'):
                for source_key, target_key in self.output_mapping.items():
                    if source_key in result['results']:
                        context.set(target_key, result['results'][source_key])

            return StepResult(
                step_id=self.step_id,
                state=StepState.COMPLETED if result['state'] == 'completed' else StepState.FAILED,
                output={
                    "sub_workflow_id": str(instance_id),
                    "sub_workflow_state": result['state'],
                    "sub_workflow_results": result.get('results', {})
                },
                end_time=datetime.utcnow()
            )

        except Exception as e:
            logger.error(f"SubWorkflowStep {self.step_id} failed: {str(e)}")
            return StepResult(
                step_id=self.step_id,
                state=StepState.FAILED,
                error=str(e),
                end_time=datetime.utcnow()
            )

    async def compensate(self, context: WorkflowContext) -> None:
        """Cancel or compensate sub-workflow"""
        if self.sub_instance_id:
            try:
                await self.workflow_engine.cancel_workflow(self.sub_instance_id)
            except Exception as e:
                logger.error(f"Failed to cancel sub-workflow: {str(e)}")


class WaitStep(WorkflowStep):
    """Step that waits for a duration or condition"""

    def __init__(self,
                 step_id: str,
                 duration: Optional[Union[int, timedelta]] = None,
                 until: Optional[Union[datetime, Callable[[WorkflowContext], bool]]] = None,
                 check_interval: int = 1,
                 **kwargs):
        super().__init__(step_id, step_type="wait", **kwargs)
        self.duration = duration if isinstance(duration, timedelta) else timedelta(
            seconds=duration) if duration else None
        self.until = until
        self.check_interval = check_interval

    async def execute(self, context: WorkflowContext) -> StepResult:
        """Wait for specified duration or condition"""
        try:
            start_time = datetime.utcnow()

            if self.duration:
                # Simple duration wait
                await asyncio.sleep(self.duration.total_seconds())
                wait_type = "duration"
                wait_info = str(self.duration)

            elif self.until:
                # Wait for condition
                if isinstance(self.until, datetime):
                    # Wait until specific time
                    wait_seconds = (self.until - datetime.utcnow()).total_seconds()
                    if wait_seconds > 0:
                        await asyncio.sleep(wait_seconds)
                    wait_type = "until_time"
                    wait_info = self.until.isoformat()
                else:
                    # Wait for condition function
                    while not self.until(context):
                        await asyncio.sleep(self.check_interval)
                        # Add timeout check here if needed
                    wait_type = "until_condition"
                    wait_info = "condition_met"
            else:
                # No wait specified
                wait_type = "none"
                wait_info = "no_wait"

            return StepResult(
                step_id=self.step_id,
                state=StepState.COMPLETED,
                output={
                    "wait_type": wait_type,
                    "wait_info": wait_info,
                    "actual_duration": str(datetime.utcnow() - start_time)
                },
                end_time=datetime.utcnow()
            )

        except Exception as e:
            logger.error(f"WaitStep {self.step_id} failed: {str(e)}")
            return StepResult(
                step_id=self.step_id,
                state=StepState.FAILED,
                error=str(e),
                end_time=datetime.utcnow()
            )


class TransformStep(WorkflowStep):
    """Step that transforms data using templates"""

    def __init__(self,
                 step_id: str,
                 transformations: Dict[str, Union[str, Dict, Callable]],
                 **kwargs):
        super().__init__(step_id, step_type="transform", **kwargs)
        self.transformations = transformations

    async def execute(self, context: WorkflowContext) -> StepResult:
        """Apply transformations to context"""
        try:
            results = {}

            for key, transformation in self.transformations.items():
                if callable(transformation):
                    # Function transformation
                    results[key] = transformation(context)
                elif isinstance(transformation, str):
                    # Jinja2 template
                    template = Template(transformation)
                    results[key] = template.render(**context.variables)
                elif isinstance(transformation, dict):
                    # Nested transformation
                    results[key] = self._transform_dict(transformation, context)
                else:
                    # Direct value
                    results[key] = transformation

                # Update context with transformation result
                context.set(key, results[key])

            return StepResult(
                step_id=self.step_id,
                state=StepState.COMPLETED,
                output={"transformations": results},
                en