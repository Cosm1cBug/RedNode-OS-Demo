# RedNode-OS Architecture — v0.24.0

## System Overview

```
Human Intent
    ↓
Constitution Check (7 immutable articles)
    ↓
Ethics Evaluation (7 core values)
    ↓
Multi-Agent Debate (Planner → Critic → Security → Economy → Governance)
    ↓
LLM Planner (Ollama Qwen2.5) / GOAP Fallback / Keyword Fallback
    ↓
Simulation Engine (predict outcome before executing)
    ↓
Governance Policy Check (Block/Warn/Log/Approve)
    ↓
Coordinator (parallel execution, state caching, circuit breaker)
    ↓
Agents (18 via NATS) → Sandboxed Executor (firejail/seccomp)
    ↓
Audit Log (SHA-256 chain) → Reflection → Episodic Memory → Goal Progress
```

## Rust Modules (47)

### Core Infrastructure
| Module | Lines | Purpose |
|---|---|---|
| `api.rs` | ~1,900 | 120 API endpoints (Axum router) |
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

### Consciousness & Identity (Phase 3-4)
| Module | Lines | Purpose |
|---|---|---|
| `consciousness.rs` | 540 | Persistent mind state, 10-second awareness loop |
| `goals.rs` | 379 | Long-term objectives, sub-goals, auto-detection |
| `identity.rs` | 535 | Purpose, principles, boundaries, self-model |
| `constitution.rs` | 476 | 7 immutable articles, amendment process |
| `personality.rs` | 288 | Tunable communication style for LLM prompts |
| `reflection.rs` | 543 | Task/periodic/daily self-assessment |

### Cognition (Phase 3-4)
| Module | Lines | Purpose |
|---|---|---|
| `meta_reasoning.rs` | 318 | Evaluates reasoning quality, strategy recommendations |
| `debate.rs` | 246 | Multi-perspective consensus before high-impact decisions |
| `simulation.rs` | 237 | Predict outcomes before executing |
| `creativity.rs` | 134 | Divergent brainstorming separate from analysis |
| `scientific.rs` | 90 | Structured research pipeline |
| `curiosity.rs` | 461 | Bounded autonomous exploration |

### Memory & Knowledge (Phase 3-4)
| Module | Lines | Purpose |
|---|---|---|
| `episodic_memory.rs` | 425 | Experiences, procedures, working memory |
| `world_model.rs` | 993 | Infrastructure graph, dependency/impact analysis |
| `time_intel.rs` | 1,096 | Unified scheduler, deadlines, timezone awareness |
| `distillation.rs` | 420 | Compress experience into runbooks/playbooks |

### Safety & Governance (Phase 3-4)
| Module | Lines | Purpose |
|---|---|---|
| `governance.rs` | 515 | Policy engine (Block/Warn/Log/Approve) |
| `immune.rs` | 452 | Prompt injection, credential leaks, agent trust |
| `ethics.rs` | 192 | 7 core values guiding ambiguous decisions |
| `trust.rs` | 138 | Dynamic trust scores per information source |
| `economy.rs` | 390 | Resource budgets and cost tracking |

### Evolution & Growth (Phase 3-4)
| Module | Lines | Purpose |
|---|---|---|
| `plugins.rs` | 329 | Installable capability packages |
| `capability_registry.rs` | 97 | Self-aware skill tracking with staleness |
| `dreaming.rs` | 173 | Idle-time consolidation and optimization |

### Distribution (Phase 4)
| Module | Lines | Purpose |
|---|---|---|
| `collective.rs` | 132 | Multi-instance collaboration |
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

## Database Tables (27)

| Table | Purpose |
|---|---|
| `audit_log` | SHA-256 hash-chained action log |
| `security_events` | Security incident records |
| `approvals` | Pending human approval requests |
| `consciousness_state` | Persistent mind state |
| `goal_store` | Long-term objectives |
| `world_model_store` | Infrastructure graph |
| `time_intel_store` | Scheduled events, patterns, deadlines |
| `personality_store` | Communication style settings |
| `reflection_store` | Self-assessment history |
| `curiosity_store` | Exploration state and discoveries |
| `distillation_store` | Distilled knowledge documents |
| `economy_store` | Resource spending history |
| `immune_store` | Threat detection state |
| `governance_store` | Policies and violation records |
| `plugin_store` | Installed plugins |
| `twin_store` | Simulation history |
| `identity_store` | Identity profile |
| `constitution_store` | Constitutional articles and amendments |
| `meta_reasoning_store` | Reasoning quality analysis |
| `episodic_memory_store` | Episodes, procedures, working memory |
| `simulation_store` | Simulation results |
| `debate_store` | Debate history |
| `trust_store` | Trust scores per source |
| `ethics_store` | Ethical evaluation history |
| `creativity_store` | Creative ideas and sessions |
| `science_store` | Experiments and results |
| `capability_registry_store` | Capability tracking |
| `dreaming_store` | Dream session history |
| `collective_store` | Peer instances and sync history |
| `hal_store` | Hardware profiles and failover records |
