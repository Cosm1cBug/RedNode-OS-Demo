# RedNode-OS Cognitive API Specification

> Every module's interface defined. Makes modules replaceable.

## Module Interface Contract

Every cognitive module MUST expose:

```
Module: <name>
Purpose: <one sentence>
Inputs: <what data it consumes>
Outputs: <what data it produces>
Events Emitted: <CognitiveEventType variants>
Events Consumed: <what bus events it reacts to>
State: <struct name persisted to PostgreSQL>
Lifecycle: init_table() → restore() → tick()/process() → persist()
Permissions: <what other modules it may call>
Metrics: <what it reports to cognitive_metrics>
Version: <semver>
```

## Core Modules

### consciousness
- **Purpose:** Persistent mind state and awareness loop
- **Inputs:** Attention queue, cognitive bus events
- **Outputs:** MindState (focus, awareness, tasks, learnings)
- **Events Emitted:** AttentionShifted
- **Events Consumed:** ThreatDetected, GoalCompleted, TaskFailed, ReflectionCompleted
- **State:** MindState → consciousness_state table
- **Lifecycle:** init_table → restore → tick(10s) → persist
- **Permissions:** reads attention, cognitive_bus, emotional_state, time_intel
- **Metrics:** awareness_level, confidence, urgency

### goals
- **Purpose:** Long-term objective management
- **Inputs:** User goals, task completion events
- **Outputs:** GoalStatus, progress, predictions
- **Events Emitted:** GoalUpdated, GoalCompleted
- **State:** Vec<UserGoal> → goal_store table
- **Permissions:** reads consciousness (via bus), calls cognitive_bus::emit
- **Metrics:** goal_completion_rate, active_goal_count

### identity
- **Purpose:** Stable anchor for purpose, principles, boundaries
- **Inputs:** Update requests (gated through constitution)
- **Outputs:** IdentityProfile, system prompt fragment
- **Events Emitted:** IdentityChanged
- **Permissions:** reads constitution (for gate), calls cognitive_bus::emit
- **Gate:** Constitution check required for all updates

### planner
- **Purpose:** Convert intent to executable tool steps
- **Inputs:** Intent string, tool context
- **Outputs:** Vec<PlanStep>
- **Permissions:** reads evolution (tool context), calls Ollama API
- **Metrics:** planning_accuracy, reasoning_latency_ms

### coordinator
- **Purpose:** Execute plans with pre/post hooks
- **Inputs:** Intent, session
- **Pre-hooks:** context_engine, constitution, cognitive_load
- **Post-hooks:** verification, reflection, economy, immune, goals, cognitive_bus
- **Permissions:** reads/writes most modules (orchestrator role)

### immune
- **Purpose:** Threat detection, behavioral baselines, cognitive security
- **Inputs:** System metrics, LLM inputs, agent behavior
- **Outputs:** Threats, health score, security audit
- **Events Emitted:** ThreatDetected, ThreatMitigated
- **Metrics:** health_score, active_threats, agent_trust_scores

### debate
- **Purpose:** Multi-perspective consensus before high-impact decisions
- **Inputs:** Proposed action, context
- **Outputs:** DebateResult with consensus
- **Events Emitted:** DebateCompleted
- **Side effects:** Stores outcome as episodic memory, feeds distillation
- **Roles:** Planner, Critic, Security, Economy, Governance, DevilsAdvocate, Historian, Scientist, Ethicist, Research

### mental_models
- **Purpose:** Internal theories with evidence and predictions
- **Inputs:** Observations, evidence, prediction validations
- **Outputs:** Models with confidence, prediction accuracy
- **Lifecycle:** Hypothetical → Supported → Established → Challenged → Revised → Deprecated

### benchmark
- **Purpose:** Measure cognitive quality across releases
- **Inputs:** Benchmark scenarios
- **Outputs:** BenchmarkRun with scores and comparison to baseline
- **Categories:** IncidentResponse, Research, Infrastructure, Security, Planning, Conversation, LongTermPlanning, MultiAgent

## Event Taxonomy

All cognitive events emitted via `cognitive_bus::emit()`:

| Event | Source Module | Meaning |
|---|---|---|
| ObservationCreated | perception | New input normalized |
| GoalUpdated | goals | Goal progress changed |
| GoalCompleted | goals | Goal finished |
| MemoryStored | memory, distillation, mental_models | Knowledge persisted |
| MemoryRecalled | episodic_memory | Memory retrieved |
| ThreatDetected | immune | Security threat found |
| ThreatMitigated | immune | Threat resolved |
| ReflectionCompleted | reflection | Review cycle done |
| SimulationFinished | simulation | Simulation complete |
| DecisionMade | consciousness, benchmark | Action decided |
| TaskStarted | coordinator | Execution began |
| TaskCompleted | coordinator | Execution succeeded |
| TaskFailed | coordinator | Execution failed |
| AttentionShifted | consciousness | Focus changed |
| TrustChanged | trust | Source trust updated |
| EpisodeRecorded | episodic_memory | Experience stored |
| DreamCycleComplete | dreaming | Consolidation done |
| PolicyViolation | governance | Policy breached |
| CapabilityVerified | capability_registry | Skill confirmed |
| PlanCreated | planner | Plan generated |
| DebateCompleted | debate | Consensus reached |
| EvolutionProposed | evolution | New tool proposed |
| ConfigChanged | config | Setting modified |
| IdentityChanged | identity | Identity modified |
