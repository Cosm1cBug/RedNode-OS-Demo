// RedNode-OS – GOAP (Goal-Oriented Action Planning)
//
// For complex multi-step goals with dependencies, GOAP provides:
//   - Precondition checking (don't do step B before step A completes)
//   - Cost-based ordering (cheapest/fastest steps first)
//   - Automatic replanning when a step fails
//
// Used by the planner when the LLM identifies a complex goal.
// Simple intents still use the direct LLM planner.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;

/// A GOAP action — a tool call with preconditions and effects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoapAction {
    pub tool: String,
    pub agent: String,
    pub cost: f32,
    pub preconditions: HashSet<String>,   // state flags that must be true
    pub effects: HashSet<String>,         // state flags this action makes true
    pub args: serde_json::Value,
}

/// A node in the A* search
#[derive(Debug, Clone)]
struct SearchNode {
    state: HashSet<String>,
    actions_taken: Vec<GoapAction>,
    total_cost: f32,
    heuristic: f32,
}

impl PartialEq for SearchNode {
    fn eq(&self, other: &Self) -> bool {
        self.total_cost == other.total_cost
    }
}
impl Eq for SearchNode {}

impl Ord for SearchNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap: lower cost = higher priority
        other.total_cost.partial_cmp(&self.total_cost)
            .unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Plan a sequence of actions to reach the goal state from the current state.
/// Uses A* search over the action space.
///
/// Returns None if no valid plan exists (missing preconditions, etc.)
pub fn plan_goap(
    current_state: &HashSet<String>,
    goal_state: &HashSet<String>,
    available_actions: &[GoapAction],
    max_depth: usize,
) -> Option<Vec<GoapAction>> {
    let mut heap = BinaryHeap::new();
    let mut visited: HashSet<Vec<String>> = HashSet::new();

    // Start node
    heap.push(SearchNode {
        state: current_state.clone(),
        actions_taken: Vec::new(),
        total_cost: 0.0,
        heuristic: heuristic(&current_state, goal_state),
    });

    while let Some(node) = heap.pop() {
        // Goal check: all goal flags satisfied
        if goal_state.iter().all(|g| node.state.contains(g)) {
            return Some(node.actions_taken);
        }

        // Depth limit
        if node.actions_taken.len() >= max_depth {
            continue;
        }

        // Dedup
        let mut state_key: Vec<String> = node.state.iter().cloned().collect();
        state_key.sort();
        if visited.contains(&state_key) {
            continue;
        }
        visited.insert(state_key);

        // Expand: try each action whose preconditions are met
        for action in available_actions {
            if action.preconditions.iter().all(|p| node.state.contains(p)) {
                let mut new_state = node.state.clone();
                for effect in &action.effects {
                    new_state.insert(effect.clone());
                }

                let mut new_actions = node.actions_taken.clone();
                new_actions.push(action.clone());

                let new_cost = node.total_cost + action.cost;
                let h = heuristic(&new_state, goal_state);

                heap.push(SearchNode {
                    state: new_state,
                    actions_taken: new_actions,
                    total_cost: new_cost + h,
                    heuristic: h,
                });
            }
        }
    }

    None // No plan found
}

/// Heuristic: count unsatisfied goal flags
fn heuristic(state: &HashSet<String>, goal: &HashSet<String>) -> f32 {
    goal.iter().filter(|g| !state.contains(*g)).count() as f32
}

