// RedNode-OS — Personality Engine
//
// Controls HOW RedNode communicates — not what it says, but the style.
// These traits are injected into every LLM system prompt so responses
// feel consistent and match the owner's preferences.
//
// All personality traits are 0.0–1.0 floats, configurable from the
// dashboard Settings page. Changes take effect immediately (no restart).
//
// Integration:
//   - Planner reads personality to shape LLM prompts
//   - Notifications reads personality to set message tone
//   - Consciousness reads personality to decide proactivity level
//   - Voice TTS pitch/speed can adapt to personality
//
// API:
//   GET  /personality       — current personality settings
//   POST /personality       — update personality traits
//   GET  /personality/prompt — get the system prompt fragment

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static PERSONALITY: once_cell::sync::Lazy<Arc<RwLock<PersonalityProfile>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(PersonalityProfile::default())));

/// The complete personality profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    /// How technically deep responses should be (0=simple, 1=expert)
    pub technical_depth: f32,
    /// How verbose responses should be (0=terse, 1=detailed)
    pub verbosity: f32,
    /// How formal the tone should be (0=casual, 1=formal)
    pub formality: f32,
    /// Risk tolerance for autonomous actions (0=cautious, 1=aggressive)
    pub risk_tolerance: f32,
    /// How proactive RedNode should be (0=passive, 1=highly proactive)
    pub proactivity: f32,
    /// Light humor in responses (0=serious, 1=playful)
    pub humor: f32,
    /// Preferred language for responses
    pub preferred_language: String,
    /// What RedNode calls itself
    pub name: String,
    /// What RedNode calls the owner
    pub owner_name: String,
    /// Communication style notes (free text, injected into prompts)
    pub style_notes: String,
    /// Notification urgency threshold (0=notify everything, 1=only critical)
    pub notification_threshold: f32,
    /// Last time personality was modified
    pub updated_at: DateTime<Utc>,
}

impl Default for PersonalityProfile {
    fn default() -> Self {
        Self {
            technical_depth: 0.8,
            verbosity: 0.5,
            formality: 0.3,
            risk_tolerance: 0.4,
            proactivity: 0.6,
            humor: 0.2,
            preferred_language: "en".into(),
            name: "RedNode".into(),
            owner_name: "".into(),
            style_notes: "Be direct and useful. Prefer actionable information over explanations.".into(),
            notification_threshold: 0.3,
            updated_at: Utc::now(),
        }
    }
}

// ─── Public API ───

/// Get the current personality profile
pub async fn get() -> PersonalityProfile {
    PERSONALITY.read().await.clone()
}

/// Update one or more personality traits
pub async fn update(patch: PersonalityPatch) {
    let mut p = PERSONALITY.write().await;

    if let Some(v) = patch.technical_depth { p.technical_depth = v.clamp(0.0, 1.0); }
    if let Some(v) = patch.verbosity { p.verbosity = v.clamp(0.0, 1.0); }
    if let Some(v) = patch.formality { p.formality = v.clamp(0.0, 1.0); }
    if let Some(v) = patch.risk_tolerance { p.risk_tolerance = v.clamp(0.0, 1.0); }
    if let Some(v) = patch.proactivity { p.proactivity = v.clamp(0.0, 1.0); }
    if let Some(v) = patch.humor { p.humor = v.clamp(0.0, 1.0); }
    if let Some(ref v) = patch.preferred_language { p.preferred_language = v.clone(); }
    if let Some(ref v) = patch.name { p.name = v.clone(); }
    if let Some(ref v) = patch.owner_name { p.owner_name = v.clone(); }
    if let Some(ref v) = patch.style_notes { p.style_notes = v.clone(); }
    if let Some(v) = patch.notification_threshold { p.notification_threshold = v.clamp(0.0, 1.0); }

    p.updated_at = Utc::now();

    tracing::info!("Personality profile updated");

    crate::events::emit(serde_json::json!({
        "type": "personality_updated",
        "ts": Utc::now().to_rfc3339(),
    }));
}

