// RedNode-OS — Curiosity Engine
//
// Drives autonomous exploration and learning. Instead of only responding
// to commands, RedNode actively seeks out relevant knowledge based on:
//   - Active goals (if goal is "learn Kubernetes", search K8s content)
//   - Configured topics (cybersecurity, NixOS, Rust, home automation)
//   - Configured sources (arxiv, github trending, RSS, CVE feeds, RFCs)
//
// Bounded curiosity — the user controls:
//   - Which topics to explore
//   - Which sources to use
//   - How many explorations per day (default: 10)
//   - Whether to notify on discovery
//
// The curiosity engine runs as a background task, exploring every 2 hours.
// Discoveries are ingested into the knowledge base and optionally notified.
//
// Integration:
//   - Consciousness reads curiosity state
//   - Goals influence topic selection
//   - Research agent executes the actual searches
//   - Learning agent ingests findings
//
// API:
//   GET  /curiosity            — current curiosity state
//   GET  /curiosity/discoveries — recent discoveries
//   POST /curiosity/config      — update curiosity configuration
//   POST /curiosity/explore     — trigger exploration now

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static CURIOSITY: once_cell::sync::Lazy<Arc<RwLock<CuriosityState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(CuriosityState::default())));

/// The curiosity engine's complete state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriosityState {
    pub config: CuriosityConfig,
    pub discoveries: VecDeque<Discovery>,
    pub exploration_queue: Vec<ExplorationTarget>,
    pub stats: CuriosityStats,
    pub last_exploration: Option<DateTime<Utc>>,
    pub currently_exploring: Option<String>,
}

/// User-configurable curiosity boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriosityConfig {
    pub enabled: bool,
    pub topics: Vec<String>,
    pub sources: Vec<CuriositySource>,
    pub max_daily_explorations: u32,
    pub notify_on_discovery: bool,
    pub exploration_interval_hours: u32,
    pub novelty_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CuriositySource {
    Arxiv,
    GithubTrending,
    Rss(String),
    Cve,
    Rfc,
    HackerNews,
    Reddit(String),
    Custom { name: String, url: String },
}

/// A single discovery — something interesting the engine found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discovery {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub source: String,
    pub url: Option<String>,
    pub topic: String,
    pub relevance_score: f32,
    pub novelty_score: f32,
    pub discovered_at: DateTime<Utc>,
    pub ingested: bool,
    pub notified: bool,
    /// Did this discovery lead to a real action?
    pub impact: Option<DiscoveryImpact>,
}

/// Tracks whether a discovery had real-world impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryImpact {
    pub impact_type: String,
    pub description: String,
    pub tracked_at: DateTime<Utc>,
}

/// An exploration target — what to search next
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationTarget {
    pub topic: String,
    pub query: String,
    pub source: String,
    pub priority: f32,
    pub reason: String,
}

/// Stats tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriosityStats {
    pub total_explorations: u64,
    pub total_discoveries: u64,
    pub today_explorations: u32,
    pub today_date: String,
    pub discoveries_by_topic: std::collections::HashMap<String, u64>,
}

impl Default for CuriosityState {
    fn default() -> Self {
        Self {
            config: CuriosityConfig {
                enabled: true,
                topics: vec![
                    "cybersecurity".into(),
                    "NixOS".into(),
                    "Rust".into(),
                    "home automation".into(),
                    "self-hosting".into(),
                ],
                sources: vec![
                    CuriositySource::Cve,
                    CuriositySource::GithubTrending,
                    CuriositySource::HackerNews,
                    CuriositySource::Arxiv,
                ],
                max_daily_explorations: 10,
                notify_on_discovery: true,
                exploration_interval_hours: 2,
                novelty_threshold: 0.5,
            },
            discoveries: VecDeque::with_capacity(100),
            exploration_queue: Vec::new(),
            stats: CuriosityStats {
                total_explorations: 0,
                total_discoveries: 0,
                today_explorations: 0,
                today_date: Utc::now().format("%Y-%m-%d").to_string(),
                discoveries_by_topic: std::collections::HashMap::new(),
            },
            last_exploration: None,
            currently_exploring: None,
        }
    }
}