/// Convert a natural language goal into GOAP actions using known tool capabilities.
/// Returns a set of GoapActions derived from the tool registry.
pub fn tools_to_goap_actions() -> Vec<GoapAction> {
    vec![
        // Network setup chain
        GoapAction {
            tool: "net.status".into(), agent: "network-agent".into(), cost: 1.0,
            preconditions: HashSet::new(),
            effects: ["network_checked".into()].into(),
            args: serde_json::json!({}),
        },
        GoapAction {
            tool: "pihole.stats".into(), agent: "infra-agent".into(), cost: 1.0,
            preconditions: ["network_checked".into()].into(),
            effects: ["dns_checked".into()].into(),
            args: serde_json::json!({}),
        },
        GoapAction {
            tool: "nas.health".into(), agent: "storage-agent".into(), cost: 1.0,
            preconditions: ["network_checked".into()].into(),
            effects: ["storage_checked".into()].into(),
            args: serde_json::json!({}),
        },
        GoapAction {
            tool: "cam.status".into(), agent: "surveillance-agent".into(), cost: 1.0,
            preconditions: ["network_checked".into()].into(),
            effects: ["cameras_checked".into()].into(),
            args: serde_json::json!({}),
        },
        // Security chain
        GoapAction {
            tool: "sec.triage".into(), agent: "security-agent".into(), cost: 2.0,
            preconditions: HashSet::new(),
            effects: ["logs_checked".into()].into(),
            args: serde_json::json!({}),
        },
        GoapAction {
            tool: "sec.cve_check".into(), agent: "security-agent".into(), cost: 3.0,
            preconditions: HashSet::new(),
            effects: ["cve_scanned".into()].into(),
            args: serde_json::json!({}),
        },
        GoapAction {
            tool: "sec.harden_ssh".into(), agent: "security-agent".into(), cost: 5.0,
            preconditions: ["logs_checked".into(), "cve_scanned".into()].into(),
            effects: ["ssh_hardened".into()].into(),
            args: serde_json::json!({}),
        },
        // System health chain
        GoapAction {
            tool: "process.list".into(), agent: "system-agent".into(), cost: 1.0,
            preconditions: HashSet::new(),
            effects: ["processes_checked".into()].into(),
            args: serde_json::json!({}),
        },
        GoapAction {
            tool: "docker.ps".into(), agent: "system-agent".into(), cost: 1.0,
            preconditions: HashSet::new(),
            effects: ["docker_checked".into()].into(),
            args: serde_json::json!({}),
        },
        // Snapshot after checks
        GoapAction {
            tool: "nas.snapshot_create".into(), agent: "storage-agent".into(), cost: 3.0,
            preconditions: ["storage_checked".into()].into(),
            effects: ["snapshot_created".into()].into(),
            args: serde_json::json!({"dataset": "tank/documents"}),
        },
        // Full security audit
        GoapAction {
            tool: "sec.threat_intel".into(), agent: "security-agent".into(), cost: 4.0,
            preconditions: ["cve_scanned".into()].into(),
            effects: ["threat_intel_synced".into()].into(),
            args: serde_json::json!({}),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_plan() {
        let current = HashSet::new();
        let goal: HashSet<String> = ["network_checked".into(), "dns_checked".into()].into();
        let actions = tools_to_goap_actions();

        let plan = plan_goap(&current, &goal, &actions, 10);
        assert!(plan.is_some());
        let steps = plan.unwrap();
        assert!(steps.len() >= 2);
        assert_eq!(steps[0].tool, "net.status"); // must come first (precondition for pihole)
    }

    #[test]
    fn test_dependency_ordering() {
        let current = HashSet::new();
        let goal: HashSet<String> = ["ssh_hardened".into()].into();
        let actions = tools_to_goap_actions();

        let plan = plan_goap(&current, &goal, &actions, 10);
        assert!(plan.is_some());
        let steps = plan.unwrap();
        // ssh_harden requires logs_checked + cve_scanned → triage and cve_check must come first
        let ssh_idx = steps.iter().position(|s| s.tool == "sec.harden_ssh").unwrap();
        let triage_idx = steps.iter().position(|s| s.tool == "sec.triage").unwrap();
        let cve_idx = steps.iter().position(|s| s.tool == "sec.cve_check").unwrap();
        assert!(triage_idx < ssh_idx);
        assert!(cve_idx < ssh_idx);
    }

    #[test]
    fn test_no_plan_possible() {
        let current = HashSet::new();
        let goal: HashSet<String> = ["impossible_flag".into()].into();
        let actions = tools_to_goap_actions();

        let plan = plan_goap(&current, &goal, &actions, 10);
        assert!(plan.is_none());
    }
}
