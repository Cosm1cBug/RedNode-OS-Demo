# RedNode-OS Changelog

## v0.39.0 — Stabilization

### Rust Compile Fixes (8)
- **bus.rs**: Added `get_connection()` — config.rs and evolution.rs require it
- **episodic_memory.rs**: Fixed duplicate `#[derive]` placement on Contradiction/StructuredMemory
- **immune.rs**: Added missing `#[derive]` on ImmuneState, fixed BehavioralBaseline duplicate
- **cognitive_metrics.rs + emotional_state.rs**: Fixed borrow checker — copy scalars before history.push()
- **context_engine.rs**: `{}` → `{:?}` for EpisodeCategory (no Display impl)
- **reflection.rs**: Added Timelike import. **time_intel.rs**: Added TimeZone import
- **sentience.rs**: `pool()` → `crate::memory::pool()`, sysinfo API updated for 0.30

### Agent Audit (16 agents fixed)
- Fixed escaped backticks in 6 agents (home, infra, media, research, surveillance, coding)
- Fixed 30 empty `catch {}` blocks across 11 agents
- Added shell input sanitization to coding-agent and network-agent
- Replaced `git add -A` with `git commit -a` in coding-agent
- Added nats dependency to agents/shared/package.json

### Safety & Reliability
- **/health endpoint**: Now reports NATS + PostgreSQL connection status with degraded flag
- **Dashboard footer**: Fixed "14 Agents · 105 Tools" → "18 Agents · 359 Tools"
- **Startup logging**: Memory, NATS, Executor failures logged instead of silently discarded

## v0.38.0 — Production Readiness (code not yet pushed — deferred)

### Planned
- Benchmark Suite, Decision Replay Engine, Cognitive Security Audit
- 5 specification documents (Cognitive API, Architecture, Testing, SDK, Maturity Model)

## v0.37.0 — Cognitive Depth

### New Module
- **Mental Models**: Internal theories with beliefs, evidence, predictions with validation, revision history

### Cognitive Depth (10 improvements)
- **Counterfactual Thinking**: "What would have happened if..." causal analysis
- **Self-Explanation**: Strategy choice and memory retrieval explain WHY
- **Confidence Calibration**: Predicted vs actual outcome tracking
- **Cognitive Health Report**: Aggregated health score with trends
- **Emergent Skill Inference**: Discover capabilities from combinations
- **Planning Horizons**: Goals span Hours/Days/Weeks/Months/Years
- **Self-Question Generation**: Consciousness asks "What am I ignoring?"
- **Meta-Learning**: Optimize the learning process itself
- **Multi-Speed Cognitive Clock**: Fast(2s)/Medium(10s)/Slow(5min)/Background(1h)
- **Cognitive Compression**: Distillation abstraction levels

## v0.36.0 — Architecture Wiring & Structural Fixes

### Cognitive Bus Integration
- 12 modules emit to cognitive bus, consciousness PULLS from bus
- Coordinator: pre-execution (context, constitution, cognitive load) + post-execution (verification, reflection, economy, trust)
- Identity gated through constitution, evolution runs safety pipeline
- Debates stored as episodic memories + fed to distillation
- Memory auto-indexes episodes, records provenance on ingest
- 8 domain-specific reflection scopes, world model prediction, trust/governance/constitution enhancements

## v0.35.0 — Cognitive Architecture Enhancement

### New Foundation Systems
- **Cognitive Bus**: 23 typed cognitive events, broadcast subscribers
- **Perception Layer**: 10 input modalities normalized
- **Language Engine**: Prompt templates, glossary, summarization, extraction

### Module Deepening (20 modules enhanced)
- Consciousness, Goals, Identity, Episodic Memory, Trust, Debate, Immune, Simulation, Meta-Reasoning, Reflection, Evolution Sandbox, Plugins, World Model, HAL, Creativity, Scientific, Distillation, Dreaming, Curiosity, Collective

## v0.34.0 — Collective Governance
- Voting, leader election, configurable consensus

## v0.33.0 — Digital Legacy + Adaptive Architecture
## v0.32.0 — Experience Replay + Evolution Sandbox
## v0.31.0 — Model Orchestrator + Verification Engine
## v0.30.0 — Emotional State + Cognitive Metrics
## v0.29.0 — Forgetting + Knowledge Lifecycle + Memory Safety
## v0.28.0 — Value Estimator + Provenance Engine
## v0.27.0 — Explainability + Uncertainty Engine
## v0.26.0 — Intent Engine + Context Engine
## v0.25.0 — Attention Engine + Cognitive Load Manager
## v0.24.0 — Mission Control + Documentation
## v0.23.0 — Collective Intelligence + Hardware Abstraction
## v0.22.0 — Capability Registry + Dreaming
## v0.21.0 — Creativity Engine + Scientific Method
## v0.20.0 — Trust Engine + Ethics & Values
## v0.19.0 — Simulation Engine + Multi-Agent Debate
## v0.18.0 — Meta-Reasoning + Episodic Memory
## v0.17.0 — Identity Engine + Constitutional Layer
## v0.16.0 — Digital Twin
## v0.15.0 — Plugin Ecosystem + Multi-Agent Society
## v0.14.0 — Economy + Immune System + Governance
## v0.13.0 — Curiosity Engine + Knowledge Distillation
## v0.12.0 — Personality Engine + Reflection System
## v0.11.0 — World Model + Time Intelligence
## v0.10.0 — Digital Consciousness + Goal Engine
## v0.9.3 — Config Management + Dashboard UI
## v0.9.0 — Tool Expansion + Self-Evolution
