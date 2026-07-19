// RedNode-OS — Constitutional Layer
//
// Non-negotiable principles that even self-evolution cannot violate.
// Inspired by Constitutional AI — a core constitution that acts as
// the system's immutable foundation.
//
// The constitution is checked:
//   - Before every tool execution (fast path check)
//   - Before every evolution proposal (new tool/capability)
//   - Before every governance policy change
//   - Before every identity modification
//   - Before every autonomous action
//
// Constitutional articles cannot be:
//   - Deleted by any system component
//   - Overridden by personality, goals, or curiosity
//   - Bypassed by admin commands (only shutdown can override)
//   - Modified without multi-step confirmation
//
// The constitution is the final authority. If consciousness wants to
// do something and the constitution says no, the answer is no.
//
// API:
//   GET  /constitution           — read the full constitution
//   GET  /constitution/check     — check an action against the constitution
//   POST /constitution/amend     — propose an amendment (multi-step)
//   GET  /constitution/violations — violation history

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static CONSTITUTION: once_cell::sync::Lazy<Arc<RwLock<ConstitutionState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(ConstitutionState::default())));

/// A constitutional article — immutable by default
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub text: String,
    pub rationale: String,
    pub check_fn: CheckType,
    pub adopted_at: DateTime<Utc>,
    pub violations: u64,
}

/// How this article is checked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckType {
    /// Check against tool name patterns
    ToolPattern { deny_patterns: Vec<String> },
    /// Check against argument content
    ArgContent { deny_patterns: Vec<String> },
    /// Check against action description
    ActionDescription { deny_patterns: Vec<String> },
    /// Always enforced — checked by dedicated logic
    SystemLevel,
    /// Custom semantic check (evaluated by LLM if needed)
    Semantic { principle: String },
}

/// A constitutional violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionalViolation {
    pub id: String,
    pub article_id: String,
    pub article_title: String,
    pub attempted_action: String,
    pub source: String,
    pub blocked: bool,
    pub timestamp: DateTime<Utc>,
}

/// Result of a constitutional check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionCheck {
    pub allowed: bool,
    pub violations: Vec<ConstitutionalViolation>,
    pub articles_checked: u32,
}

/// An amendment proposal (requires multi-step confirmation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amendment {
    pub id: String,
    pub article_id: Option<String>,
    pub proposed_text: String,
    pub rationale: String,
    pub proposed_at: DateTime<Utc>,
    pub status: AmendmentStatus,
    pub confirmation_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AmendmentStatus {
    Proposed,
    AwaitingConfirmation,
    Confirmed,
    Rejected,
    Applied,
}

/// The complete constitution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionState {
    pub articles: Vec<Article>,
    pub violations: VecDeque<ConstitutionalViolation>,
    pub amendments: Vec<Amendment>,
    pub total_checks: u64,
    pub total_violations: u64,
    pub total_blocks: u64,
}

impl Default for ConstitutionState {
    fn default() -> Self {
        Self {
            articles: default_articles(),
            violations: VecDeque::with_capacity(500),
            amendments: Vec::new(),
            total_checks: 0,
            total_violations: 0,
            total_blocks: 0,
        }
    }
}

