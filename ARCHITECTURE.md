# RedNode-OS Architecture — v0.34.0

## System Overview

```
Human Intent
    ↓
Attention Engine (score by urgency/importance/security/goal/novelty)
    ↓
Intent Engine (vague → specific clarification)
    ↓
Context Engine (activate relevant memories/goals/procedures)
    ↓
Constitution Check (7 immutable articles)
    ↓
Ethics Evaluation (7 core values)
    ↓
Uncertainty Assessment (confidence scoring)
    ↓
Multi-Agent Debate (Planner → Critic → Security → Economy → Governance)
    ↓
Value Estimation (benefit/risk/cost/time/learning/goal scoring)
    ↓
LLM Planner (Ollama Qwen2.5) / GOAP Fallback / Keyword Fallback
    ↓
Simulation Engine (predict outcome before executing)
    ↓
Governance Policy Check (Block/Warn/Log/Approve)
    ↓
Cognitive Load Check (capacity available?)
    ↓
Coordinator (parallel execution, state caching, circuit breaker)
    ↓
Agents (18 via NATS) → Sandboxed Executor (firejail/seccomp)
    ↓
Verification Engine (independently validate outcome)
    ↓
Explainability (record why, alternatives, evidence, assumptions)
    ↓
Audit Log (SHA-256 chain) → Reflection → Episodic Memory → Goal Progress
    ↓
Cognitive Metrics Update → Trust Score Update → Economy Cost Recording
```

## Rust Modules (67)

### Core Infrastructure (20 modules — original + config)
| Module | Lines | Purpose |
|---|---|---|
| `api.rs` | 2,035 | 150 API endpoints (Axum router) |
| `auth.rs` | 107 | Bearer token authentication middleware |
| `bus.rs` | 63 | NATS JetStream message bus |
| `config.rs` | 365 | Web-based configuration management |
| `coordinator.rs` | 271 | Plan execution — parallel dispatch, approval gates, circuit breaker |
| `events.rs` | 137 | WebSocket event bus (broadcast channel) |
| `executor.rs` | 512 | Sandboxed tool execution (firejail/seccomp) |
| `init.rs` | 473 | NixOS PID1 init process |
| `intent_router.rs` | 210 | Intent → coordinator dispatch |
| `memory.rs` | 1,218 | PostgreSQL + Qdrant + knowledge graph |
| `memory_optimizer.rs` | 207 | Memory compaction and optimization |
| `notifications.rs` | 274 | Smart notification delivery |
| `pii.rs` | 249 | PII detection and redaction |
| `planner.rs` | 429 | LLM planning with dynamic tool context |
| `goap.rs` | 278 | Goal-oriented action planning fallback |
| `predict.rs` | 343 | Predictive maintenance (linear regression) |
| `pipelines.rs` | 615 | Cross-agent autonomous workflows |
| `security.rs` | 241 | Risk assessment, deny patterns, approval logic |
| `sentience.rs` | 995 | Self-model, 5 drives, goal generation, memory consolidation |
| `evolution.rs` | 444 | Autonomous tool discovery and creation |

### Consciousness & Identity (6 modules — v0.10–v0.17)
| Module | Lines | Purpose |
|---|---|---|
| `consciousness.rs` | 553 | Persistent mind state, 10-second awareness loop |
| `goals.rs` | 393 | Long-term objectives, sub-goals, auto-detection |
| `identity.rs` | 535 | Purpose, principles, boundaries, self-model |
| `constitution.rs` | 476 | 7 immutable articles, amendment process |
| `personality.rs` | 288 | Tunable communication style for LLM prompts |
| `reflection.rs` | 543 | Task/periodic/daily self-assessment |

### Cognition (16 modules — v0.11–v0.34)
| Module | Lines | Purpose |
|---|---|---|
| `attention.rs` | 384 | Signal scoring, priority queue, focus management |
| `cognitive_load.rs` | 254 | Intelligence scheduling, LLM throttling, capacity |
| `intent_engine.rs` | 134 | Vague-to-specific intent clarification |
| `context_engine.rs` | 135 | Task-relevant information activation |
| `meta_reasoning.rs` | 318 | Evaluates reasoning quality, strategy recommendations |
| `debate.rs` | 246 | Multi-perspective consensus before high-impact decisions |
| `simulation.rs` | 237 | Predict outcomes before executing |
| `creativity.rs` | 134 | Divergent brainstorming separate from analysis |
| `scientific.rs` | 90 | Structured research pipeline |
| `curiosity.rs` | 461 | Bounded autonomous exploration |
| `explainability.rs` | 68 | Records why every decision was made |
| `uncertainty.rs` | 80 | Confidence scoring, knows when to ask |
| `value_estimator.rs` | 67 | Action scoring by benefit/risk/cost/learning/goal |
| `model_orchestrator.rs` | 83 | Selects best local model per task type |
| `verification.rs` | 67 | Independently validates task outcomes |
| `experience_replay.rs` | 84 | Replays past situations for improvement |

### Memory & Knowledge (8 modules — v0.11–v0.33)
| Module | Lines | Purpose |
|---|---|---|
| `episodic_memory.rs` | 425 | Experiences, procedures, working memory |
| `world_model.rs` | 993 | Infrastructure graph, dependency/impact analysis |
| `time_intel.rs` | 1,096 | Unified scheduler, deadlines, timezone awareness |
| `distillation.rs` | 420 | Compress experience into runbooks/playbooks |
| `provenance.rs` | 69 | Tracks origin of every knowledge item |
| `knowledge_lifecycle.rs` | 68 | Created → Verified → Deprecated → Archived |
| `forgetting.rs` | 82 | Prevents uncontrolled memory growth |
| `memory_safety.rs` | 71 | Integrity validation with checksums |

