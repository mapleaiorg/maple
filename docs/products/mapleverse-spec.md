# Mapleverse Product Specification

**Version**: 1.0.0
**Status**: Draft
**Product Owner**: MapleAI Intelligence Inc.

## Executive Summary

Mapleverse is a high-throughput swarm orchestration platform built on MAPLE, designed to coordinate millions of AI agents in real-time for simulation, gaming, and distributed computing workloads.

## Product Vision

Enable planetary-scale multi-agent coordination where throughput and responsiveness take precedence over individual agent accountability, while maintaining system-wide observability.

## Target Use Cases

1. **Massively Multiplayer Simulations**: Virtual worlds with millions of NPCs
2. **Swarm Robotics Coordination**: Coordinating drone fleets and robot swarms
3. **Distributed Computing**: Agent-based parallel processing
4. **Real-time Strategy Games**: AI opponent coordination
5. **Traffic & Logistics Simulation**: Large-scale movement optimization

## Architecture

### System Overview
```
┌────────────────────────────────────────────────────────────────────┐
│                         Mapleverse Platform                        │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────┐  │
│  │   Swarm     │  │   World     │  │  Behavior   │  │  Event    │  │
│  │ Coordinator │  │   State     │  │   Engine    │  │  Router   │  │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └─────┬─────┘  │
│         │                │                │               │        │
│         └────────────────┼────────────────┼───────────────┘        │
│                          │                │                        │
│                    ┌─────▼────────────────▼──────┐                 │
│                    │    Mapleverse Runtime       │                 │
│                    │    (mapleverse-pack)        │                 │
│                    └─────────────┬───────────────┘                 │
│                                  │                                 │
├──────────────────────────────────┼─────────────────────────────────┤
│                                  │                                 │
│                    ┌─────────────▼───────────────┐                 │
│                    │       PALM Runtime          │                 │
│                    │  Control │ Policy │ Health  │                 │
│                    └─────────────────────────────┘                 │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. Swarm Coordinator

Manages collections of agents as unified swarms.
```rust
pub struct SwarmCoordinator {
    swarms: HashMap<SwarmId, Swarm>,
    topology: SwarmTopology,
    load_balancer: SwarmLoadBalancer,
}

pub struct Swarm {
    pub id: SwarmId,
    pub agents: Vec<AgentId>,
    pub behavior: SwarmBehavior,
    pub bounds: SpatialBounds,
    pub max_size: usize,
}

pub enum SwarmBehavior {
    Flocking { cohesion: f64, separation: f64, alignment: f64 },
    Formation { pattern: FormationPattern },
    Distributed { task_distribution: TaskDistribution },
    Custom { behavior_id: String },
}
```

#### 2. World State Manager

Maintains distributed world state with eventual consistency.
```rust
pub struct WorldState {
    pub regions: HashMap<RegionId, RegionState>,
    pub global_time: SimulationTime,
    pub tick_rate: u32,
}

pub struct RegionState {
    pub id: RegionId,
    pub bounds: SpatialBounds,
    pub entities: Vec<EntityState>,
    pub version: u64,
    pub last_sync: Instant,
}

impl WorldState {
    pub async fn tick(&mut self) -> TickResult;
    pub async fn sync_region(&mut self, region: RegionId) -> SyncResult;
    pub async fn partition(&mut self, strategy: PartitionStrategy) -> Vec<RegionId>;
}
```

#### 3. Behavior Engine

Executes agent behaviors at scale.
```rust
pub struct BehaviorEngine {
    behavior_registry: BehaviorRegistry,
    executor: BehaviorExecutor,
    batch_size: usize,
}

impl BehaviorEngine {
    pub async fn execute_batch(&self, agents: &[AgentId]) -> Vec<BehaviorResult>;
    pub async fn register_behavior(&mut self, behavior: Behavior) -> BehaviorId;
}

pub struct Behavior {
    pub id: BehaviorId,
    pub priority: u8,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
    pub cooldown: Duration,
}
```

#### 4. Event Router

High-throughput event distribution.
```rust
pub struct EventRouter {
    topics: HashMap<Topic, Vec<Subscriber>>,
    buffer: RingBuffer<Event>,
    batch_dispatcher: BatchDispatcher,
}

impl EventRouter {
    pub async fn publish(&self, event: Event) -> PublishResult;
    pub async fn subscribe(&mut self, topic: Topic, handler: Handler) -> SubscriptionId;
    pub async fn batch_publish(&self, events: Vec<Event>) -> BatchPublishResult;
}
```

## API Specification

### REST API

#### Swarm Management
```yaml
# Create Swarm
POST /api/v1/swarms
Content-Type: application/json

{
  "name": "drone-fleet-alpha",
  "behavior": {
    "type": "flocking",
    "cohesion": 0.5,
    "separation": 0.8,
    "alignment": 0.6
  },
  "bounds": {
    "min": [0, 0, 0],
    "max": [1000, 1000, 100]
  },
  "initial_size": 1000,
  "max_size": 10000
}

Response: 201 Created
{
  "id": "swarm-abc123",
  "name": "drone-fleet-alpha",
  "agent_count": 1000,
  "status": "active"
}

# Scale Swarm
POST /api/v1/swarms/{id}/scale
{
  "target_size": 5000,
  "strategy": "gradual",
  "rate": 100  // agents per second
}

# Get Swarm Status
GET /api/v1/swarms/{id}

Response: 200 OK
{
  "id": "swarm-abc123",
  "agent_count": 5000,
  "health": "healthy",
  "metrics": {
    "tps": 150000,
    "avg_latency_ms": 2.5,
    "coupling_intensity": 0.45
  }
}
```

#### World State
```yaml
# Get Region State
GET /api/v1/world/regions/{id}

