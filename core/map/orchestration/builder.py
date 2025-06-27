# File: maple/core/map/orchestration/builder.py
# Description: Workflow builder with fluent API and DSL for creating workflows
# programmatically or from YAML/JSON definitions.

from __future__ import annotations
import yaml
import json
from datetime import timedelta
from typing import Dict, List, Optional, Any, Callable, Union, TypeVar
from uuid import uuid4
import inspect

from maple.core.map.orchestration.models import (
    WorkflowDefinition, WorkflowStep, RetryPolicy, RetryStrategy,
    CompensationStrategy
)
from maple.core.map.orchestration.steps import (
    MessageStep, ParallelStep, ConditionalStep, LoopStep,
    SubWorkflowStep, WaitStep, TransformStep, AggregateStep, CustomStep
)

T = TypeVar('T')


class StepBuilder:
    """Builder for individual workflow steps"""

    def __init__(self, step_type: type[WorkflowStep], step_id: Optional[str] = None):
        self.step_type = step_type
        self.step_id = step_id or f"step_{uuid4().hex[:8]}"
        self.params: Dict[str, Any] = {"step_id": self.step_id}

    def with_name(self, name: str) -> 'StepBuilder':
        """Set step name"""
        self.params["name"] = name
        return self

    def with_description(self, description: str) -> 'StepBuilder':
        """Set step description"""
        self.params["description"] = description
        return self

    def with_timeout(self, timeout: Union[int, timedelta]) -> 'StepBuilder':
        """Set step timeout"""
        if isinstance(timeout, int):
            timeout = timedelta(seconds=timeout)
        self.params["timeout"] = timeout
        return self

    def with_retry(self,
                   max_attempts: int = 3,
                   strategy: RetryStrategy = RetryStrategy.EXPONENTIAL,
                   initial_delay: float = 1.0) -> 'StepBuilder':
        """Configure retry policy"""
        self.params["retry_policy"] = RetryPolicy(
            strategy=strategy,
            max_attempts=max_attempts,
            initial_delay=initial_delay
        )
        return self

    def on_success(self, handler: Callable) -> 'StepBuilder':
        """Set success handler"""
        self.params["on_success"] = handler
        return self

    def on_failure(self, handler: Callable) -> 'StepBuilder':
        """Set failure handler"""
        self.params["on_failure"] = handler
        return self

    def with_compensation(self, handler: Callable) -> 'StepBuilder':
        """Set compensation handler"""
        self.params["compensation_handler"] = handler
        return self

    def with_metadata(self, metadata: Dict[str, Any]) -> 'StepBuilder':
        """Set step metadata"""
        self.params["metadata"] = metadata
        return self

    def build(self) -> WorkflowStep:
        """Build the step"""
        return self.step_type(**self.params)


