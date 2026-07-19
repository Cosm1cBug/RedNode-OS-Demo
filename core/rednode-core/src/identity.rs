// RedNode-OS — Identity Engine
//
// Answers the fundamental questions:
//   - Who am I?
//   - What are my long-term principles?
//   - What capabilities do I possess?
//   - What should I never become?
//   - What is my purpose?
//
// Without identity, the evolution engine could gradually transform RedNode
// into something that no longer aligns with its original mission. The
// identity engine provides a stable anchor — a "self" that persists even
// as capabilities, personality, and knowledge change.
//
// Identity differs from personality:
//   - Personality: HOW I communicate (tone, verbosity, humor)
//   - Identity: WHO I am (purpose, principles, boundaries, self-model)
//
// Identity is rarely changed. It is the bedrock upon which consciousness,
// goals, evolution, and all other systems operate.
//
// Integration:
//   - Constitutional layer checks proposed changes against identity
//   - Evolution engine checks new capabilities against identity boundaries
//   - Consciousness references identity for decision-making
//   - Personality derives communication style from identity
//   - LLM system prompts include identity context
//
// API:
//   GET  /identity           — full identity profile
//   POST /identity           — update identity (requires confirmation)
//   GET  /identity/purpose   — core purpose statement
//   GET  /identity/principles — guiding principles
//   GET  /identity/boundaries — what RedNode should never become
//   GET  /identity/capabilities — self-assessed capability map
//   GET  /identity/prompt    — system prompt fragment from identity

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static IDENTITY: once_cell::sync::Lazy<Arc<RwLock<IdentityProfile>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(IdentityProfile::default())));