Response: 200 OK
{
  "id": "region-001",
  "entity_count": 25000,
  "tick": 1234567,
  "last_sync": "2026-02-01T10:00:00Z"
}

# Trigger World Tick
POST /api/v1/world/tick
{
  "regions": ["region-001", "region-002"],
  "delta_time": 0.016
}
```

### WebSocket Streaming API
```javascript
// Connect to event stream
const ws = new WebSocket('wss://api.mapleverse.io/v1/stream');

// Subscribe to swarm events
ws.send(JSON.stringify({
  action: 'subscribe',
  topics: ['swarm.drone-fleet-alpha.*', 'world.tick']
}));

// Receive events
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  // { topic: 'swarm.drone-fleet-alpha.position', payload: [...] }
};
```

### gRPC API
```protobuf
syntax = "proto3";

package mapleverse.v1;

service SwarmService {
  rpc CreateSwarm(CreateSwarmRequest) returns (Swarm);
  rpc ScaleSwarm(ScaleSwarmRequest) returns (ScaleResponse);
  rpc StreamSwarmEvents(StreamRequest) returns (stream SwarmEvent);
  rpc BatchUpdateAgents(stream AgentUpdate) returns (BatchUpdateResponse);
}

message CreateSwarmRequest {
  string name = 1;
  SwarmBehavior behavior = 2;
  SpatialBounds bounds = 3;
  uint32 initial_size = 4;
  uint32 max_size = 5;
}

message SwarmEvent {
  string swarm_id = 1;
  string event_type = 2;
  bytes payload = 3;
  uint64 timestamp = 4;
}
```

## Performance Requirements

| Metric | Requirement |
|--------|-------------|
| Max Agents | 10,000,000 concurrent |
| Tick Rate | 60 ticks/second |
| Event Throughput | 1,000,000 events/second |
| Agent Update Latency | < 10ms p99 |
| Region Sync Latency | < 50ms p99 |
| Swarm Scale Time | < 30s for 10k agents |

## Deployment Architecture

### Kubernetes Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mapleverse-coordinator
spec:
  replicas: 5
  selector:
    matchLabels:
      app: mapleverse-coordinator
  template:
    spec:
      containers:
      - name: coordinator
        image: mapleai/mapleverse-coordinator:latest
        resources:
          requests:
            cpu: "4"
            memory: "16Gi"
          limits:
            cpu: "8"
            memory: "32Gi"
        env:
        - name: PALM_PLATFORM
          value: "mapleverse"
        - name: MAPLEVERSE_MAX_SWARMS
          value: "1000"
        - name: MAPLEVERSE_TICK_RATE
          value: "60"
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: mapleverse-world-state
spec:
  replicas: 10
  serviceName: world-state
  template:
    spec:
      containers:
      - name: world-state
        image: mapleai/mapleverse-world-state:latest
        volumeMounts:
        - name: state-storage
          mountPath: /data
  volumeClaimTemplates:
  - metadata:
      name: state-storage
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 100Gi
```

### Resource Scaling
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: mapleverse-coordinator-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: mapleverse-coordinator
  minReplicas: 3
  maxReplicas: 50
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Pods
    pods:
      metric:
        name: swarm_agent_count
      target:
        type: AverageValue
        averageValue: "100000"
```

## Operational Procedures

### Scaling a Swarm
```bash
# CLI command
mapleverse swarm scale drone-fleet-alpha --target 50000 --rate 1000

# Expected output
Scaling swarm drone-fleet-alpha: 5000 -> 50000
Progress: [████████████████████░░░░░░░░░░] 67% (33500/50000)
Estimated completion: 17 seconds
```

### Emergency Procedures
```bash
# Pause all swarms
mapleverse swarm pause --all

# Reduce world tick rate
mapleverse world tick-rate --set 10

# Drain region
mapleverse region drain region-001 --target region-002
```

## Monitoring & Observability

### Key Metrics

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `mapleverse_swarm_agent_count` | Agents per swarm | N/A |
| `mapleverse_tick_duration_seconds` | Tick processing time | > 16ms |
| `mapleverse_event_queue_depth` | Pending events | > 100,000 |
| `mapleverse_region_sync_lag_seconds` | Cross-region sync delay | > 1s |
| `mapleverse_behavior_execution_errors` | Failed behavior executions | > 1% |

### Dashboards

- **Swarm Overview**: Agent counts, health status, spatial distribution
- **Performance**: Tick rate, latency percentiles, throughput
- **Resources**: CPU, memory, network utilization per component
- **Events**: Event flow, queue depths, processing rates

## Security Considerations

### Access Control
```yaml
# RBAC configuration
roles:
  swarm-admin:
    permissions:
      - swarm:create
      - swarm:scale
      - swarm:delete
      - world:read

  swarm-operator:
    permissions:
      - swarm:read
      - swarm:scale
      - world:read

  observer:
    permissions:
      - swarm:read
      - world:read
      - events:subscribe
```

### Network Security

- All API endpoints require TLS 1.3
- Internal communication uses mTLS
- Event streams authenticated via JWT

## Roadmap

### Phase 1: Core Platform (Q1 2026)
- [x] Basic swarm management
- [x] World state distribution
- [x] Event routing
- [ ] WebSocket streaming

### Phase 2: Scale (Q2 2026)
- [ ] 1M+ agent support
- [ ] Cross-region distribution
- [ ] Advanced load balancing

### Phase 3: Features (Q3 2026)
- [ ] Visual debugging tools
- [ ] Behavior marketplace
- [ ] Analytics platform