### Safety & Governance (5 modules — v0.14–v0.20)
| Module | Lines | Purpose |
|---|---|---|
| `governance.rs` | 515 | Policy engine (Block/Warn/Log/Approve) |
| `immune.rs` | 452 | Prompt injection, credential leaks, agent trust |
| `ethics.rs` | 192 | 7 core values guiding ambiguous decisions |
| `trust.rs` | 138 | Dynamic trust scores per information source |
| `economy.rs` | 390 | Resource budgets and cost tracking |

### Evolution & Growth (6 modules — v0.15–v0.33)
| Module | Lines | Purpose |
|---|---|---|
| `plugins.rs` | 329 | Installable capability packages |
| `capability_registry.rs` | 97 | Self-aware skill tracking with staleness |
| `dreaming.rs` | 173 | Idle-time consolidation and optimization |
| `evolution_sandbox.rs` | 86 | Tests self-modifications before applying |
| `digital_legacy.rs` | 82 | Inheritable state across upgrades |
| `adaptive_arch.rs` | 62 | Proposes structural improvements |

### Operational State (2 modules — v0.30)
| Module | Lines | Purpose |
|---|---|---|
| `emotional_state.rs` | 83 | Confidence/stress/curiosity/satisfaction/fatigue |
| `cognitive_metrics.rs` | 101 | 14 intelligence metrics tracked over time |

### Distribution (4 modules — v0.16–v0.34)
| Module | Lines | Purpose |
|---|---|---|
| `collective.rs` | 132 | Multi-instance collaboration |
| `collective_governance.rs` | 126 | Voting, leader election, consensus |
| `hal.rs` | 115 | Hardware profiling and failover |
| `twin.rs` | 480 | Infrastructure "what if" simulation |

## Configuration

The web dashboard is the **single source of truth** for all configuration.
No `.env` editing required.

```
Browser → http://rednode:3000/setup     (first boot)
Browser → http://rednode:3000/settings  (ongoing)
    → CNS API → config.json (encrypted secrets)
    → NATS broadcast → agents re-fetch automatically
```

## Database Tables (53)

### Core Infrastructure
| Table | Purpose |
|---|---|
| `audit_log` | SHA-256 hash-chained action log |
| `security_events` | Security incident records |
| `approvals` | Pending human approval requests |
| `documents` | Ingested documents for RAG |
| `kg_entities` | Knowledge graph entities |
| `kg_relationships` | Knowledge graph relationships |

### Consciousness & Identity
| Table | Purpose |
|---|---|
| `consciousness_state` | Persistent mind state |
| `goal_store` | Long-term objectives |
| `identity_store` | Identity profile |
| `constitution_store` | Constitutional articles and amendments |
| `personality_store` | Communication style settings |
| `reflection_store` | Self-assessment history |

### Cognition
| Table | Purpose |
|---|---|
| `attention_store` | Signal queue and focus state |
| `cognitive_load_store` | Cognitive slot allocation |
| `intent_engine_store` | Intent clarification history |
| `context_engine_store` | Context activation records |
| `meta_reasoning_store` | Reasoning quality analysis |
| `debate_store` | Multi-agent debate history |
| `simulation_store` | Simulation results |
| `creativity_store` | Creative ideas and sessions |
| `science_store` | Experiments and results |
| `explainability_store` | Decision explanations |
| `uncertainty_store` | Confidence assessments |
| `value_estimator_store` | Action value estimates |
| `model_orchestrator_store` | Model selection history |
| `verification_store` | Task verification results |
| `experience_replay_store` | Experience replay results |

### Memory & Knowledge
| Table | Purpose |
|---|---|
| `episodic_memory_store` | Episodes, procedures, working memory |
| `world_model_store` | Infrastructure graph |
| `time_intel_store` | Scheduled events, patterns, deadlines |
| `curiosity_store` | Exploration state and discoveries |
| `distillation_store` | Distilled knowledge documents |
| `provenance_store` | Knowledge origin and verification |
| `knowledge_lifecycle_store` | Knowledge stage tracking |
| `forgetting_store` | Forgetting sweep records |
| `memory_safety_store` | Integrity check results |

### Safety & Governance
| Table | Purpose |
|---|---|
| `governance_store` | Policies and violation records |
| `immune_store` | Threat detection state |
| `ethics_store` | Ethical evaluation history |
| `trust_store` | Trust scores per source |
| `economy_store` | Resource spending history |

### Evolution & Growth
| Table | Purpose |
|---|---|
| `plugin_store` | Installed plugins |
| `capability_registry_store` | Capability tracking |
| `dreaming_store` | Dream session history |
| `evolution_sandbox_store` | Sandbox test results |
| `digital_legacy_store` | Legacy export/import records |
| `adaptive_arch_store` | Architecture proposal history |

### Operational State
| Table | Purpose |
|---|---|
| `emotional_state_store` | Emotional state snapshots |
| `cognitive_metrics_store` | Intelligence metrics history |

### Distribution
| Table | Purpose |
|---|---|
| `collective_store` | Peer instances and sync history |
| `collective_governance_store` | Voting and election records |
| `hal_store` | Hardware profiles and failover records |
| `twin_store` | Digital twin simulation history |