class WorkflowBuilder:
    """Fluent API for building workflows"""

    def __init__(self, workflow_id: Optional[str] = None, name: Optional[str] = None):
        self.workflow_id = workflow_id or f"workflow_{uuid4().hex[:8]}"
        self.name = name or self.workflow_id
        self.steps: List[WorkflowStep] = []
        self.current_builder: Optional[StepBuilder] = None
        self.config: Dict[str, Any] = {}

    def with_name(self, name: str) -> 'WorkflowBuilder':
        """Set workflow name"""
        self.name = name
        return self

    def with_description(self, description: str) -> 'WorkflowBuilder':
        """Set workflow description"""
        self.config["description"] = description
        return self

    def with_version(self, version: str) -> 'WorkflowBuilder':
        """Set workflow version"""
        self.config["version"] = version
        return self

    def with_timeout(self, timeout: Union[int, timedelta]) -> 'WorkflowBuilder':
        """Set workflow timeout"""
        if isinstance(timeout, int):
            timeout = timedelta(seconds=timeout)
        self.config["timeout"] = timeout
        return self

    def with_compensation_strategy(self, strategy: CompensationStrategy) -> 'WorkflowBuilder':
        """Set compensation strategy"""
        self.config["compensation_strategy"] = strategy
        return self

    def with_tags(self, *tags: str) -> 'WorkflowBuilder':
        """Add workflow tags"""
        self.config["tags"] = list(tags)
        return self

    def with_metadata(self, metadata: Dict[str, Any]) -> 'WorkflowBuilder':
        """Set workflow metadata"""
        self.config["metadata"] = metadata
        return self

    # Step builders

    def message(self,
                destination: str,
                action: str,
                data: Optional[Dict[str, Any]] = None) -> 'WorkflowBuilder':
        """Add message step"""
        self._finish_current_step()

        self.current_builder = StepBuilder(MessageStep)
        self.current_builder.params.update({
            "destination": destination,
            "action": action,
            "data_template": data or {}
        })

        return self

    def parallel(self, *steps: Union[WorkflowStep, 'WorkflowBuilder']) -> 'WorkflowBuilder':
        """Add parallel execution step"""
        self._finish_current_step()

        # Convert builders to steps
        parallel_steps = []
        for step in steps:
            if isinstance(step, WorkflowBuilder):
                parallel_steps.extend(step.steps)
            else:
                parallel_steps.append(step)

        parallel = ParallelStep(
            step_id=f"parallel_{len(self.steps)}",
            steps=parallel_steps
        )
        self.steps.append(parallel)

        return self

    def sequential(self, *steps: Union[WorkflowStep, 'WorkflowBuilder']) -> 'WorkflowBuilder':
        """Add sequential steps"""
        self._finish_current_step()

        for step in steps:
            if isinstance(step, WorkflowBuilder):
                self.steps.extend(step.steps)
            else:
                self.steps.append(step)

        return self

    def if_condition(self,
                     condition: Union[str, Callable],
                     if_true: Union[WorkflowStep, 'WorkflowBuilder'],
                     if_false: Optional[Union[WorkflowStep, 'WorkflowBuilder']] = None) -> 'WorkflowBuilder':
        """Add conditional step"""
        self._finish_current_step()

        # Convert builders to steps
        true_step = if_true.steps[0] if isinstance(if_true, WorkflowBuilder) else if_true
        false_step = None
        if if_false:
            false_step = if_false.steps[0] if isinstance(if_false, WorkflowBuilder) else if_false

        conditional = ConditionalStep(
            step_id=f"conditional_{len(self.steps)}",
            condition=condition,
            if_true=true_step,
            if_false=false_step
        )
        self.steps.append(conditional)

        return self

    def for_each(self,
                 items_source: Union[str, Callable],
                 loop_variable: str,
                 body: Union[WorkflowStep, 'WorkflowBuilder']) -> 'WorkflowBuilder':
        """Add loop step"""
        self._finish_current_step()

        # Convert builder to step
        body_step = body.steps[0] if isinstance(body, WorkflowBuilder) else body

        loop = LoopStep(
            step_id=f"loop_{len(self.steps)}",
            items_source=items_source,
            loop_variable=loop_variable,
            body=body_step
        )
        self.steps.append(loop)

        return self

    def sub_workflow(self,
                     workflow_id: str,
                     input_mapping: Optional[Dict[str, str]] = None,
                     output_mapping: Optional[Dict[str, str]] = None) -> 'WorkflowBuilder':
        """Add sub-workflow step"""
        self._finish_current_step()

        self.current_builder = StepBuilder(SubWorkflowStep)
        self.current_builder.params.update({
            "workflow_id": workflow_id,
            "input_mapping": input_mapping or {},
            "output_mapping": output_mapping or {}
        })

        return self

    def wait(self,
             duration: Optional[Union[int, timedelta]] = None,
             until_condition: Optional[Union[str, Callable]] = None) -> 'WorkflowBuilder':
        """Add wait step"""
        self._finish_current_step()

        if duration and isinstance(duration, int):
            duration = timedelta(seconds=duration)

        self.current_builder = StepBuilder(WaitStep)
        self.current_builder.params.update({
            "duration": duration,
            "until_condition": until_condition
        })

        return self

    def transform(self, **transformations: Union[str, Callable]) -> 'WorkflowBuilder':
        """Add transform step"""
        self._finish_current_step()

        self.current_builder = StepBuilder(TransformStep)
        self.current_builder.params["transformations"] = transformations

        return self

    def aggregate(self,
                  source_steps: List[str],
                  aggregation_type: str = "merge",
                  output_key: Optional[str] = None) -> 'WorkflowBuilder':
        """Add aggregate step"""
        self._finish_current_step()

        self.current_builder = StepBuilder(AggregateStep)
        self.current_builder.params.update({
            "source_steps": source_steps,
            "aggregation_type": aggregation_type,
            "output_key": output_key
        })

        return self

    def custom(self,
               execute_handler: Callable,
               compensate_handler: Optional[Callable] = None) -> 'WorkflowBuilder':
        """Add custom step"""
        self._finish_current_step()

        self.current_builder = StepBuilder(CustomStep)
        self.current_builder.params.update({
            "execute_handler": execute_handler,
            "compensate_handler": compensate_handler
        })

        return self

    def _finish_current_step(self) -> None:
        """Finish building current step and add to workflow"""
        if self.current_builder:
            step = self.current_builder.build()
            self.steps.append(step)
            self.current_builder = None

    def build(self) -> WorkflowDefinition:
        """Build the workflow definition"""
        self._finish_current_step()

        definition = WorkflowDefinition(
            workflow_id=self.workflow_id,
            name=self.name,
            steps=self.steps
        )

        # Apply configuration
        for key, value in self.config.items():
            if hasattr(definition, key):
                setattr(definition, key, value)

        return definition


