// RedNode-OS — Trust Engine
//
// Every information source gets a dynamic trust score (0.0–1.0)
// that changes based on historical reliability.
//
// Tracked sources: GitHub, CVE feeds, user input, local sensors,
// web search results, plugins, agents, LLM outputs, APIs.
//
// Trust affects:
//   - How much weight information gets in decisions
//   - Whether results are double-checked
//   - Alert thresholds for suspicious data
//
// API:
//   GET  /trust             — all trust scores
//   GET  /trust/:source     — specific source trust
//   POST /trust/:source     — manually adjust trust

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

static TRUST: once_cell::sync::Lazy<Arc<RwLock<TrustState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(TrustState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    pub source: String,
    pub category: SourceCategory,
    pub score: f32,
    pub interactions: u64,
    pub accurate_count: u64,
    pub inaccurate_count: u64,
    pub last_interaction: Option<DateTime<Utc>>,
    pub trend: TrustTrend,
    pub notes: Vec<String>,
    /// Multi-axis trust breakdown
    pub axes: TrustAxes,
    /// Federated trust from collective peers (if available)
    pub community_trust: Option<f32>,
}

/// Multi-dimensional trust — different aspects of reliability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAxes {
    /// Does the source produce technically correct results?
    pub technical: f32,
    /// Does the source behave predictably and consistently?
    pub behavioral: f32,
    /// Does the source avoid security risks?
    pub security: f32,
    /// Is the source historically reliable over time?
    pub historical: f32,
}

impl Default for TrustAxes {
    fn default() -> Self {
        Self { technical: 0.5, behavioral: 0.5, security: 0.5, historical: 0.5 }
    }
}

impl TrustAxes {
    /// Weighted composite score from all axes
    pub fn composite(&self) -> f32 {
        (self.technical * 0.3 + self.behavioral * 0.2 + self.security * 0.3 + self.historical * 0.2)
            .clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SourceCategory { User, Agent, LLM, WebSearch, CveFeed, GitHub, Api, Plugin, Sensor, System, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrustTrend { Improving, Stable, Declining, New }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustState {
    pub scores: HashMap<String, TrustScore>,
    pub total_interactions: u64,
}

impl Default for TrustState {
    fn default() -> Self {
        let mut scores = HashMap::new();
        let defaults = vec![
            ("user", SourceCategory::User, 1.0), ("ollama", SourceCategory::LLM, 0.7),
            ("searxng", SourceCategory::WebSearch, 0.6), ("cve_feed", SourceCategory::CveFeed, 0.85),
            ("github", SourceCategory::GitHub, 0.8), ("pihole", SourceCategory::Api, 0.9),
            ("truenas", SourceCategory::Api, 0.9), ("home_assistant", SourceCategory::Api, 0.85),
            ("local_sensors", SourceCategory::Sensor, 0.95),
        ];
        for (name, cat, score) in defaults {
            scores.insert(name.into(), TrustScore {
                source: name.into(), category: cat, score, interactions: 0,
                accurate_count: 0, inaccurate_count: 0, last_interaction: None,
                trend: TrustTrend::New, notes: vec![],
                axes: TrustAxes { technical: score, behavioral: score, security: score, historical: score },
                community_trust: None,
            });
        }
        Self { scores, total_interactions: 0 }
    }
}

pub async fn get_all() -> HashMap<String, TrustScore> { TRUST.read().await.scores.clone() }
pub async fn get_score(source: &str) -> Option<TrustScore> { TRUST.read().await.scores.get(source).cloned() }
pub async fn get_trust_value(source: &str) -> f32 { TRUST.read().await.scores.get(source).map(|s| s.score).unwrap_or(0.5) }

/// Record an interaction outcome (accurate or not)
pub async fn record_interaction(source: &str, category: SourceCategory, accurate: bool) {
    let mut state = TRUST.write().await;
    let entry = state.scores.entry(source.into()).or_insert(TrustScore {
        source: source.into(), category, score: 0.5, interactions: 0,
        accurate_count: 0, inaccurate_count: 0, last_interaction: None,
        trend: TrustTrend::New, notes: vec![],
        axes: TrustAxes::default(), community_trust: None,
    });
    entry.interactions += 1;
    if accurate { entry.accurate_count += 1; } else { entry.inaccurate_count += 1; }
    entry.last_interaction = Some(Utc::now());

    let old_score = entry.score;
    // Exponential moving average
    let new_data = if accurate { 1.0 } else { 0.0 };
    entry.score = entry.score * 0.95 + new_data * 0.05;
    entry.score = entry.score.clamp(0.05, 1.0);

    entry.trend = if entry.score > old_score + 0.01 { TrustTrend::Improving }
        else if entry.score < old_score - 0.01 { TrustTrend::Declining }
        else { TrustTrend::Stable };

    state.total_interactions += 1;
}

/// Manually set trust score
pub async fn set_trust(source: &str, score: f32, note: Option<String>) {
    let mut state = TRUST.write().await;
    if let Some(entry) = state.scores.get_mut(source) {
        entry.score = score.clamp(0.05, 1.0);
        if let Some(n) = note { entry.notes.push(n); }
    }
}

pub async fn persist() {
    let state = TRUST.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) { Ok(v) => v, Err(_) => return };
        let _ = sqlx::query("INSERT INTO trust_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await;
    }
}
pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM trust_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<TrustState>(json) { let mut s = TRUST.write().await; *s = r; } }
    }
}
pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query("CREATE TABLE IF NOT EXISTS trust_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default() {
        let s = TrustState::default();
        assert!(s.scores.contains_key("user"));
        assert_eq!(s.scores["user"].score, 1.0);
        assert_eq!(s.scores["user"].axes.technical, 1.0);
        assert!(s.scores["user"].community_trust.is_none());
    }
    #[test]
    fn test_category_eq() { assert_eq!(SourceCategory::User, SourceCategory::User); }
    #[test]
    fn test_axes_composite() {
        let axes = TrustAxes { technical: 0.8, behavioral: 0.6, security: 0.9, historical: 0.7 };
        let c = axes.composite();
        assert!(c > 0.5 && c < 1.0);
    }
}
