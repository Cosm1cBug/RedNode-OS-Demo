# RedNode-OS Changelog

## v0.24.0 — Mission Control + Documentation

- **Mission Control API**: Single endpoint aggregating consciousness, goals, world, time, security, economy, trust, collective, dreaming, and hardware into one response
- Full documentation update reflecting all v0.10.0–v0.24.0 changes
- 47 Rust modules, 120 API endpoints, 27 PostgreSQL tables

## v0.23.0 — Collective Intelligence + Hardware Abstraction

- **Collective Intelligence**: Multi-instance collaboration — peer discovery, knowledge sync, task delegation, trust-based routing
- **Hardware Abstraction Layer**: Hardware profiling, failover targets, cluster-wide view, state migration support

## v0.22.0 — Capability Registry + Dreaming

- **Capability Registry**: Self-aware capability tracking with version, confidence, benchmarks, staleness detection
- **Dreaming/Offline Consolidation**: Background optimization during idle — memory consolidation, knowledge compression, hypothesis generation

## v0.21.0 — Creativity Engine + Scientific Method

- **Creativity Engine**: Domain-specific divergent brainstorming separate from analytical reasoning
- **Scientific Method Engine**: Observe → Hypothesis → Experiment → Measure → Analyze → Conclude → Store pipeline

## v0.20.0 — Trust Engine + Ethics & Values

- **Trust Engine**: Dynamic trust scores per information source with EMA tracking and trend detection
- **Ethics & Values Layer**: 7 core values guiding ambiguous decisions — privacy, safety, transparency, reversibility, minimality, explainability, data ownership

## v0.19.0 — Simulation Engine + Multi-Agent Debate

- **Simulation Engine**: Simulate plan execution, deployments, security responses, code changes with predicted outcomes and failure modes
- **Multi-Agent Debate**: Planner → Critic → Security → Economy → Governance consensus before high-impact decisions

## v0.18.0 — Meta-Reasoning + Episodic Memory

- **Meta-Reasoning Engine**: Evaluates reasoning quality — strategy fitness, plan efficiency, tool selection scoring, improvement suggestions
- **Episodic Memory**: Separates experiences/skills/working context — distinct recall for "what happened" vs "how to" vs "what am I doing now"

## v0.17.0 — Identity Engine + Constitutional Layer

- **Identity Engine**: Purpose, principles, boundaries, self-model, capability map — the stable anchor preventing evolution drift
- **Constitutional Layer**: 7 immutable articles (preserve user control, never conceal, never falsify, prefer reversible, protect privacy, require approval for destructive ops, protect constitutional integrity) with multi-step amendment process

## v0.16.0 — Digital Twin

- **Digital Twin**: "What if" simulation against world model — machine/service offline, power outage, VLAN removal, blast radius analysis, recovery sequences

## v0.15.0 — Plugin Ecosystem + Multi-Agent Society

- **Plugin Ecosystem**: Installable capability packages with manifest format, permission checks, CRUD
- **Multi-Agent Society**: agent.ts rewrite with reputation (EMA), confidence, peer review, work queues, metrics heartbeat

## v0.14.0 — Economy + Immune System + Governance

- **Internal Economy**: Per-task CPU/RAM/GPU/API cost tracking, daily budgets, per-task caps
- **Digital Immune System**: Prompt injection detection (13 patterns), credential leak scanning (12 patterns), agent trust scoring
- **Governance Layer**: Policy engine with Block/Warn/Log/Approve enforcement, rate limiting, time-range blocking

## v0.13.0 — Curiosity Engine + Knowledge Distillation

- **Curiosity Engine**: Bounded autonomous exploration with configurable topics, sources, daily limits, novelty threshold
- **Knowledge Distillation**: Compresses experience into runbooks, playbooks, best practices, troubleshooting guides

## v0.12.0 — Personality Engine + Reflection System

- **Personality Engine**: Tunable communication style (depth, verbosity, formality, proactivity, humor) injected into LLM prompts
- **Reflection System**: Task/periodic/daily self-assessment feeding learnings back into consciousness and suggesting automations

## v0.11.0 — World Model + Time Intelligence

- **World Model**: Infrastructure graph — machines, services, VLANs, containers, cameras, IoT with dependency/impact analysis, snapshots, diffs
- **Time Intelligence**: Unified scheduler with Daily/Weekly/Monthly/NthWeekday/Interval schedules, deadlines, IST timezone, Patch Tuesday

## v0.10.0 — Digital Consciousness + Goal Engine

- **Digital Consciousness**: Persistent mind state with focus tracking, awareness indicators, 10-second tick loop, PostgreSQL persistence
- **Goal Engine**: Long-term objectives with sub-goals, auto-detected contributions, progress tracking

## v0.9.3 — Config Management + Dashboard UI

- Web-based configuration (config.rs + config-loader.ts + setup wizard + settings page)
- API endpoints for config integrated into CNS
- ROADMAP.md rewritten

## v0.9.2 — Config Architecture

- config.rs: serve config via API
- config-loader.ts: agents fetch from CNS
- NATS broadcast on config change

## v0.9.1 — Agent Bug Fixes + Voice + ISO

- Fixed 37 commented-out consts, dangling text, broken try/catch across agents
- Voice systemd services (STT, TTS, wake word)
- Live ISO profile (DHCP, auto-login, SSH)
- Headless branding (Plymouth, TTY banner)
- GUI/voice toggles, DHCP networking

## v0.9.0 — Tool Expansion + Self-Evolution

- 359 tools across 18 agents (up from 164)
- Self-evolution engine (evolution.rs)
- Dynamic planner loads tools from tools.json at runtime
- helpers.ts for agent handler implementations