class WorkflowDSL:
    """DSL for loading workflows from YAML/JSON"""

    @staticmethod
    def from_yaml(yaml_content: str) -> WorkflowDefinition:
        """Load workflow from YAML"""
        data = yaml.safe_load(yaml_content)
        return WorkflowDSL._build_from_dict(data)

    @staticmethod
    def from_json(json_content: str) -> WorkflowDefinition:
        """Load workflow from JSON"""
        data = json.loads(json_content)
        return WorkflowDSL._build_from_dict(data)

    @staticmethod
    def from_file(file_path: str) -> WorkflowDefinition:
        """Load workflow from file (YAML or JSON)"""
        with open(file_path, 'r') as f:
            content = f.read()

        if file_path.endswith(('.yaml', '.yml')):
            return WorkflowDSL.from_yaml(content)
        elif file_path.endswith('.json'):
            return WorkflowDSL.from_json(content)
        else:
            # Try to detect format
            try:
                return WorkflowDSL.from_json(content)
            except json.JSONDecodeError:
                return WorkflowDSL.from_yaml(content)

    @staticmethod
    def _build_from_dict(data: Dict[str, Any]) -> WorkflowDefinition:
        """Build workflow from dictionary"""
        builder = WorkflowBuilder(
            workflow_id=data.get('id'),
            name=data.get('name')
        )

        # Apply workflow configuration
        if 'description' in data:
            builder.with_description(data['description'])

        if 'version' in data:
            builder.with_version(data['version'])

        if 'timeout' in data:
            builder.with_timeout(data['timeout'])

        if 'compensation_strategy' in data:
            builder.with_compensation_strategy(
                CompensationStrategy(data['compensation_strategy'])
            )

        if 'tags' in data:
            builder.with_tags(*data['tags'])

        if 'metadata' in data:
            builder.with_metadata(data['metadata'])

        # Build steps
        for step_data in data.get('steps', []):
            step = WorkflowDSL._build_step(step_data)
            builder.steps.append(step)

        return builder.build()

    @staticmethod
    def _build_step(step_data: Dict[str, Any]) -> WorkflowStep:
        """Build step from dictionary"""
        step_type = step_data.get('type', 'custom')
        step_id = step_data.get('id', f"step_{uuid4().hex[:8]}")

        # Common parameters
        common_params = {
            'step_id': step_id,
            'name': step_data.get('name'),
            'description': step_data.get('description'),
            'timeout': timedelta(seconds=step_data['timeout']) if 'timeout' in step_data else None
        }

        # Build retry policy
        if 'retry' in step_data:
            retry_data = step_data['retry']
            common_params['retry_policy'] = RetryPolicy(
                strategy=RetryStrategy(retry_data.get('strategy', 'exponential')),
                max_attempts=retry_data.get('max_attempts', 3),
                initial_delay=retry_data.get('initial_delay', 1.0)
            )

        # Build specific step type
        if step_type == 'message':
            return MessageStep(
                **common_params,
                destination=step_data['destination'],
                action=step_data['action'],
                data_template=step_data.get('data', {}),
                wait_for_response=step_data.get('wait_for_response', True)
            )

        elif step_type == 'parallel':
            sub_steps = [
                WorkflowDSL._build_step(s)
                for s in step_data.get('steps', [])
            ]
            return ParallelStep(
                **common_params,
                steps=sub_steps,
                wait_strategy=step_data.get('wait_strategy', 'all')
            )

        elif step_type == 'conditional':
            return ConditionalStep(
                **common_params,
                condition=step_data['condition'],
                if_true=WorkflowDSL._build_step(step_data['if_true']) if 'if_true' in step_data else None,
                if_false=WorkflowDSL._build_step(step_data['if_false']) if 'if_false' in step_data else None
            )

        elif step_type == 'loop':
            return LoopStep(
                **common_params,
                items_source=step_data['items_source'],
                loop_variable=step_data['loop_variable'],
                body=WorkflowDSL._build_step(step_data['body']),
                max_concurrent=step_data.get('max_concurrent', 1)
            )

        elif step_type == 'wait':
            duration = None
            if 'duration' in step_data:
                duration = timedelta(seconds=step_data['duration'])

            return WaitStep(
                **common_params,
                duration=duration,
                until_condition=step_data.get('until_condition')
            )

        elif step_type == 'transform':
            return TransformStep(
                **common_params,
                transformations=step_data.get('transformations', {})
            )

        else:
            # Default to custom step
            return CustomStep(
                **common_params,
                execute_handler=lambda ctx: None,  # Would need to load actual handler
                compensate_handler=lambda ctx: None
            )


