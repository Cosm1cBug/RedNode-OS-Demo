// RedNode-OS — Collective Governance
//
// Defines how multiple RedNode instances resolve disagreements:
// consensus, voting, or leader election.
//
// API:
//   POST /collective/vote     — start a vote across instances
//   GET  /collective/votes    — active/past votes
//   POST /collective/election — trigger leader election

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static COLGOV: once_cell::sync::Lazy<Arc<RwLock<CollGovState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(CollGovState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub id: String, pub topic: String, pub description: String,
    pub options: Vec<String>, pub votes: Vec<CastVote>,
    pub quorum_required: u32, pub status: VoteStatus,
    pub result: Option<String>, pub created_at: DateTime<Utc>,
    pub deadline: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastVote { pub peer_id: String, pub peer_name: String, pub choice: String, pub timestamp: DateTime<Utc>, pub weight: f32 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoteStatus { Open, Closed, QuorumNotMet, Decided }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElection {
    pub id: String, pub candidates: Vec<Candidate>,
    pub winner: Option<String>, pub status: VoteStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate { pub peer_id: String, pub peer_name: String, pub trust_score: f32, pub capabilities: Vec<String>, pub uptime_secs: u64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollGovState {
    pub votes: VecDeque<Vote>, pub elections: VecDeque<LeaderElection>,
    pub current_leader: Option<String>, pub total_votes: u64, pub total_elections: u64,
    pub consensus_method: ConsensusMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConsensusMethod { SimpleMajority, SuperMajority, Unanimous, WeightedByTrust }

impl Default for CollGovState {
    fn default() -> Self {
        Self { votes: VecDeque::with_capacity(50), elections: VecDeque::with_capacity(10), current_leader: None, total_votes: 0, total_elections: 0, consensus_method: ConsensusMethod::SimpleMajority }
    }
}

fn gen_id() -> String { format!("vote_{}", chrono::Utc::now().timestamp_millis()) }

/// Create a new vote
pub async fn create_vote(topic: &str, description: &str, options: Vec<String>, quorum: u32, deadline_hours: u32) -> Vote {
    let mut state = COLGOV.write().await;
    let vote = Vote { id: gen_id(), topic: topic.into(), description: description.into(), options, votes: vec![], quorum_required: quorum, status: VoteStatus::Open, result: None, created_at: Utc::now(), deadline: Utc::now() + chrono::Duration::hours(deadline_hours as i64) };
    state.votes.push_back(vote.clone());
    state.total_votes += 1;
    vote
}

/// Cast a vote
pub async fn cast_vote(vote_id: &str, peer_id: &str, peer_name: &str, choice: &str, weight: f32) -> bool {
    let mut state = COLGOV.write().await;
    if let Some(vote) = state.votes.iter_mut().find(|v| v.id == vote_id && v.status == VoteStatus::Open) {
        if vote.votes.iter().any(|v| v.peer_id == peer_id) { return false; } // Already voted
        vote.votes.push(CastVote { peer_id: peer_id.into(), peer_name: peer_name.into(), choice: choice.into(), timestamp: Utc::now(), weight });

        // Check if quorum met
        if vote.votes.len() as u32 >= vote.quorum_required {
            // Count votes
            let mut counts: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
            for v in &vote.votes { *counts.entry(v.choice.clone()).or_insert(0.0) += v.weight; }
            let winner = counts.into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            vote.result = winner.map(|(choice, _)| choice);
            vote.status = VoteStatus::Decided;
        }
        return true;
    }
    false
}

/// Trigger a leader election based on trust scores and capabilities
pub async fn elect_leader() -> LeaderElection {
    let peers = crate::collective::get_peers().await;
    let candidates: Vec<Candidate> = peers.iter().filter(|p| p.status == crate::collective::PeerStatus::Online).map(|p| {
        Candidate { peer_id: p.id.clone(), peer_name: p.name.clone(), trust_score: p.trust_score, capabilities: p.capabilities.clone(), uptime_secs: 0 }
    }).collect();

    let winner = candidates.iter().max_by(|a, b| a.trust_score.partial_cmp(&b.trust_score).unwrap_or(std::cmp::Ordering::Equal)).map(|c| c.peer_id.clone());

    let election = LeaderElection { id: gen_id(), candidates, winner: winner.clone(), status: VoteStatus::Decided, timestamp: Utc::now() };

    let mut state = COLGOV.write().await;
    state.current_leader = winner;
    if state.elections.len() >= 10 { state.elections.pop_front(); }
    state.elections.push_back(election.clone());
    state.total_elections += 1;

    election
}

pub async fn get_votes() -> Vec<Vote> { COLGOV.read().await.votes.iter().cloned().collect() }
pub async fn get_current_leader() -> Option<String> { COLGOV.read().await.current_leader.clone() }

pub async fn get_stats() -> serde_json::Value {
    let s = COLGOV.read().await;
    serde_json::json!({ "total_votes": s.total_votes, "total_elections": s.total_elections, "current_leader": s.current_leader, "consensus_method": format!("{:?}", s.consensus_method) })
}

pub async fn persist() { let s = COLGOV.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO collective_governance_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM collective_governance_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<CollGovState>(json) { let mut s = COLGOV.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS collective_governance_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = CollGovState::default(); assert_eq!(s.consensus_method, ConsensusMethod::SimpleMajority); } #[test] fn test_vote_status_eq() { assert_eq!(VoteStatus::Open, VoteStatus::Open); } }
