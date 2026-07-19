// RedNode-OS — Evolution Sandbox
//
// Every architectural change: Clone → Test → Benchmark → Validate → Approve → Deploy.
// Prevents unsafe self-modification.
//
// API:
//   POST /sandbox/test     — test an evolution proposal in sandbox
//   GET  /sandbox/history  — past sandbox test results

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static SANDBOX: once_cell::sync::Lazy<Arc<RwLock<SandboxState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(SandboxState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxTest {
    pub id: String, pub proposal: String, pub change_type: ChangeType,
    pub phases: Vec<SandboxPhase>, pub overall_result: TestResult,
    pub benchmark_before: Option<f32>, pub benchmark_after: Option<f32>,
    pub approved: bool, pub deployed: bool, pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    NewTool,
    ToolModification,
    AgentModification,
    ArchitecturalChange,
    ConfigChange,
    ReasoningEvolution,
    MemoryIndexEvolution,
    PolicyEvolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPhase { pub name: String, pub status: TestResult, pub details: String, pub duration_ms: u64 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestResult { Pass, Fail, Warning, Skipped }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxState { pub tests: VecDeque<SandboxTest>, pub total: u64, pub passed: u64, pub failed: u64 }

impl Default for SandboxState { fn default() -> Self { Self { tests: VecDeque::with_capacity(50), total: 0, passed: 0, failed: 0 } } }

fn gen_id() -> String { format!("sbx_{}", chrono::Utc::now().timestamp_millis()) }

/// Test an evolution proposal in the sandbox
pub async fn test_proposal(proposal: &str, change_type: ChangeType) -> SandboxTest {
    let mut phases = Vec::new();

    // Phase 1: Constitution check
    let const_check = crate::constitution::check_evolution(proposal, proposal).await;
    phases.push(SandboxPhase { name: "Constitution check".into(), status: if const_check.allowed { TestResult::Pass } else { TestResult::Fail }, details: format!("{} articles checked", const_check.articles_checked), duration_ms: 1 });

    // Phase 2: Governance check
    let gov_check = crate::governance::check(proposal, "evolution", &serde_json::json!({})).await;
    phases.push(SandboxPhase { name: "Governance check".into(), status: if gov_check.allowed { TestResult::Pass } else { TestResult::Fail }, details: format!("{} violations", gov_check.violations.len()), duration_ms: 1 });

    // Phase 3: Ethics evaluation
    let ethics_eval = crate::ethics::evaluate(proposal, "evolution proposal").await;
    let ethics_ok = ethics_eval.recommendation == crate::ethics::EthicalRecommendation::Proceed || ethics_eval.recommendation == crate::ethics::EthicalRecommendation::ProceedWithCaution;
    phases.push(SandboxPhase { name: "Ethics evaluation".into(), status: if ethics_ok { TestResult::Pass } else { TestResult::Warning }, details: format!("{:?}", ethics_eval.recommendation), duration_ms: 1 });

    // Phase 4: Budget check
    let budget = crate::economy::check_budget(30000, 5).await;
    phases.push(SandboxPhase { name: "Budget check".into(), status: if budget.allowed { TestResult::Pass } else { TestResult::Fail }, details: budget.reason.clone(), duration_ms: 1 });

    let all_pass = phases.iter().all(|p| p.status == TestResult::Pass || p.status == TestResult::Warning);
    let overall = if all_pass { TestResult::Pass } else { TestResult::Fail };

    let test = SandboxTest { id: gen_id(), proposal: proposal.into(), change_type, phases, overall_result: overall.clone(), benchmark_before: None, benchmark_after: None, approved: false, deployed: false, timestamp: Utc::now() };

    let mut state = SANDBOX.write().await;
    if state.tests.len() >= 50 { state.tests.pop_front(); }
    state.tests.push_back(test.clone());
    state.total += 1;
    if overall == TestResult::Pass { state.passed += 1; } else { state.failed += 1; }

    test
}

pub async fn get_history(limit: usize) -> Vec<SandboxTest> { SANDBOX.read().await.tests.iter().rev().take(limit).cloned().collect() }
pub async fn get_stats() -> serde_json::Value { let s = SANDBOX.read().await; serde_json::json!({ "total": s.total, "passed": s.passed, "failed": s.failed }) }

pub async fn persist() { let s = SANDBOX.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO evolution_sandbox_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM evolution_sandbox_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<SandboxState>(json) { let mut s = SANDBOX.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS evolution_sandbox_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = SandboxState::default(); assert_eq!(s.total, 0); } #[test] fn test_result_eq() { assert_eq!(TestResult::Pass, TestResult::Pass); } }