# Example workflow definitions

def create_example_workflow() -> WorkflowDefinition:
    """Example: Create a data processing workflow using the builder"""

    return (WorkflowBuilder("data_processing", "Data Processing Workflow")
            .with_description("Process incoming data through validation, transformation, and storage")
            .with_timeout(timedelta(minutes=30))
            .with_compensation_strategy(CompensationStrategy.BACKWARD)
            .with_tags("data", "processing", "etl")

            # Step 1: Validate input data
            .message("validator_agent", "validate_data", {
        "schema": "data_schema_v1",
        "strict_mode": True
    })
            .with_name("Validate Input")
            .with_retry(max_attempts=3)
            .with_timeout(60)

            # Step 2: Transform data in parallel
            .parallel(
        WorkflowBuilder()
        .message("transformer_agent", "normalize_data", {
            "format": "canonical"
        })
        .with_name("Normalize Data"),

        WorkflowBuilder()
        .message("enrichment_agent", "enrich_data", {
            "sources": ["source1", "source2"]
        })
        .with_name("Enrich Data")
    )

            # Step 3: Conditional processing based on data quality
            .if_condition(
        "${data_quality_score} > 0.8",
        WorkflowBuilder()
        .message("ml_agent", "process_high_quality", {
            "model": "advanced_model_v2"
        }),
        WorkflowBuilder()
        .message("review_agent", "flag_for_review", {
            "reason": "low_quality_score"
        })
    )

            # Step 4: Store results
            .message("storage_agent", "save_results", {
        "destination": "data_lake",
        "format": "parquet",
        "partitioning": ["date", "category"]
    })
            .with_name("Store Results")
            .with_compensation(lambda ctx: print("Rollback storage"))

            .build()
            )


# Example YAML workflow
example_yaml_workflow = """
id: order_processing
name: Order Processing Workflow
version: "1.0"
description: Process customer orders through fulfillment
timeout: 1800  # 30 minutes
compensation_strategy: backward
tags:
  - orders
  - fulfillment
  - critical

steps:
  - id: validate_order
    type: message
    name: Validate Order
    destination: order_validator
    action: validate
    data:
      order_id: "${order_id}"
      customer_id: "${customer_id}"
    retry:
      strategy: exponential
      max_attempts: 3
      initial_delay: 1

  - id: check_inventory
    type: message
    name: Check Inventory
    destination: inventory_service
    action: check_availability
    data:
      items: "${order_items}"
    wait_for_response: true
    timeout: 300

  - id: process_payment
    type: conditional
    name: Process Payment
    condition: "${inventory_available} == true"
    if_true:
      type: message
      destination: payment_service
      action: charge_payment
      data:
        amount: "${order_total}"
        payment_method: "${payment_method}"
        customer_id: "${customer_id}"
    if_false:
      type: message
      destination: notification_service
      action: notify_out_of_stock
      data:
        order_id: "${order_id}"
        unavailable_items: "${unavailable_items}"

  - id: fulfill_order
    type: parallel
    name: Fulfill Order
    wait_strategy: all
    steps:
      - type: message
        destination: warehouse_service
        action: pick_and_pack
        data:
          order_id: "${order_id}"
          items: "${order_items}"
          priority: "${order_priority}"

      - type: message
        destination: shipping_service
        action: generate_label
        data:
          order_id: "${order_id}"
          shipping_address: "${shipping_address}"
          shipping_method: "${shipping_method}"

  - id: update_tracking
    type: message
    name: Update Tracking
    destination: tracking_service
    action: create_tracking
    data:
      order_id: "${order_id}"
      carrier: "${shipping_carrier}"
      tracking_number: "${tracking_number}"

  - id: notify_customer
    type: message
    name: Notify Customer
    destination: notification_service
    action: send_order_confirmation
    data:
      customer_email: "${customer_email}"
      order_details: "${order_summary}"
      tracking_info: "${tracking_info}"
"""


