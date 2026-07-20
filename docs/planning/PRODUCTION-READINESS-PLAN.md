# RedNode-OS Production Readiness Plan

> Your friend's 12 items fall into 3 categories:
> **Build now** (code), **Write now** (specs/docs), **Build later** (v1.0+).
>
> His core message: "Stop adding cognitive modules. Start making the
> existing ones measurable, testable, auditable, and replaceable."

---

## What to build as v0.38.0 (code)

### 1. Cognitive Benchmark Suite — `benchmark.rs` (NEW)

The single most impactful thing missing. Without benchmarks, we can't
answer "did this release make RedNode smarter?"

```rust
pub struct BenchmarkSuite {
    pub scenarios: Vec<BenchmarkScenario>,
    pub results: Vec<BenchmarkRun>,
    pub baseline: Option<BenchmarkRun>,
}

pub struct BenchmarkScenario {
    pub id: String,
    pub name: String,
    pub category: BenchmarkCategory, // IncidentResponse, Research, Coding, etc.
    pub description: String,
    pub input: serde_json::Value,
    pub expected_outcome: String,
    pub scoring_criteria: Vec<String>,
}

pub struct BenchmarkRun {
    pub id: String,
    pub version: String,         // "0.37.0"
    pub timestamp: DateTime<Utc>,
    pub scenario_scores: Vec<ScenarioScore>,
    pub overall_score: f32,
    pub comparison_to_baseline: Option<f32>, // +/- vs baseline
}
```

Categories: IncidentResponse, Research, Coding, InfraManagement,
Conversation, LongTermPlanning, MultiAgent, SecurityResponse.

API: `POST /benchmark/run`, `GET /benchmark/results`, `GET /benchmark/compare`

### 2. Cognitive Replay Engine — deepen `experience_replay.rs`

Add `replay_decision()` that reconstructs the full thought chain:

```rust
pub struct DecisionReplay {
    pub decision_id: String,
    pub thought_timeline: Vec<ThoughtStep>,
    pub events_involved: Vec<CognitiveEvent>,
    pub memory_retrievals: Vec<String>,
    pub planner_reasoning: String,
    pub simulation_result: Option<String>,
    pub debate_result: Option<String>,
    pub execution_result: String,
    pub final_outcome: String,
}

pub struct ThoughtStep {
    pub timestamp: DateTime<Utc>,
    pub module: String,
    pub action: String,
    pub reasoning: String,
}
```

### 3. Failure & Recovery — deepen existing modules

Add to each cognitive module:

```rust
pub struct ModuleHealth {
    pub module_name: String,
    pub status: ModuleStatus,        // Healthy, Degraded, Failed, Recovering
    pub last_error: Option<String>,
    pub recovery_attempts: u32,
    pub last_healthy: DateTime<Utc>,
}
```

Add `health()` and `recover()` functions to key modules.

### 4. Cognitive Security Audit — deepen `immune.rs`

Add cognitive-layer security checks:
- Can memory be poisoned by prompt injection?
- Can goals be manipulated by malicious plugins?
- Can trust be artificially inflated?
- Can debates be biased by bad data?

```rust
pub async fn audit_cognitive_security() -> CognitiveSecurityReport {
    // Check memory for injection patterns
    // Check goals for unauthorized modifications
    // Check trust scores for anomalous jumps
    // Check debate history for bias patterns
}
```

### 5. Digital Legacy expansion — deepen `digital_legacy.rs`

Add:
- Architecture evolution log (what changed and why)
- Major decision records
- Failed experiment records
- Lessons learned

---

## What to write as v0.38.0 (documentation)

### 6. Cognitive API Specification — `docs/specs/COGNITIVE-API.md`

Every module's interface documented:
- Inputs / Outputs
- Events emitted / consumed
- State shape
- Lifecycle hooks
- Permissions
- Metrics exposed
- Version

### 7. Formal Architecture Specification — `docs/specs/ARCHITECTURE-SPEC.md`

Living specification:
- Cognitive lifecycle (the full observe→think→act→reflect cycle)
- Module responsibilities (single-sentence per module)
- Allowed dependencies (which modules can call which)
- Event taxonomy (all 23+ cognitive event types)
- Decision hierarchy (who decides what)
- State transitions (module status lifecycle)

### 8. Cognitive Testing Framework — `docs/specs/TESTING-FRAMEWORK.md`

Test cognition as software:
- Does reflection improve planning? (measure before/after)
- Does trust affect decisions correctly?
- Can it recover from contradictory memories?
- Does simulation reduce failures?
- Does debate outperform single planner?

Defines test scenarios, expected behaviors, pass criteria.

### 9. Developer SDK Specification — `docs/SDK.md`

Defines how external developers will write:
- Cognitive modules
- Planners
- Debate roles
- Sensors
- Tools
- Simulators
- Reflection strategies

Without modifying the core. Based on the cognitive bus event system.

### 10. Cognitive Maturity Model — `docs/MATURITY-MODEL.md`

```
Level 1: Reactive         — responds to commands
Level 2: Context-aware    — understands environment
Level 3: Goal-driven      — pursues objectives
Level 4: Self-reflective  — evaluates own performance
Level 5: Self-improving   — optimizes own processes
Level 6: Predictive       — anticipates future states
Level 7: Collaborative    — works with peers
Level 8: Adaptive         — redesigns own architecture
Level 9: Strategic        — long-term multi-horizon planning
Level 10: Persistent Cognitive Entity — continuous, self-aware, evolving
```

Current assessment: Level 7 (collaborative) with elements of Level 8-9.

---

## What to defer to v1.0+ (future)

### 11. Autonomous Software Engineering

RedNode maintaining its own codebase. Requires:
- Real compiler access
- Git integration
- CI/CD pipeline
- This is deployment infrastructure, not cognitive architecture

### 12. Full Cognitive Observatory Dashboard

Rich visualization of attention flow, reasoning chains, debate graphs.
Requires frontend development (Next.js dashboard pages). The API data
is already there — this is a UI project.

---

## Execution Plan

v0.38.0 should contain:
- 1 new module: `benchmark.rs`
- 4 deepened modules: experience_replay, immune, digital_legacy, + recovery hooks
- 5 specification documents
- Total: ~500 lines code + ~2000 lines documentation