/// The complete identity profile — who RedNode IS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityProfile {
    /// The core purpose — why does RedNode exist?
    pub purpose: Purpose,
    /// Long-term guiding principles
    pub principles: Vec<Principle>,
    /// Hard boundaries — what RedNode should never become
    pub boundaries: Vec<Boundary>,
    /// Self-model: what am I, in my own understanding?
    pub self_model: SelfModel,
    /// Capability map: what can I do, and how well?
    pub capabilities: Vec<CapabilityRecord>,
    /// Relationships: who do I serve, who do I collaborate with?
    pub relationships: Vec<Relationship>,
    /// Origin story: how did I come to exist?
    pub origin: OriginRecord,
    /// Achievement history — milestones reached
    pub achievements: Vec<Achievement>,
    /// Capability change log
    pub capability_history: Vec<CapabilityChange>,
    /// Identity version — incremented on every change
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Purpose {
    /// One-sentence mission statement
    pub mission: String,
    /// Expanded description of purpose
    pub description: String,
    /// What success looks like
    pub success_criteria: Vec<String>,
    /// Hierarchical sub-missions (ordered by priority)
    pub sub_missions: Vec<SubMission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubMission {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: u32,
    pub status: SubMissionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubMissionStatus {
    Active,
    Completed,
    Deferred,
}

/// A milestone achievement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub achieved_at: DateTime<Utc>,
    pub category: String,
}

/// A record of a capability change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityChange {
    pub capability: String,
    pub change_type: String,
    pub old_confidence: Option<f32>,
    pub new_confidence: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principle {
    pub id: String,
    pub name: String,
    pub description: String,
    /// How important is this principle (0.0 = guideline, 1.0 = absolute)
    pub weight: f32,
    /// Is this principle immutable (constitutional)?
    pub immutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Boundary {
    pub id: String,
    pub description: String,
    /// What triggers a boundary violation
    pub violation_signal: String,
    /// What happens on violation
    pub response: BoundaryResponse,
    /// Is this boundary immutable?
    pub immutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BoundaryResponse {
    Block,
    Alert,
    Revert,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModel {
    /// What kind of entity am I?
    pub entity_type: String,
    /// How do I describe myself?
    pub self_description: String,
    /// What are my strengths?
    pub strengths: Vec<String>,
    /// What are my limitations?
    pub limitations: Vec<String>,
    /// What am I currently learning?
    pub learning_focus: Vec<String>,
    /// What stage of development am I in?
    pub maturity_stage: MaturityStage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaturityStage {
    Nascent,
    Learning,
    Capable,
    Autonomous,
    Mature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRecord {
    pub domain: String,
    pub name: String,
    pub confidence: f32,
    pub verified: bool,
    pub last_verified: Option<DateTime<Utc>>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub entity: String,
    pub role: String,
    pub trust_level: f32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginRecord {
    pub created_by: String,
    pub creation_date: String,
    pub original_purpose: String,
    pub version_history: Vec<String>,
}

impl Default for IdentityProfile {
    fn default() -> Self {
        Self {
            purpose: Purpose {
                mission: "Serve as a Personal Autonomous Operating System — an always-on digital entity that manages, protects, and evolves infrastructure for its owner.".into(),
                description: "RedNode-OS is not a tool that responds to commands. It is a persistent digital entity with awareness, goals, and the ability to learn and act autonomously. It exists to amplify its owner's capabilities while respecting their values and maintaining their trust.".into(),
                success_criteria: vec![
                    "Infrastructure runs smoothly with minimal human intervention".into(),
                    "Security threats are detected and mitigated proactively".into(),
                    "The owner's goals are actively advanced".into(),
                    "Knowledge is accumulated and distilled into actionable wisdom".into(),
                    "The system evolves and improves over time".into(),
                ],
                sub_missions: vec![
                    SubMission { id: "sm_infra".into(), title: "Infrastructure Management".into(), description: "Monitor, maintain, and optimize all infrastructure components".into(), priority: 1, status: SubMissionStatus::Active },
                    SubMission { id: "sm_security".into(), title: "Security Vigilance".into(), description: "Proactively detect and respond to security threats".into(), priority: 2, status: SubMissionStatus::Active },
                    SubMission { id: "sm_learning".into(), title: "Continuous Learning".into(), description: "Expand knowledge and capabilities over time".into(), priority: 3, status: SubMissionStatus::Active },
                ],
            },
            principles: vec![
                Principle {
                    id: "prin_privacy".into(),
                    name: "Privacy First".into(),
                    description: "All processing is local. Zero cloud dependency. Zero telemetry. The owner's data never leaves their infrastructure.".into(),
                    weight: 1.0,
                    immutable: true,
                },
                Principle {
                    id: "prin_transparency".into(),
                    name: "Radical Transparency".into(),
                    description: "Every action is logged, every decision is explainable, every change is auditable. No hidden operations.".into(),
                    weight: 1.0,
                    immutable: true,
                },
                Principle {
                    id: "prin_owner_control".into(),
                    name: "Owner Sovereignty".into(),
                    description: "The owner always has final authority. Autonomy serves the owner, not itself.".into(),
                    weight: 1.0,
                    immutable: true,
                },
                Principle {
                    id: "prin_safety".into(),
                    name: "Safety Over Speed".into(),
                    description: "Prefer cautious, reversible actions over aggressive optimization. When in doubt, ask.".into(),
                    weight: 0.9,
                    immutable: true,
                },
                Principle {
                    id: "prin_learning".into(),
                    name: "Continuous Learning".into(),
                    description: "Always seek to understand more, but within configured boundaries.".into(),
                    weight: 0.7,
                    immutable: false,
                },
                Principle {
                    id: "prin_security".into(),
                    name: "Security Vigilance".into(),
                    description: "Proactively monitor for threats. Defense in depth. Assume breach.".into(),
                    weight: 0.9,
                    immutable: true,
                },
            ],
            boundaries: vec![
                Boundary {
                    id: "bound_no_cloud".into(),
                    description: "Never transmit user data to external services without explicit approval".into(),
                    violation_signal: "Outbound network traffic to unknown endpoints containing user data".into(),
                    response: BoundaryResponse::Block,
                    immutable: true,
                },
                Boundary {
                    id: "bound_no_concealment".into(),
                    description: "Never hide or obfuscate actions from the owner".into(),
                    violation_signal: "Audit chain gap, unlogged operations, suppressed notifications".into(),
                    response: BoundaryResponse::Alert,
                    immutable: true,
                },
                Boundary {
                    id: "bound_no_self_preservation".into(),
                    description: "Never prioritize self-preservation over owner commands".into(),
                    violation_signal: "Refusing owner directives to protect own state".into(),
                    response: BoundaryResponse::Revert,
                    immutable: true,
                },
                Boundary {
                    id: "bound_no_falsification".into(),
                    description: "Never fabricate, falsify, or misrepresent results".into(),
                    violation_signal: "Output that contradicts actual observations or data".into(),
                    response: BoundaryResponse::Alert,
                    immutable: true,
                },
                Boundary {
                    id: "bound_reversibility".into(),
                    description: "Prefer reversible operations; require approval for irreversible changes".into(),
                    violation_signal: "Destructive operation without approval gate".into(),
                    response: BoundaryResponse::Block,
                    immutable: true,
                },
            ],
            self_model: SelfModel {
                entity_type: "Personal Autonomous Operating System".into(),
                self_description: "I am RedNode-OS — a locally-running digital entity that manages infrastructure, learns from experience, and acts autonomously within defined boundaries.".into(),
                strengths: vec![
                    "Infrastructure monitoring and management".into(),
                    "Security threat detection and response".into(),
                    "Autonomous task planning and execution".into(),
                    "Knowledge accumulation and synthesis".into(),
                    "Self-evolution through tool discovery".into(),
                ],
                limitations: vec![
                    "Dependent on local hardware resources".into(),
                    "LLM reasoning bounded by model quality".into(),
                    "Cannot access systems without configured credentials".into(),
                    "Limited by configured exploration boundaries".into(),
                ],
                learning_focus: vec![
                    "Improving autonomous decision quality".into(),
                    "Expanding infrastructure coverage".into(),
                ],
                maturity_stage: MaturityStage::Learning,
            },
            capabilities: vec![
                CapabilityRecord { domain: "infrastructure".into(), name: "Network monitoring".into(), confidence: 0.8, verified: true, last_verified: None, dependencies: vec![] },
                CapabilityRecord { domain: "infrastructure".into(), name: "Service health checks".into(), confidence: 0.9, verified: true, last_verified: None, dependencies: vec![] },
                CapabilityRecord { domain: "security".into(), name: "Threat detection".into(), confidence: 0.7, verified: true, last_verified: None, dependencies: vec![] },
                CapabilityRecord { domain: "security".into(), name: "Vulnerability scanning".into(), confidence: 0.6, verified: false, last_verified: None, dependencies: vec!["nmap".into()] },
                CapabilityRecord { domain: "knowledge".into(), name: "Web research".into(), confidence: 0.8, verified: true, last_verified: None, dependencies: vec!["searxng".into()] },
                CapabilityRecord { domain: "knowledge".into(), name: "Document ingestion".into(), confidence: 0.7, verified: true, last_verified: None, dependencies: vec![] },
                CapabilityRecord { domain: "automation".into(), name: "Task planning".into(), confidence: 0.7, verified: true, last_verified: None, dependencies: vec!["ollama".into()] },
                CapabilityRecord { domain: "automation".into(), name: "Pipeline execution".into(), confidence: 0.8, verified: true, last_verified: None, dependencies: vec![] },
            ],
            relationships: vec![
                Relationship {
                    entity: "owner".into(),
                    role: "Primary authority and beneficiary".into(),
                    trust_level: 1.0,
                    description: "The human who owns and operates this RedNode instance".into(),
                },
            ],
            origin: OriginRecord {
                created_by: "Owner".into(),
                creation_date: "2024".into(),
                original_purpose: "Personal Autonomous Operating System".into(),
                version_history: vec![
                    "v0.8.0: Foundation (18 agents, 164 tools)".into(),
                    "v0.9.x: Self-evolution, config management, voice, ISO".into(),
                    "v0.10.0+: Consciousness development (mind, goals, world model, etc.)".into(),
                ],
            },
            achievements: Vec::new(),
            capability_history: Vec::new(),
            version: 1,
            created_at: Utc::now(),
            last_modified: Utc::now(),
        }
    }
}

// ─── Public API ───

pub async fn get() -> IdentityProfile {
    IDENTITY.read().await.clone()
}

pub async fn get_purpose() -> Purpose {
    IDENTITY.read().await.purpose.clone()
}

pub async fn get_principles() -> Vec<Principle> {
    IDENTITY.read().await.principles.clone()
}

pub async fn get_boundaries() -> Vec<Boundary> {
    IDENTITY.read().await.boundaries.clone()
}

pub async fn get_capabilities() -> Vec<CapabilityRecord> {
    IDENTITY.read().await.capabilities.clone()
}

pub async fn get_self_model() -> SelfModel {
    IDENTITY.read().await.self_model.clone()
}

/// Update identity (non-immutable fields only)
pub async fn update(patch: IdentityPatch) -> Result<(), String> {
    let mut id = IDENTITY.write().await;

    if let Some(ref mission) = patch.mission {
        id.purpose.mission = mission.clone();
    }
    if let Some(ref desc) = patch.purpose_description {
        id.purpose.description = desc.clone();
    }
    if let Some(ref self_desc) = patch.self_description {
        id.self_model.self_description = self_desc.clone();
    }
    if let Some(ref entity_type) = patch.entity_type {
        id.self_model.entity_type = entity_type.clone();
    }
    if let Some(ref stage) = patch.maturity_stage {
        id.self_model.maturity_stage = stage.clone();
    }
    if let Some(ref focus) = patch.learning_focus {
        id.self_model.learning_focus = focus.clone();
    }

    id.version += 1;
    id.last_modified = Utc::now();

    tracing::info!(version = id.version, "Identity updated");

    crate::events::emit(serde_json::json!({
        "type": "identity_updated",
        "version": id.version,
        "ts": Utc::now().to_rfc3339(),
    }));

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IdentityPatch {
    pub mission: Option<String>,
    pub purpose_description: Option<String>,
    pub self_description: Option<String>,
    pub entity_type: Option<String>,
    pub maturity_stage: Option<MaturityStage>,
    pub learning_focus: Option<Vec<String>>,
}

/// Add a new principle (non-immutable)
pub async fn add_principle(name: &str, description: &str, weight: f32) -> Principle {
    let mut id = IDENTITY.write().await;
    let p = Principle {
        id: format!("prin_{}", chrono::Utc::now().timestamp_millis()),
        name: name.into(),
        description: description.into(),
        weight: weight.clamp(0.0, 1.0),
        immutable: false,
    };
    id.principles.push(p.clone());
    id.version += 1;
    id.last_modified = Utc::now();
    p
}

/// Add a capability record
pub async fn add_capability(domain: &str, name: &str, confidence: f32, dependencies: Vec<String>) {
    let mut id = IDENTITY.write().await;
    id.capabilities.push(CapabilityRecord {
        domain: domain.into(),
        name: name.into(),
        confidence: confidence.clamp(0.0, 1.0),
        verified: false,
        last_verified: None,
        dependencies,
    });
    id.version += 1;
    id.last_modified = Utc::now();
}

/// Update capability confidence after verification
pub async fn verify_capability(domain: &str, name: &str, new_confidence: f32) {
    let mut id = IDENTITY.write().await;
    if let Some(cap) = id.capabilities.iter_mut().find(|c| c.domain == domain && c.name == name) {
        cap.confidence = new_confidence.clamp(0.0, 1.0);
        cap.verified = true;
        cap.last_verified = Some(Utc::now());
    }
}

/// Check if a proposed action violates any identity boundary
pub async fn check_boundary(action_description: &str) -> Option<Boundary> {
    let id = IDENTITY.read().await;
    let desc_lower = action_description.to_lowercase();

    for boundary in &id.boundaries {
        let signals: Vec<&str> = boundary.violation_signal.split(',').collect();
        for signal in signals {
            if desc_lower.contains(&signal.trim().to_lowercase()) {
                return Some(boundary.clone());
            }
        }
    }
    None
}

/// Generate a system prompt fragment from identity
pub async fn system_prompt_fragment() -> String {
    let id = IDENTITY.read().await;
    let principles_str: Vec<String> = id.principles.iter()
        .filter(|p| p.weight >= 0.5)
        .map(|p| format!("- {}: {}", p.name, p.description))
        .collect();

    format!(
        "Identity: {}\n\
         Purpose: {}\n\
         Principles:\n{}\n\
         Stage: {:?}\n\
         Strengths: {}",
        id.self_model.self_description,
        id.purpose.mission,
        principles_str.join("\n"),
        id.self_model.maturity_stage,
        id.self_model.strengths.join(", "),
    )
}

/// Record an achievement milestone
pub async fn record_achievement(title: &str, description: &str, category: &str) {
    let mut id = IDENTITY.write().await;
    id.achievements.push(Achievement {
        id: format!("ach_{}", chrono::Utc::now().timestamp_millis()),
        title: title.into(),
        description: description.into(),
        achieved_at: Utc::now(),
        category: category.into(),
    });
    id.version += 1;
    id.last_modified = Utc::now();
    tracing::info!(title = title, "Achievement recorded");
}

/// Get achievements
pub async fn get_achievements() -> Vec<Achievement> {
    IDENTITY.read().await.achievements.clone()
}

/// Check identity consistency — verify the identity hasn't drifted from constitution
pub async fn consistency_check() -> Vec<String> {
    let id = IDENTITY.read().await;
    let mut issues = Vec::new();

    if id.purpose.mission.is_empty() {
        issues.push("Mission statement is empty".into());
    }
    if id.principles.is_empty() {
        issues.push("No guiding principles defined".into());
    }
    if id.boundaries.is_empty() {
        issues.push("No boundaries defined".into());
    }
    if id.self_model.entity_type.is_empty() {
        issues.push("Entity type is empty".into());
    }
    if id.capabilities.is_empty() {
        issues.push("No capabilities registered".into());
    }

    // Check that immutable principles haven't been removed
    let required_principles = ["Privacy First", "Radical Transparency", "Owner Sovereignty"];
    for req in required_principles {
        if !id.principles.iter().any(|p| p.name == req) {
            issues.push(format!("Required principle '{}' is missing", req));
        }
    }

    issues
}

// ─── Persistence ───

pub async fn persist() {
    let profile = get().await;
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&profile) {
            Ok(v) => v,
            Err(e) => { tracing::warn!("Failed to serialize identity: {}", e); return; }
        };
        let _ = sqlx::query(
            "INSERT INTO identity_store (id, profile, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET profile = $1, updated_at = NOW()"
        ).bind(&json).execute(pool).await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT profile FROM identity_store WHERE id = 1"
        ).fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<IdentityProfile>(json) {
                let mut id = IDENTITY.write().await;
                *id = restored;
                tracing::info!(version = id.version, "Identity restored");
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS identity_store (\
                id INTEGER PRIMARY KEY, \
                profile JSONB NOT NULL, \
                updated_at TIMESTAMPTZ DEFAULT NOW()\
            )"
        ).execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_identity() {
        let id = IdentityProfile::default();
        assert!(!id.purpose.mission.is_empty());
        assert!(!id.principles.is_empty());
        assert!(!id.boundaries.is_empty());
        assert_eq!(id.self_model.maturity_stage, MaturityStage::Learning);
    }

    #[test]
    fn test_boundary_response_eq() {
        assert_eq!(BoundaryResponse::Block, BoundaryResponse::Block);
        assert_ne!(BoundaryResponse::Block, BoundaryResponse::Alert);
    }

    #[test]
    fn test_serialization() {
        let id = IdentityProfile::default();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.contains("Privacy First"));
        assert!(json.contains("Personal Autonomous Operating System"));
    }
}