def create_advanced_workflow_examples():
    """Create more advanced workflow examples"""

    # Example 1: Machine Learning Pipeline
    ml_pipeline = (WorkflowBuilder("ml_pipeline", "ML Training Pipeline")
                   .with_description("End-to-end machine learning training pipeline")
                   .with_timeout(timedelta(hours=4))

                   # Data preparation
                   .transform(
        dataset_path="${raw_data_path}",
        preprocessed_path="/tmp/preprocessed_${job_id}.parquet"
    )
                   .with_name("Set Data Paths")

                   .message("data_prep_agent", "prepare_dataset", {
        "input_path": "${dataset_path}",
        "output_path": "${preprocessed_path}",
        "preprocessing_config": {
            "normalize": True,
            "handle_missing": "mean",
            "encoding": "one_hot"
        }
    })
                   .with_name("Prepare Dataset")
                   .with_timeout(timedelta(minutes=30))

                   # Feature engineering
                   .message("feature_agent", "engineer_features", {
        "input_path": "${preprocessed_path}",
        "feature_config": "${feature_engineering_config}"
    })
                   .with_name("Engineer Features")

                   # Model training with hyperparameter search
                   .for_each(
        items_source="${hyperparameter_sets}",
        loop_variable="hp_config",
        body=WorkflowBuilder()
        .message("training_agent", "train_model", {
            "dataset": "${preprocessed_path}",
            "model_type": "${model_type}",
            "hyperparameters": "${hp_config}",
            "experiment_id": "${experiment_id}_${hp_config_index}"
        })
        .with_retry(max_attempts=2)
    )
                   .with_name("Hyperparameter Search")

                   # Select best model
                   .aggregate(
        source_steps=["hyperparameter_search[*]"],
        aggregation_type="custom",
        output_key="best_model"
    )
                   .with_name("Select Best Model")

                   # Evaluate model
                   .message("evaluation_agent", "evaluate_model", {
        "model_id": "${best_model.model_id}",
        "test_dataset": "${test_dataset_path}",
        "metrics": ["accuracy", "precision", "recall", "f1", "auc"]
    })
                   .with_name("Evaluate Model")

                   # Deploy if performance meets threshold
                   .if_condition(
        "${evaluation_metrics.accuracy} > ${deployment_threshold}",
        WorkflowBuilder()
        .message("deployment_agent", "deploy_model", {
            "model_id": "${best_model.model_id}",
            "deployment_config": "${deployment_config}",
            "endpoints": ["production", "staging"]
        })
        .with_name("Deploy Model"),
        WorkflowBuilder()
        .message("notification_agent", "alert_team", {
            "reason": "model_performance_below_threshold",
            "metrics": "${evaluation_metrics}"
        })
    )

                   .build()
                   )

    # Example 2: Distributed Web Scraping
    web_scraping = (WorkflowBuilder("web_scraping", "Distributed Web Scraping")
                    .with_description("Scrape multiple websites in parallel with rate limiting")
                    .with_compensation_strategy(CompensationStrategy.FORWARD)

                    # Initialize scraping session
                    .message("scraper_coordinator", "init_session", {
        "session_id": "${session_id}",
        "config": "${scraping_config}"
    })
                    .with_name("Initialize Session")

                    # Scrape URLs in parallel with rate limiting
                    .for_each(
        items_source="${url_list}",
        loop_variable="url",
        body=WorkflowBuilder()
        .wait(duration=timedelta(seconds=1))  # Rate limiting
        .message("scraper_agent", "scrape_url", {
            "url": "${url}",
            "selectors": "${selectors}",
            "headers": "${request_headers}"
        })
        .with_retry(
            max_attempts=5,
            strategy=RetryStrategy.EXPONENTIAL,
            initial_delay=2.0
        )
    )
                    .with_name("Scrape URLs")

                    # Process scraped data
                    .transform(
        scraped_data="${loop_results}",
        flattened_data="{{ scraped_data | flatten | unique }}"
    )

                    # Store results
                    .parallel(
        WorkflowBuilder()
        .message("storage_agent", "save_to_database", {
            "data": "${flattened_data}",
            "table": "scraped_content",
            "batch_size": 1000
        }),
        WorkflowBuilder()
        .message("storage_agent", "save_to_s3", {
            "data": "${flattened_data}",
            "bucket": "${s3_bucket}",
            "key": "scraping/${session_id}/data.json"
        })
    )
                    .with_name("Store Results")

                    .build()
                    )

    # Example 3: Event-Driven Data Pipeline
    event_pipeline = (WorkflowBuilder("event_pipeline", "Event Processing Pipeline")
                      .with_description("Process streaming events with complex routing")

                      # Receive and validate event
                      .message("event_receiver", "receive", {
        "event_id": "${event_id}",
        "source": "${event_source}"
    })
                      .with_name("Receive Event")

                      # Route based on event type
                      .custom(
        execute_handler=lambda ctx: ctx.set(
            "event_handler",
            {
                "user_action": "user_analytics_agent",
                "system_event": "system_monitor_agent",
                "transaction": "transaction_processor",
                "error": "error_handler"
            }.get(ctx.get("event_type"), "default_handler")
        )
    )
                      .with_name("Determine Event Handler")

                      # Process event based on type
                      .message("${event_handler}", "process_event", {
        "event": "${event_data}",
        "metadata": "${event_metadata}"
    })
                      .with_name("Process Event")
                      .with_timeout(timedelta(seconds=30))

                      # Check if event requires aggregation
                      .if_condition(
        "${requires_aggregation} == true",
        WorkflowBuilder()
        .wait(
            duration=timedelta(seconds=5),
            until_condition="${aggregation_buffer_size} >= 100"
        )
        .message("aggregation_agent", "aggregate_events", {
            "event_type": "${event_type}",
            "window": "5_seconds"
        })
    )

                      # Persist processed event
                      .message("persistence_agent", "store_event", {
        "processed_event": "${processed_event}",
        "storage_type": "${storage_preference}"
    })
                      .with_compensation(
        lambda ctx: print(f"Rollback event {ctx.get('event_id')}")
    )

                      .build()
                      )

    return {
        "ml_pipeline": ml_pipeline,
        "web_scraping": web_scraping,
        "event_pipeline": event_pipeline
    }