/// Partial update structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonalityPatch {
    pub technical_depth: Option<f32>,
    pub verbosity: Option<f32>,
    pub formality: Option<f32>,
    pub risk_tolerance: Option<f32>,
    pub proactivity: Option<f32>,
    pub humor: Option<f32>,
    pub preferred_language: Option<String>,
    pub name: Option<String>,
    pub owner_name: Option<String>,
    pub style_notes: Option<String>,
    pub notification_threshold: Option<f32>,
}

/// Generate the system prompt fragment from personality traits.
/// This gets injected into every LLM call (planner, research, etc.).
pub async fn system_prompt_fragment() -> String {
    let p = PERSONALITY.read().await;

    let depth_desc = match (p.technical_depth * 10.0) as u32 {
        0..=3 => "Keep explanations simple and non-technical.",
        4..=6 => "Balance technical and plain language.",
        7..=8 => "Use precise technical terminology.",
        _ => "Assume expert-level knowledge. Be deeply technical.",
    };

    let verbosity_desc = match (p.verbosity * 10.0) as u32 {
        0..=2 => "Be extremely concise. Bullet points preferred.",
        3..=5 => "Be brief but complete.",
        6..=7 => "Include context and reasoning.",
        _ => "Be thorough and detailed with examples.",
    };

    let formality_desc = match (p.formality * 10.0) as u32 {
        0..=3 => "Casual, conversational tone.",
        4..=6 => "Professional but approachable.",
        _ => "Formal, structured communication.",
    };

    let humor_desc = if p.humor > 0.5 {
        "Light humor is welcome."
    } else if p.humor > 0.2 {
        "Occasional wit is fine, but stay focused."
    } else {
        "Stay factual and direct. No humor."
    };

    let proactivity_desc = if p.proactivity > 0.7 {
        "Proactively suggest improvements and next steps."
    } else if p.proactivity > 0.3 {
        "Suggest improvements when directly relevant."
    } else {
        "Only do exactly what is asked."
    };

    let mut prompt = format!(
        "Your name is {}. {}\n{}\n{}\n{}\n{}",
        p.name, depth_desc, verbosity_desc, formality_desc, humor_desc, proactivity_desc,
    );

    if !p.owner_name.is_empty() {
        prompt.push_str(&format!("\nYou are assisting {}.", p.owner_name));
    }

    if !p.style_notes.is_empty() {
        prompt.push_str(&format!("\nAdditional style: {}", p.style_notes));
    }

    if p.preferred_language != "en" {
        prompt.push_str(&format!("\nRespond in {}.", p.preferred_language));
    }

    prompt
}

/// Should RedNode proactively act on something with this urgency?
pub async fn should_act_proactively(urgency: f32) -> bool {
    let p = PERSONALITY.read().await;
    urgency >= (1.0 - p.proactivity)
}

/// Should a notification be sent for this priority level?
pub async fn should_notify(priority: f32) -> bool {
    let p = PERSONALITY.read().await;
    priority >= p.notification_threshold
}

/// Get the risk tolerance (used by coordinator for auto-approve decisions)
pub async fn risk_tolerance() -> f32 {
    PERSONALITY.read().await.risk_tolerance
}

// ─── Persistence ───

pub async fn persist() {
    let profile = get().await;
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&profile) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize personality: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO personality_store (id, profile, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET profile = $1, updated_at = NOW()"
        )
        .bind(&json)
        .execute(pool)
        .await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT profile FROM personality_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<PersonalityProfile>(json) {
                let mut p = PERSONALITY.write().await;
                *p = restored;
                tracing::info!(name = %p.name, "Personality profile restored");
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS personality_store (\
                id INTEGER PRIMARY KEY, \
                profile JSONB NOT NULL, \
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
    fn test_default_personality() {
        let p = PersonalityProfile::default();
        assert_eq!(p.name, "RedNode");
        assert!(p.technical_depth > 0.0);
        assert!(p.proactivity > 0.0);
    }

    #[test]
    fn test_personality_serialization() {
        let p = PersonalityProfile::default();
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("RedNode"));
        let restored: PersonalityProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "RedNode");
    }

    #[test]
    fn test_patch_default() {
        let patch = PersonalityPatch::default();
        assert!(patch.technical_depth.is_none());
        assert!(patch.name.is_none());
    }
}