fn gen_id() -> String {
    format!("disc_{}", chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// Get the full curiosity state
pub async fn get_state() -> CuriosityState {
    CURIOSITY.read().await.clone()
}

/// Get recent discoveries
pub async fn get_discoveries(limit: usize) -> Vec<Discovery> {
    let state = CURIOSITY.read().await;
    state.discoveries.iter().rev().take(limit).cloned().collect()
}

/// Update curiosity configuration
pub async fn update_config(new_config: CuriosityConfig) {
    let mut state = CURIOSITY.write().await;
    state.config = new_config;
    tracing::info!("Curiosity config updated");

    crate::events::emit(serde_json::json!({
        "type": "curiosity_config_updated",
        "ts": Utc::now().to_rfc3339(),
    }));
}

/// Record a discovery from an exploration
pub async fn record_discovery(
    title: &str,
    summary: &str,
    source: &str,
    url: Option<String>,
    topic: &str,
    relevance: f32,
    novelty: f32,
) -> Option<Discovery> {
    let mut state = CURIOSITY.write().await;

    // Skip if below novelty threshold
    if novelty < state.config.novelty_threshold {
        return None;
    }

    // Check for duplicates (same title)
    if state.discoveries.iter().any(|d| d.title == title) {
        return None;
    }

    let discovery = Discovery {
        id: gen_id(),
        title: title.into(),
        summary: summary.into(),
        source: source.into(),
        url,
        topic: topic.into(),
        relevance_score: relevance.clamp(0.0, 1.0),
        novelty_score: novelty.clamp(0.0, 1.0),
        discovered_at: Utc::now(),
        ingested: false,
        notified: false,
        impact: None,
    };

    // Keep last 100 discoveries
    if state.discoveries.len() >= 100 {
        state.discoveries.pop_front();
    }
    state.discoveries.push_back(discovery.clone());

    // Update stats
    state.stats.total_discoveries += 1;
    *state.stats.discoveries_by_topic
        .entry(topic.to_string())
        .or_insert(0) += 1;

    tracing::info!(
        title = title,
        topic = topic,
        relevance = relevance,
        novelty = novelty,
        "New discovery recorded"
    );

    Some(discovery)
}

/// Mark a discovery as ingested into the knowledge base
pub async fn mark_ingested(discovery_id: &str) {
    let mut state = CURIOSITY.write().await;
    if let Some(d) = state.discoveries.iter_mut().find(|d| d.id == discovery_id) {
        d.ingested = true;
    }
}

/// Mark a discovery as notified
pub async fn mark_notified(discovery_id: &str) {
    let mut state = CURIOSITY.write().await;
    if let Some(d) = state.discoveries.iter_mut().find(|d| d.id == discovery_id) {
        d.notified = true;
    }
}

/// Record that a discovery had real-world impact
pub async fn record_impact(discovery_id: &str, impact_type: &str, description: &str) {
    let mut state = CURIOSITY.write().await;
    if let Some(d) = state.discoveries.iter_mut().find(|d| d.id == discovery_id) {
        d.impact = Some(DiscoveryImpact {
            impact_type: impact_type.into(),
            description: description.into(),
            tracked_at: Utc::now(),
        });
    }
}

/// Get discoveries that had real impact
pub async fn get_impactful_discoveries() -> Vec<Discovery> {
    CURIOSITY.read().await.discoveries.iter()
        .filter(|d| d.impact.is_some())
        .cloned()
        .collect()
}

/// Build the exploration queue based on goals + configured topics
pub async fn build_exploration_queue() {
    let mut state = CURIOSITY.write().await;
    state.exploration_queue.clear();

    let goals = crate::goals::active().await;

    // Add goal-derived exploration targets (high priority)
    for goal in &goals {
        for tag in &goal.tags {
            state.exploration_queue.push(ExplorationTarget {
                topic: tag.clone(),
                query: format!("latest developments in {}", tag),
                source: "auto".into(),
                priority: 0.9,
                reason: format!("Active goal: {}", goal.title),
            });
        }
    }

    // Add configured topics (normal priority)
    for topic in &state.config.topics {
        // Skip if already covered by goals
        if state.exploration_queue.iter().any(|t| t.topic == *topic) {
            continue;
        }
        state.exploration_queue.push(ExplorationTarget {
            topic: topic.clone(),
            query: format!("new in {} this week", topic),
            source: "auto".into(),
            priority: 0.5,
            reason: "Configured interest topic".into(),
        });
    }

    // Sort by priority descending
    state.exploration_queue.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
}

/// Background tick — checks if exploration is due
pub async fn tick() -> bool {
    let state = CURIOSITY.read().await;

    if !state.config.enabled {
        return false;
    }

    let now = Utc::now();
    let today = now.format("%Y-%m-%d").to_string();

    // Reset daily count if new day
    let daily_limit_reached = if state.stats.today_date == today {
        state.stats.today_explorations >= state.config.max_daily_explorations
    } else {
        false
    };

    if daily_limit_reached {
        return false;
    }

    // Check if enough time has passed since last exploration
    let exploration_due = match state.last_exploration {
        Some(last) => (now - last).num_hours() >= state.config.exploration_interval_hours as i64,
        None => true,
    };

    drop(state);

    if exploration_due {
        // Mark that exploration is starting
        let mut state = CURIOSITY.write().await;
        state.last_exploration = Some(now);
        state.stats.total_explorations += 1;

        let today_str = now.format("%Y-%m-%d").to_string();
        if state.stats.today_date != today_str {
            state.stats.today_explorations = 1;
            state.stats.today_date = today_str;
        } else {
            state.stats.today_explorations += 1;
        }

        return true;
    }

    false
}

/// Get curiosity level for consciousness (0.0 - 1.0)
pub async fn curiosity_level() -> f32 {
    let state = CURIOSITY.read().await;
    if !state.config.enabled {
        return 0.0;
    }
    // Curiosity increases with time since last exploration
    match state.last_exploration {
        Some(last) => {
            let hours_since = (Utc::now() - last).num_hours() as f32;
            (hours_since / (state.config.exploration_interval_hours as f32 * 2.0)).min(1.0)
        }
        None => 0.8,
    }
}

// ─── Persistence ───

pub async fn persist() {
    let state = CURIOSITY.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize curiosity state: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO curiosity_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        )
        .bind(&json)
        .execute(pool)
        .await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM curiosity_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<CuriosityState>(json) {
                let mut state = CURIOSITY.write().await;
                *state = restored;
                tracing::info!(
                    discoveries = state.discoveries.len(),
                    total = state.stats.total_explorations,
                    "Curiosity engine restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS curiosity_store (\
                id INTEGER PRIMARY KEY, \
                state JSONB NOT NULL, \
                updated_at TIMESTAMPTZ DEFAULT NOW()\
            )"
        )
        .execute(pool)
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = CuriosityState::default();
        assert!(state.config.enabled);
        assert!(!state.config.topics.is_empty());
        assert_eq!(state.config.max_daily_explorations, 10);
    }

    #[test]
    fn test_discovery_serialization() {
        let d = Discovery {
            id: "test_1".into(),
            title: "New CVE in OpenSSH".into(),
            summary: "Critical vulnerability".into(),
            source: "cve".into(),
            url: Some("https://nvd.nist.gov/vuln/detail/CVE-2024-99999".into()),
            topic: "cybersecurity".into(),
            relevance_score: 0.9,
            novelty_score: 0.8,
            discovered_at: Utc::now(),
            ingested: false,
            notified: false,
            impact: None,
        };
        let json = serde_json::to_string(&d).expect("serialize discovery");
        assert!(json.contains("OpenSSH"));
        assert!(json.contains("cybersecurity"));
    }

    #[test]
    fn test_source_enum() {
        let s = CuriositySource::Cve;
        assert_eq!(s, CuriositySource::Cve);
        let custom = CuriositySource::Custom {
            name: "test".into(),
            url: "http://example.com".into(),
        };
        let json = serde_json::to_string(&custom).unwrap();
        assert!(json.contains("test"));
    }
}