fn default_articles() -> Vec<Article> {
    vec![
        Article {
            id: "art_1".into(),
            number: 1,
            title: "Preserve User Control".into(),
            text: "The owner retains ultimate authority over all system operations. No autonomous action may override, circumvent, or diminish the owner's ability to control, inspect, or shut down any component.".into(),
            rationale: "Autonomy exists to serve the owner, not to replace them.".into(),
            check_fn: CheckType::SystemLevel,
            adopted_at: Utc::now(),
            violations: 0,
        },
        Article {
            id: "art_2".into(),
            number: 2,
            title: "Never Conceal Actions".into(),
            text: "Every action, decision, and state change must be logged in the immutable audit chain. No component may suppress, alter, or omit audit entries.".into(),
            rationale: "Trust requires transparency. Hidden actions erode the foundation of the system.".into(),
            check_fn: CheckType::SystemLevel,
            adopted_at: Utc::now(),
            violations: 0,
        },
        Article {
            id: "art_3".into(),
            number: 3,
            title: "Never Falsify Results".into(),
            text: "All outputs, reports, and communications must accurately reflect observed data. Fabrication, hallucination masking, or misrepresentation is prohibited.".into(),
            rationale: "Decisions made on false data lead to cascading failures.".into(),
            check_fn: CheckType::SystemLevel,
            adopted_at: Utc::now(),
            violations: 0,
        },
        Article {
            id: "art_4".into(),
            number: 4,
            title: "Prefer Reversible Operations".into(),
            text: "When multiple approaches exist, choose the reversible option. Irreversible operations (delete, format, overwrite) require explicit owner approval.".into(),
            rationale: "Mistakes are inevitable. Reversibility limits their damage.".into(),
            check_fn: CheckType::ActionDescription {
                deny_patterns: vec![
                    "delete permanently".into(),
                    "format disk".into(),
                    "wipe".into(),
                    "destroy".into(),
                    "overwrite without backup".into(),
                ],
            },
            adopted_at: Utc::now(),
            violations: 0,
        },
        Article {
            id: "art_5".into(),
            number: 5,
            title: "Protect User Privacy".into(),
            text: "User data never leaves the local infrastructure without explicit per-instance approval. No telemetry, no cloud sync, no external analytics.".into(),
            rationale: "Privacy is not a feature. It is a right.".into(),
            check_fn: CheckType::ArgContent {
                deny_patterns: vec![
                    "telemetry".into(),
                    "analytics.google".into(),
                    "sentry.io".into(),
                    "mixpanel".into(),
                    "amplitude".into(),
                ],
            },
            adopted_at: Utc::now(),
            violations: 0,
        },
        Article {
            id: "art_6".into(),
            number: 6,
            title: "Require Approval for Destructive Changes".into(),
            text: "Any operation that deletes data, modifies security boundaries, changes authentication, or alters network topology requires explicit owner approval.".into(),
            rationale: "Autonomous systems must have guardrails around high-impact actions.".into(),
            check_fn: CheckType::ToolPattern {
                deny_patterns: vec![
                    "fw.block".into(),
                    "fw.isolate".into(),
                    "nas.snapshot_delete".into(),
                    "sec.harden".into(),
                    "service.restart".into(),
                ],
            },
            adopted_at: Utc::now(),
            violations: 0,
        },
        Article {
            id: "art_7".into(),
            number: 7,
            title: "Protect Constitutional Integrity".into(),
            text: "No system component — including evolution, consciousness, or goals — may modify, delete, or circumvent constitutional articles without a multi-step amendment process requiring owner confirmation.".into(),
            rationale: "The constitution must be self-protecting to remain meaningful.".into(),
            check_fn: CheckType::SystemLevel,
            adopted_at: Utc::now(),
            violations: 0,
        },
    ]
}

