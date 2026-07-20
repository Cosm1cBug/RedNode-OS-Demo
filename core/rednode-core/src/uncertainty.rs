// RedNode-OS — Uncertainty Engine
//
// Every belief and prediction carries a confidence score.
// When uncertainty is high, RedNode asks for confirmation.
//
// API:
//   GET  /uncertainty         — current uncertainty levels
//   POST /uncertainty/assess  — assess uncertainty for an action

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static UNCERTAINTY: once_cell::sync::Lazy<Arc<RwLock<UncertaintyState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(UncertaintyState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintyAssessment {
    pub id: String,
    pub action: String,
    pub confidence: f32,
    pub uncertainty_sources: Vec<UncertaintySource>,
    pub recommendation: UncertaintyAction,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintySource { pub factor: String, pub impact: f32, pub reducible: bool, pub suggestion: String }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UncertaintyAction { Proceed, DoubleCheck, AskUser, Defer, Abort }

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A record tracking predicted confidence vs actual outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationRecord {
    pub predicted_confidence: f32,
    pub actual_success: bool,
    pub timestamp: DateTime<Utc>,
}

/// Aggregated calibration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationStats {
    pub total_predictions: u64,
    pub calibration_error: f32,
    pub overconfidence_rate: f32,
    pub underconfidence_rate: f32,
}

pub struct UncertaintyState {
    pub assessments: VecDeque<UncertaintyAssessment>,
    pub total: u64,
    pub ask_threshold: f32,
    pub abort_threshold: f32,
    /// Calibration tracking: predicted confidence vs actual outcome
    pub calibration_records: VecDeque<CalibrationRecord>,
    pub calibration: CalibrationStats,
}

impl Default for UncertaintyState {
    fn default() -> Self {
        Self {
            assessments: VecDeque::with_capacity(100), total: 0,
            ask_threshold: 0.4, abort_threshold: 0.2,
            calibration_records: VecDeque::with_capacity(500),
            calibration: CalibrationStats { total_predictions: 0, calibration_error: 0.0, overconfidence_rate: 0.0, underconfidence_rate: 0.0 },
        }
    }
}

fn gen_id() -> String { format!("unc_{}", chrono::Utc::now().timestamp_millis()) }

/// Assess uncertainty for a proposed action
pub async fn assess(action: &str, data_confidence: f32, model_confidence: f32, source_trust: f32) -> UncertaintyAssessment {
    let mut state = UNCERTAINTY.write().await;
    let composite = (data_confidence * 0.4 + model_confidence * 0.3 + source_trust * 0.3).clamp(0.0, 1.0);

    let mut sources = Vec::new();
    if data_confidence < 0.5 { sources.push(UncertaintySource { factor: "Insufficient data".into(), impact: 0.5 - data_confidence, reducible: true, suggestion: "Gather more observations".into() }); }
    if model_confidence < 0.5 { sources.push(UncertaintySource { factor: "Low model confidence".into(), impact: 0.5 - model_confidence, reducible: true, suggestion: "Try alternative model or strategy".into() }); }
    if source_trust < 0.5 { sources.push(UncertaintySource { factor: "Untrusted source".into(), impact: 0.5 - source_trust, reducible: true, suggestion: "Verify with trusted source".into() }); }

    let recommendation = if composite >= 0.8 { UncertaintyAction::Proceed }
        else if composite >= 0.6 { UncertaintyAction::DoubleCheck }
        else if composite >= state.ask_threshold { UncertaintyAction::AskUser }
        else if composite >= state.abort_threshold { UncertaintyAction::Defer }
        else { UncertaintyAction::Abort };

    let assessment = UncertaintyAssessment { id: gen_id(), action: action.into(), confidence: composite, uncertainty_sources: sources, recommendation, timestamp: Utc::now() };
    if state.assessments.len() >= 100 { state.assessments.pop_front(); }
    state.assessments.push_back(assessment.clone());
    state.total += 1;
    assessment
}

pub async fn get_recent(limit: usize) -> Vec<UncertaintyAssessment> { UNCERTAINTY.read().await.assessments.iter().rev().take(limit).cloned().collect() }
pub async fn should_ask_user(confidence: f32) -> bool { UNCERTAINTY.read().await.ask_threshold > confidence }

/// Record a calibration data point — predicted confidence vs actual outcome
pub async fn record_calibration(predicted_confidence: f32, actual_success: bool) {
    let mut state = UNCERTAINTY.write().await;
    if state.calibration_records.len() >= 500 { state.calibration_records.pop_front(); }
    state.calibration_records.push_back(CalibrationRecord {
        predicted_confidence, actual_success, timestamp: Utc::now(),
    });
    state.calibration.total_predictions += 1;

    // Recompute calibration stats from last 100 records
    let recent: Vec<_> = state.calibration_records.iter().rev().take(100).collect();
    if !recent.is_empty() {
        let mut total_error = 0.0f32;
        let mut overconfident = 0u64;
        let mut underconfident = 0u64;
        for r in &recent {
            let actual = if r.actual_success { 1.0 } else { 0.0 };
            total_error += (r.predicted_confidence - actual).abs();
            if r.predicted_confidence > 0.7 && !r.actual_success { overconfident += 1; }
            if r.predicted_confidence < 0.3 && r.actual_success { underconfident += 1; }
        }
        state.calibration.calibration_error = total_error / recent.len() as f32;
        state.calibration.overconfidence_rate = overconfident as f32 / recent.len() as f32;
        state.calibration.underconfidence_rate = underconfident as f32 / recent.len() as f32;
    }
}

/// Get calibration stats
pub async fn get_calibration() -> CalibrationStats {
    UNCERTAINTY.read().await.calibration.clone()
}

pub async fn persist() { let s = UNCERTAINTY.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO uncertainty_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM uncertainty_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<UncertaintyState>(json) { let mut s = UNCERTAINTY.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS uncertainty_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = UncertaintyState::default(); assert_eq!(s.ask_threshold, 0.4); } #[test] fn test_action_eq() { assert_eq!(UncertaintyAction::Proceed, UncertaintyAction::Proceed); } }
