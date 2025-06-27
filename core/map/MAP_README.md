# MAP Core Components:

## Message Types and Models (maple/core/map/models/message.py)

Core message structure with headers, payloads, and security context
Support for different message types, priorities, and delivery modes
Flexible destination routing (specific agent, service, broadcast, multicast)


## Routing Engine (maple/core/map/routing/engine.py)

Intelligent agent selection based on capabilities, load, and performance
Multiple routing strategies (load-balanced, priority-based)
Service discovery and capability indexing
Health monitoring and metrics tracking


## Transport Layer (maple/core/map/transport/base.py)

Multiple transport protocols (HTTP, WebSocket)
Delivery guarantees (at-most-once, at-least-once, exactly-once)
Retry mechanisms and circuit breakers
Batch message support


## Workflow Orchestration (maple/core/map/orchestration/workflow.py)

Complex multi-step workflow execution
Parallel, conditional, and loop step types
Compensation/rollback capabilities
Workflow state management and persistence
Fluent API for workflow creation

---

These components form the backbone of the MAP protocol, enabling reliable, scalable communication between agents in the MAPLE ecosystem.