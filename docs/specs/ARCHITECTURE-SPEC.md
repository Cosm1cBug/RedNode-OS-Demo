# RedNode-OS Formal Architecture Specification

> Living specification: cognitive lifecycle, module responsibilities,
> allowed dependencies, decision hierarchy, state transitions.

## Cognitive Lifecycle

Every cognitive cycle follows this sequence:

```
Observe (perception)
  ↓
Perceive (normalize input)
  ↓
Attend (score priority, allocate focus)
  ↓
Activate Context (retrieve relevant memories/goals)
  ↓
Update Consciousness (process bus events, update awareness)
  ↓
Consult Identity (check principles and boundaries)
  ↓
Retrieve Memory (episodic, semantic, procedural)
  ↓
Update Mental Models (revise theories with new evidence)
  ↓
Review Goals (check progress, predict completion)
  ↓
Reason (select strategy via meta-reasoning)
  ↓
Debate (multi-perspective evaluation if high-risk)
  ↓
Plan (decompose intent into executable steps)
  ↓
Simulate (predict outcome before executing)
  ↓
Govern (check policies, constitution, ethics)
  ↓
Execute (dispatch to agents via coordinator)
  ↓
Verify (independently validate outcome)
  ↓
Reflect (assess success/failure, extract learnings)
  ↓
Learn (update knowledge, skills, models)
  ↓
Distill (compress experience into wisdom)
  ↓
Persist (save state to PostgreSQL)
  ↓
Dream (background consolidation when idle)
  ↓
Repeat
```

Not every cycle hits every step. The cognitive bus allows modules to
skip steps that aren't relevant to the current situation.

## Multi-Speed Cognitive Clocks

| Clock | Interval | Modules |
|---|---|---|
| Fast | 2 seconds | attention, cognitive_bus |
| Medium | 10 seconds | consciousness, emotional_state, episodic_memory, world_model, economy |
| Slow | 5 minutes | reflection, cognitive_metrics, capability_registry, time_intel, immune |
| Background | 1 hour | dreaming, curiosity, distillation, forgetting |

## Module Responsibility Matrix

Each module has exactly ONE responsibility:

| Module | Single Responsibility |
|---|---|
| consciousness | Maintain awareness state |
| identity | Define who RedNode is |
| constitution | Enforce immutable principles |
| ethics | Guide ambiguous decisions |
| governance | Enforce mutable policies |
| goals | Track long-term objectives |
| attention | Prioritize signals |
| cognitive_load | Manage thinking capacity |
| context_engine | Activate relevant information |
| perception | Normalize inputs |
| planner | Convert intent to steps |
| coordinator | Execute plans with hooks |
| verification | Validate outcomes |
| reflection | Self-assess performance |
| meta_reasoning | Evaluate reasoning quality |
| memory | Store/retrieve knowledge |
| episodic_memory | Store experiences |
| mental_models | Track theories |
| trust | Score source reliability |
| immune | Detect threats |
| economy | Track resource costs |
| benchmark | Measure cognitive quality |

## Allowed Dependencies

Tier 1 (infrastructure — no cognitive deps):
  memory, events, bus, executor, auth, config

Tier 2 (perception — reads Tier 1 only):
  perception, attention, cognitive_bus, pii

Tier 3 (cognition — reads Tier 1-2):
  consciousness, context_engine, planner, goals, identity,
  trust, uncertainty, emotional_state

Tier 4 (decision — reads Tier 1-3):
  coordinator, debate, simulation, governance, constitution, ethics

Tier 5 (learning — reads all tiers):
  reflection, meta_reasoning, episodic_memory, mental_models,
  distillation, forgetting, curiosity, dreaming, evolution

Tier 6 (meta — reads all tiers):
  cognitive_metrics, benchmark, explainability, verification

## Decision Hierarchy

When modules disagree:

1. Constitution (immutable) — absolute veto
2. Ethics (values) — strong guidance
3. Governance (policies) — enforcement
4. Debate (consensus) — multi-perspective
5. Planner (strategy) — execution plan
6. Consciousness (awareness) — coordination

Constitution > Ethics > Governance > Debate > Planning > Execution