class WorkflowLibrary:
    """Library of reusable workflow patterns"""

    @staticmethod
    def retry_with_backoff(
            step: WorkflowStep,
            max_attempts: int = 5,
            backoff_factor: float = 2.0,
            max_delay: float = 300.0
    ) -> WorkflowStep:
        """Add retry with exponential backoff to a step"""
        step.retry_policy = RetryPolicy(
            strategy=RetryStrategy.EXPONENTIAL,
            max_attempts=max_attempts,
            initial_delay=1.0,
            max_delay=max_delay,
            backoff_factor=backoff_factor
        )
        return step

    @staticmethod
    def circuit_breaker_pattern(
            primary_step: WorkflowStep,
            fallback_step: WorkflowStep,
            failure_threshold: int = 3
    ) -> ConditionalStep:
        """Implement circuit breaker pattern"""
        return ConditionalStep(
            step_id=f"circuit_breaker_{primary_step.step_id}",
            condition=lambda ctx: ctx.get(f"{primary_step.step_id}_failure_count", 0) < failure_threshold,
            if_true=primary_step,
            if_false=fallback_step
        )

    @staticmethod
    def fan_out_fan_in(
            scatter_step: WorkflowStep,
            process_steps: List[WorkflowStep],
            gather_step: WorkflowStep
    ) -> List[WorkflowStep]:
        """Fan-out/fan-in pattern for distributed processing"""
        return [
            scatter_step,
            ParallelStep(
                step_id="fan_out_processing",
                steps=process_steps,
                wait_strategy="all"
            ),
            gather_step
        ]

    @staticmethod
    def saga_pattern(
            steps: List[Tuple[WorkflowStep, WorkflowStep]]
    ) -> WorkflowDefinition:
        """Implement saga pattern with compensation"""
        forward_steps = [step for step, _ in steps]

        # Create workflow with automatic compensation
        workflow = WorkflowDefinition(
            workflow_id="saga_workflow",
            name="Saga Pattern Workflow",
            steps=forward_steps,
            compensation_strategy=CompensationStrategy.BACKWARD
        )

        # Set compensation handlers
        for step, compensation in steps:
            if compensation:
                step.compensation_handler = compensation.execute

        return workflow