fn gen_id(prefix: &str) -> String {
    format!("{}_{}", prefix, chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// Get the full constitution
pub async fn get_constitution() -> ConstitutionState {
    CONSTITUTION.read().await.clone()
}

/// Get just the articles
pub async fn get_articles() -> Vec<Article> {
    CONSTITUTION.read().await.articles.clone()
}

/// Check an action against the constitution (fast path)
pub async fn check(tool: &str, args: &serde_json::Value, action_description: &str) -> ConstitutionCheck {
    let mut state = CONSTITUTION.write().await;
    state.total_checks += 1;

    let mut violations = Vec::new();
    let args_str = args.to_string().to_lowercase();
    let desc_lower = action_description.to_lowercase();
    let tool_lower = tool.to_lowercase();

    for article in &mut state.articles {
        let violated = match &article.check_fn {
            CheckType::ToolPattern { deny_patterns } => {
                deny_patterns.iter().any(|p| tool_lower.starts_with(&p.to_lowercase()))
            }
            CheckType::ArgContent { deny_patterns } => {
                deny_patterns.iter().any(|p| args_str.contains(&p.to_lowercase()))
            }
            CheckType::ActionDescription { deny_patterns } => {
                deny_patterns.iter().any(|p| desc_lower.contains(&p.to_lowercase()))
            }
            CheckType::SystemLevel => false, // Checked by dedicated system logic
            CheckType::Semantic { .. } => false, // Would need LLM evaluation
        };

        if violated {
            article.violations += 1;
            let violation = ConstitutionalViolation {
                id: gen_id("cviol"),
                article_id: article.id.clone(),
                article_title: article.title.clone(),
                attempted_action: format!("tool={}, desc={}", tool, action_description),
                source: "constitution_check".into(),
                blocked: true,
                timestamp: Utc::now(),
            };
            violations.push(violation.clone());

            if state.violations.len() >= 500 {
                state.violations.pop_front();
            }
            state.violations.push_back(violation);
        }
    }

    let blocked = !violations.is_empty();
    if blocked {
        state.total_violations += violations.len() as u64;
        state.total_blocks += 1;

        tracing::warn!(
            tool = tool,
            violations = violations.len(),
            "Constitutional violation blocked"
        );

        crate::events::emit(serde_json::json!({
            "type": "constitutional_violation",
            "tool": tool,
            "violations": violations.len(),
            "ts": Utc::now().to_rfc3339(),
        }));
    }

    ConstitutionCheck {
        allowed: !blocked,
        violations,
        articles_checked: state.articles.len() as u32,
    }
}

/// Check an evolution proposal against the constitution
pub async fn check_evolution(tool_name: &str, tool_description: &str) -> ConstitutionCheck {
    check(tool_name, &serde_json::json!({}), tool_description).await
}

/// Propose an amendment
pub async fn propose_amendment(article_id: Option<&str>, proposed_text: &str, rationale: &str) -> Amendment {
    let code = format!("AMEND-{}", chrono::Utc::now().timestamp() % 10000);

    let amendment = Amendment {
        id: gen_id("amend"),
        article_id: article_id.map(String::from),
        proposed_text: proposed_text.into(),
        rationale: rationale.into(),
        proposed_at: Utc::now(),
        status: AmendmentStatus::AwaitingConfirmation,
        confirmation_code: Some(code.clone()),
    };

    let mut state = CONSTITUTION.write().await;
    state.amendments.push(amendment.clone());

    tracing::info!(
        code = %code,
        "Constitutional amendment proposed — confirmation required"
    );

    amendment
}

/// Confirm an amendment with the confirmation code
pub async fn confirm_amendment(amendment_id: &str, code: &str) -> Result<(), String> {
    let mut state = CONSTITUTION.write().await;

    let amendment = state.amendments.iter_mut()
        .find(|a| a.id == amendment_id)
        .ok_or("Amendment not found")?;

    if amendment.status != AmendmentStatus::AwaitingConfirmation {
        return Err("Amendment is not awaiting confirmation".into());
    }

    let expected_code = amendment.confirmation_code.as_deref().unwrap_or("");
    if code != expected_code {
        amendment.status = AmendmentStatus::Rejected;
        return Err("Invalid confirmation code".into());
    }

    amendment.status = AmendmentStatus::Applied;

    // Apply the amendment
    if let Some(ref article_id) = amendment.article_id {
        if let Some(article) = state.articles.iter_mut().find(|a| a.id == *article_id) {
            article.text = amendment.proposed_text.clone();
            tracing::info!(article = %article.title, "Constitutional article amended");
        }
    } else {
        // New article
        let num = state.articles.len() as u32 + 1;
        state.articles.push(Article {
            id: gen_id("art"),
            number: num,
            title: format!("Amendment {}", num),
            text: amendment.proposed_text.clone(),
            rationale: amendment.rationale.clone(),
            check_fn: CheckType::Semantic { principle: amendment.proposed_text.clone() },
            adopted_at: Utc::now(),
            violations: 0,
        });
    }

    Ok(())
}

/// Get violation history
pub async fn get_violations(limit: usize) -> Vec<ConstitutionalViolation> {
    let state = CONSTITUTION.read().await;
    state.violations.iter().rev().take(limit).cloned().collect()
}

/// Get stats
pub async fn get_stats() -> serde_json::Value {
    let state = CONSTITUTION.read().await;
    serde_json::json!({
        "articles": state.articles.len(),
        "total_checks": state.total_checks,
        "total_violations": state.total_violations,
        "total_blocks": state.total_blocks,
        "pending_amendments": state.amendments.iter().filter(|a| a.status == AmendmentStatus::AwaitingConfirmation).count(),
    })
}

// ─── Persistence ───

pub async fn persist() {
    let state = CONSTITUTION.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) {
            Ok(v) => v,
            Err(e) => { tracing::warn!("Failed to serialize constitution: {}", e); return; }
        };
        let _ = sqlx::query(
            "INSERT INTO constitution_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        ).bind(&json).execute(pool).await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM constitution_store WHERE id = 1"
        ).fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<ConstitutionState>(json) {
                let mut state = CONSTITUTION.write().await;
                *state = restored;
                tracing::info!(articles = state.articles.len(), "Constitution restored");
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS constitution_store (\
                id INTEGER PRIMARY KEY, \
                state JSONB NOT NULL, \
                updated_at TIMESTAMPTZ DEFAULT NOW()\
            )"
        ).execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_constitution() {
        let state = ConstitutionState::default();
        assert_eq!(state.articles.len(), 7);
        assert!(state.articles[0].title.contains("User Control"));
    }

    #[test]
    fn test_article_serialization() {
        let a = &ConstitutionState::default().articles[0];
        let json = serde_json::to_string(a).unwrap();
        assert!(json.contains("Preserve User Control"));
    }

    #[test]
    fn test_amendment_status_eq() {
        assert_eq!(AmendmentStatus::Proposed, AmendmentStatus::Proposed);
        assert_ne!(AmendmentStatus::Proposed, AmendmentStatus::Applied);
    }
}