class WorkflowValidator:
    """Validate workflow definitions"""

    @staticmethod
    def validate(workflow: WorkflowDefinition) -> List[str]:
        """Validate workflow and return list of errors"""
        errors = []

        # Basic validation from workflow
        errors.extend(workflow.validate())

        # Check for circular dependencies
        errors.extend(WorkflowValidator._check_circular_deps(workflow))

        # Validate step references
        errors.extend(WorkflowValidator._validate_references(workflow))

        # Check resource requirements
        errors.extend(WorkflowValidator._check_resources(workflow))

        return errors

    @staticmethod
    def _check_circular_deps(workflow: WorkflowDefinition) -> List[str]:
        """Check for circular dependencies in workflow"""
        errors = []
        # Implementation would check for cycles in step dependencies
        return errors

    @staticmethod
    def _validate_references(workflow: WorkflowDefinition) -> List[str]:
        """Validate all step references exist"""
        errors = []
        step_ids = {step.step_id for step in workflow.steps}

        for step in workflow.steps:
            if hasattr(step, 'source_steps'):
                for source_id in step.source_steps:
                    if source_id not in step_ids:
                        errors.append(f"Step {step.step_id} references unknown step: {source_id}")

        return errors

    @staticmethod
    def _check_resources(workflow: WorkflowDefinition) -> List[str]:
        """Check if workflow resource requirements are reasonable"""
        errors = []

        # Check total timeout
        total_timeout = sum(
            step.timeout.total_seconds()
            for step in workflow.steps
            if step.timeout
        )

        if total_timeout > workflow.timeout.total_seconds():
            errors.append(
                f"Sum of step timeouts ({total_timeout}s) exceeds workflow timeout ({workflow.timeout.total_seconds()}s)")

        return errors


# Workflow template functions for common patterns

def create_etl_workflow(
        source_config: Dict[str, Any],
        transform_config: Dict[str, Any],
        destination_config: Dict[str, Any]
) -> WorkflowDefinition:
    """Create an ETL workflow from configuration"""

    return (WorkflowBuilder("etl_workflow", "ETL Workflow")
            .with_description("Extract, Transform, Load data workflow")

            # Extract
            .message("extractor_agent", "extract_data", source_config)
            .with_name("Extract Data")
            .with_retry(max_attempts=3)

            # Transform
            .message("transformer_agent", "transform_data", transform_config)
            .with_name("Transform Data")

            # Validate
            .message("validator_agent", "validate_data", {
        "schema": destination_config.get("schema"),
        "rules": destination_config.get("validation_rules", [])
    })
            .with_name("Validate Data")

            # Load
            .message("loader_agent", "load_data", destination_config)
            .with_name("Load Data")
            .with_compensation(
        lambda ctx: print(f"Rollback load for job {ctx.get('job_id')}")
    )

            .build()
            )


def create_approval_workflow(
        approval_levels: List[str],
        timeout_per_level: timedelta = timedelta(hours=24)
) -> WorkflowDefinition:
    """Create multi-level approval workflow"""

    builder = WorkflowBuilder("approval_workflow", "Multi-Level Approval")

    for i, approver in enumerate(approval_levels):
        builder.message(
            f"{approver}_agent",
            "request_approval",
            {
                "level": i + 1,
                "approver": approver,
                "timeout": timeout_per_level.total_seconds()
            }
        ).with_name(f"Level {i + 1} Approval: {approver}")
        .with_timeout(timeout_per_level)

        # Check approval status
        builder.if_condition(
            f"${{approval_status_{i}}} == 'rejected'",
            WorkflowBuilder()
            .message("notification_agent", "notify_rejection", {
                "level": i + 1,
                "approver": approver,
                "reason": f"${{rejection_reason_{i}}}"
            })
            .custom(lambda ctx: ctx.set("workflow_result", "rejected"))
        )

    return builder.build()